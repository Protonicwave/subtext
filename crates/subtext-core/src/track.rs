//! A parsed subtitle file.

use crate::{Cue, Timestamp};

/// The cues of one subtitle file, in playback order.
///
/// Lookups are the reason this is a type rather than a bare vector: the player
/// asks which cue is on screen sixty times a second, and the transcript asks
/// for the neighbours of the current position every time someone presses an
/// arrow key. Both are binary searches over the sorted cues.
#[derive(Clone, Debug)]
pub struct SubtitleTrack {
    cues: Vec<Cue>,
    /// The latest end time among the cues up to and including each index.
    ///
    /// Cues may overlap, so the cue on screen at a given moment is not
    /// necessarily the last one to have started. This lets the backwards walk
    /// stop as soon as no earlier cue can still be running, which keeps lookup
    /// exact without an interval tree.
    latest_end: Vec<Timestamp>,
    encoding: &'static str,
}

impl SubtitleTrack {
    /// Builds a track from cues that are already in playback order.
    ///
    /// `encoding` is the name of the character encoding the file was read as,
    /// which the import sheet shows when a file needed guessing.
    #[must_use]
    pub fn new(cues: Vec<Cue>, encoding: &'static str) -> Self {
        let mut latest_end = Vec::with_capacity(cues.len());
        let mut latest = Timestamp::ZERO;
        for cue in &cues {
            latest = latest.max(cue.end);
            latest_end.push(latest);
        }
        Self {
            cues,
            latest_end,
            encoding,
        }
    }

    #[must_use]
    pub fn cues(&self) -> &[Cue] {
        &self.cues
    }

    #[must_use]
    pub fn into_cues(self) -> Vec<Cue> {
        self.cues
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cues.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }

    #[must_use]
    pub fn encoding(&self) -> &'static str {
        self.encoding
    }

    /// The last cue to have started that is still on screen at `at`.
    #[must_use]
    pub fn cue_at(&self, at: Timestamp) -> Option<&Cue> {
        let mut candidate = self.cues.partition_point(|cue| cue.start <= at);
        while candidate > 0 {
            candidate -= 1;
            if self.latest_end[candidate] <= at {
                return None;
            }
            if self.cues[candidate].contains(at) {
                return Some(&self.cues[candidate]);
            }
        }
        None
    }

    /// The first cue starting after `at`, for the next line control.
    #[must_use]
    pub fn cue_after(&self, at: Timestamp) -> Option<&Cue> {
        self.cues
            .get(self.cues.partition_point(|cue| cue.start <= at))
    }

    /// The last cue starting before `at`, for the previous line control.
    #[must_use]
    pub fn cue_before(&self, at: Timestamp) -> Option<&Cue> {
        let after = self.cues.partition_point(|cue| cue.start < at);
        (after > 0).then(|| &self.cues[after - 1])
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::SubtitleTrack;
    use crate::{Cue, Timestamp};

    fn track(spans: &[(u32, u32)]) -> SubtitleTrack {
        let cues = spans
            .iter()
            .enumerate()
            .map(|(position, &(start, end))| Cue {
                index: u32::try_from(position).unwrap() + 1,
                start: Timestamp::from_millis(start),
                end: Timestamp::from_millis(end),
                text: format!("line {position}"),
                position: None,
            })
            .collect();
        SubtitleTrack::new(cues, "UTF-8")
    }

    fn at(millis: u32) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    #[test]
    fn finds_the_cue_on_screen() {
        let track = track(&[(0, 1_000), (2_000, 3_000), (4_000, 5_000)]);
        assert_eq!(track.cue_at(at(0)).unwrap().index, 1);
        assert_eq!(track.cue_at(at(2_500)).unwrap().index, 2);
        assert_eq!(track.cue_at(at(4_999)).unwrap().index, 3);
    }

    #[test]
    fn reports_nothing_in_the_gaps() {
        let track = track(&[(0, 1_000), (2_000, 3_000)]);
        assert!(track.cue_at(at(1_500)).is_none());
        assert!(track.cue_at(at(3_000)).is_none());
    }

    #[test]
    fn prefers_the_latest_of_two_overlapping_cues() {
        let track = track(&[(0, 10_000), (1_000, 2_000)]);
        assert_eq!(track.cue_at(at(1_500)).unwrap().index, 2);
        // Only the long cue is still running here, so the walk must look past
        // the one that has already ended.
        assert_eq!(track.cue_at(at(5_000)).unwrap().index, 1);
    }

    #[test]
    fn walks_to_the_neighbours() {
        let track = track(&[(0, 1_000), (2_000, 3_000), (4_000, 5_000)]);
        assert_eq!(track.cue_after(at(0)).unwrap().index, 2);
        assert_eq!(track.cue_after(at(2_500)).unwrap().index, 3);
        assert!(track.cue_after(at(4_000)).is_none());

        assert!(track.cue_before(at(0)).is_none());
        assert_eq!(track.cue_before(at(2_500)).unwrap().index, 2);
        assert_eq!(track.cue_before(at(4_000)).unwrap().index, 2);
    }

    #[test]
    fn an_empty_track_answers_nothing() {
        let track = track(&[]);
        assert!(track.is_empty());
        assert!(track.cue_at(at(0)).is_none());
        assert!(track.cue_after(at(0)).is_none());
        assert!(track.cue_before(at(0)).is_none());
    }
}
