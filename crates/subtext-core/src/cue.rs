//! A single line of dialogue and where it sits on screen.

use crate::Timestamp;

/// One subtitle entry: a span of time and the text shown during it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    /// Position in the track, counted from one in playback order.
    ///
    /// This is assigned by the parser rather than taken from the file, because
    /// the numbering in a subtitle file is frequently absent, repeated or out
    /// of order, and the rest of the application needs a stable handle.
    pub index: u32,
    pub start: Timestamp,
    pub end: Timestamp,
    /// The dialogue as plain text. Multi-line cues keep their line breaks.
    pub text: String,
    /// Where the file asked for the cue to be drawn, if it asked at all.
    pub position: Option<CuePosition>,
}

impl Cue {
    /// Whether the cue is on screen at `at`.
    ///
    /// The end is exclusive, so a cue of zero length is never on screen. Files
    /// do contain such cues, and they are kept because their text is still
    /// worth searching, but they are not drawn.
    #[must_use]
    pub fn contains(&self, at: Timestamp) -> bool {
        self.start <= at && at < self.end
    }

    #[must_use]
    pub fn duration(&self) -> Timestamp {
        self.end.saturating_sub(self.start)
    }
}

/// Where a cue should be drawn, as one of the nine positions the substation
/// alpha alignment tags describe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CuePosition {
    TopLeft,
    TopCentre,
    TopRight,
    MiddleLeft,
    MiddleCentre,
    MiddleRight,
    BottomLeft,
    BottomCentre,
    BottomRight,
}

impl CuePosition {
    /// Reads an `\anN` alignment, numbered from the bottom left round to the
    /// top right in reading order.
    #[must_use]
    pub const fn from_alignment(alignment: u8) -> Option<Self> {
        match alignment {
            1 => Some(Self::BottomLeft),
            2 => Some(Self::BottomCentre),
            3 => Some(Self::BottomRight),
            4 => Some(Self::MiddleLeft),
            5 => Some(Self::MiddleCentre),
            6 => Some(Self::MiddleRight),
            7 => Some(Self::TopLeft),
            8 => Some(Self::TopCentre),
            9 => Some(Self::TopRight),
            _ => None,
        }
    }

    /// Reads the older `\aN` alignment, where the low two bits give the
    /// horizontal position and a separate bit raises the cue.
    #[must_use]
    pub const fn from_legacy_alignment(alignment: u8) -> Option<Self> {
        match alignment {
            1 => Some(Self::BottomLeft),
            2 => Some(Self::BottomCentre),
            3 => Some(Self::BottomRight),
            5 => Some(Self::TopLeft),
            6 => Some(Self::TopCentre),
            7 => Some(Self::TopRight),
            9 => Some(Self::MiddleLeft),
            10 => Some(Self::MiddleCentre),
            11 => Some(Self::MiddleRight),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cue, CuePosition};
    use crate::Timestamp;

    fn cue(start: u32, end: u32) -> Cue {
        Cue {
            index: 1,
            start: Timestamp::from_millis(start),
            end: Timestamp::from_millis(end),
            text: "text".to_owned(),
            position: None,
        }
    }

    #[test]
    fn a_cue_covers_its_start_but_not_its_end() {
        let cue = cue(1_000, 2_000);
        assert!(!cue.contains(Timestamp::from_millis(999)));
        assert!(cue.contains(Timestamp::from_millis(1_000)));
        assert!(cue.contains(Timestamp::from_millis(1_999)));
        assert!(!cue.contains(Timestamp::from_millis(2_000)));
    }

    #[test]
    fn a_cue_of_no_length_is_never_on_screen() {
        let cue = cue(1_000, 1_000);
        assert!(!cue.contains(Timestamp::from_millis(1_000)));
        assert_eq!(cue.duration(), Timestamp::ZERO);
    }

    #[test]
    fn alignments_map_to_the_reading_order() {
        assert_eq!(CuePosition::from_alignment(8), Some(CuePosition::TopCentre));
        assert_eq!(
            CuePosition::from_alignment(2),
            Some(CuePosition::BottomCentre)
        );
        assert_eq!(CuePosition::from_alignment(0), None);
        assert_eq!(
            CuePosition::from_legacy_alignment(6),
            Some(CuePosition::TopCentre)
        );
        assert_eq!(
            CuePosition::from_legacy_alignment(10),
            Some(CuePosition::MiddleCentre)
        );
        assert_eq!(CuePosition::from_legacy_alignment(4), None);
    }
}
