//! Saying how far along a scan is.

/// Where a scan has got to.
///
/// The stages are in the order they happen and a scan passes through each of
/// them once, so a front end can show them as steps rather than guessing from
/// the counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanStage {
    /// Walking the folder. Nothing has been read yet.
    Discovering,
    /// Working out which subtitle belongs to which film.
    Pairing,
    /// Reading subtitle files and writing what they say.
    Indexing,
    Finished,
}

/// The counts a scan reports as it goes.
///
/// Everything here only ever grows, apart from the stage, so a report that
/// arrives out of order is at worst old rather than wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanProgress {
    pub folder_id: i64,
    pub stage: ScanStage,
    /// Every file the walk looked at, whether or not it was of use.
    pub files_seen: usize,
    pub films_found: usize,
    pub subtitles_found: usize,
    /// Films with at least one subtitle file to their name.
    pub films_paired: usize,
    /// Subtitle files that need reading, which is the total the indexing stage
    /// is working towards.
    pub subtitles_to_read: usize,
    pub subtitles_read: usize,
    pub cues_indexed: usize,
}

impl ScanProgress {
    #[must_use]
    pub fn new(folder_id: i64) -> Self {
        Self {
            folder_id,
            stage: ScanStage::Discovering,
            files_seen: 0,
            films_found: 0,
            subtitles_found: 0,
            films_paired: 0,
            subtitles_to_read: 0,
            subtitles_read: 0,
            cues_indexed: 0,
        }
    }

    /// How much of the indexing is done, from zero to one.
    ///
    /// A scan with nothing to read is finished rather than nowhere, which is
    /// what a rescan of an unchanged library is.
    #[must_use]
    // Both sides are counts of files, far below the point where an f32 loses a
    // whole number, and the result is a fraction.
    #[allow(clippy::cast_precision_loss)]
    pub fn fraction_read(&self) -> f32 {
        if self.subtitles_to_read == 0 {
            return 1.0;
        }
        (self.subtitles_read as f32 / self.subtitles_to_read as f32).clamp(0.0, 1.0)
    }
}

/// Where progress reports go.
///
/// Reports are sent from whichever thread is doing the writing, so this has to
/// be usable from several at once. The application turns them into events for
/// the front end; the tests count them.
pub trait ProgressSink: Send + Sync {
    fn report(&self, progress: &ScanProgress);
}

impl<F: Fn(&ScanProgress) + Send + Sync> ProgressSink for F {
    fn report(&self, progress: &ScanProgress) {
        self(progress);
    }
}

/// A sink for callers with nothing to show.
#[derive(Clone, Copy, Debug, Default)]
pub struct Silent;

impl ProgressSink for Silent {
    fn report(&self, _progress: &ScanProgress) {}
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{ProgressSink, ScanProgress, ScanStage, Silent};

    #[test]
    fn nothing_to_read_is_finished_rather_than_nowhere() {
        let mut progress = ScanProgress::new(1);
        assert!((progress.fraction_read() - 1.0).abs() < f32::EPSILON);

        progress.subtitles_to_read = 4;
        assert!((progress.fraction_read() - 0.0).abs() < f32::EPSILON);
        progress.subtitles_read = 1;
        assert!((progress.fraction_read() - 0.25).abs() < f32::EPSILON);
        progress.subtitles_read = 9;
        assert!((progress.fraction_read() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_closure_is_a_sink() {
        let seen = Mutex::new(Vec::new());
        let sink = |progress: &ScanProgress| {
            if let Ok(mut seen) = seen.lock() {
                seen.push(progress.stage);
            }
        };

        sink.report(&ScanProgress::new(1));
        Silent.report(&ScanProgress::new(1));

        assert_eq!(
            seen.into_inner().unwrap_or_default(),
            [ScanStage::Discovering]
        );
    }
}
