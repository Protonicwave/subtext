//! Reading subtitle files.

mod decode;
mod srt;
pub mod tags;
mod warning;

pub use self::warning::{ParseWarning, ParseWarningKind};

use crate::SubtitleTrack;

/// A parsed subtitle file and everything the parser had to work around.
///
/// There is no error case. A file that cannot be read at all yields an empty
/// track and a list of warnings, which is what the import sheet needs in order
/// to tell someone which of their files is broken and how.
#[derive(Clone, Debug)]
pub struct ParseOutcome {
    pub track: SubtitleTrack,
    pub warnings: Vec<ParseWarning>,
}

impl ParseOutcome {
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Reads the bytes of an SRT file.
///
/// The encoding is worked out from the bytes, since subtitle files do not
/// declare one.
#[must_use]
pub fn parse_srt(bytes: &[u8]) -> ParseOutcome {
    let decoded = decode::decode(bytes);
    srt::parse(&decoded.text, decoded.encoding, decoded.replaced)
}

/// Reads SRT text that has already been decoded.
#[must_use]
pub fn parse_srt_text(text: &str) -> ParseOutcome {
    srt::parse(text, "UTF-8", false)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{ParseWarningKind, parse_srt, parse_srt_text};
    use crate::{CuePosition, Timestamp};

    #[test]
    fn reads_a_well_formed_file() {
        let outcome = parse_srt_text(
            "1\n00:00:01,000 --> 00:00:04,000\nHello there.\n\n\
             2\n00:00:05,000 --> 00:00:06,500\nSecond line,\nover two rows.\n",
        );

        assert!(!outcome.has_warnings());
        let cues = outcome.track.cues();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].index, 1);
        assert_eq!(cues[0].start, Timestamp::from_millis(1_000));
        assert_eq!(cues[0].end, Timestamp::from_millis(4_000));
        assert_eq!(cues[0].text, "Hello there.");
        assert_eq!(cues[1].text, "Second line,\nover two rows.");
    }

    #[test]
    fn accepts_windows_line_endings() {
        let outcome = parse_srt_text("1\r\n00:00:01,000 --> 00:00:02,000\r\nHello.\r\n\r\n");
        assert_eq!(outcome.track.cues()[0].text, "Hello.");
        assert!(!outcome.has_warnings());
    }

    #[test]
    fn accepts_a_missing_trailing_newline() {
        let outcome = parse_srt_text("1\n00:00:01,000 --> 00:00:02,000\nHello.");
        assert_eq!(outcome.track.len(), 1);
    }

    #[test]
    fn recovers_from_a_missing_index() {
        let outcome = parse_srt_text("00:00:01,000 --> 00:00:02,000\nHello.\n");
        assert_eq!(outcome.track.len(), 1);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::MissingIndex);
    }

    #[test]
    fn recovers_from_an_index_that_is_not_a_number() {
        let outcome = parse_srt_text("one\n00:00:01,000 --> 00:00:02,000\nHello.\n");
        assert_eq!(outcome.track.len(), 1);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::MalformedIndex);
    }

    #[test]
    fn skips_a_cue_with_an_unreadable_timing_and_keeps_going() {
        let outcome = parse_srt_text(
            "1\n00:00:01,000 --> broken\nSkipped.\n\n\
             2\n00:00:05,000 --> 00:00:06,000\nKept.\n",
        );

        assert_eq!(outcome.track.len(), 1);
        assert_eq!(outcome.track.cues()[0].text, "Kept.");
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::MalformedTiming);
    }

    #[test]
    fn puts_cues_back_in_order() {
        let outcome = parse_srt_text(
            "1\n00:00:09,000 --> 00:00:10,000\nSecond.\n\n\
             2\n00:00:01,000 --> 00:00:02,000\nFirst.\n",
        );

        let cues = outcome.track.cues();
        assert_eq!(cues[0].text, "First.");
        assert_eq!(cues[0].index, 1);
        assert_eq!(cues[1].text, "Second.");
        assert_eq!(cues[1].index, 2);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::OutOfOrder);
    }

    #[test]
    fn keeps_overlapping_cues_and_says_so() {
        let outcome = parse_srt_text(
            "1\n00:00:01,000 --> 00:00:09,000\nLong.\n\n\
             2\n00:00:02,000 --> 00:00:03,000\nShort.\n",
        );

        assert_eq!(outcome.track.len(), 2);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::OverlappingCues);
    }

    #[test]
    fn holds_a_backwards_cue_at_its_start() {
        let outcome = parse_srt_text("1\n00:00:05,000 --> 00:00:01,000\nBackwards.\n");
        let cue = &outcome.track.cues()[0];
        assert_eq!(cue.start, cue.end);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::NegativeDuration);
    }

    #[test]
    fn drops_cues_that_have_no_text() {
        let outcome = parse_srt_text(
            "1\n00:00:01,000 --> 00:00:02,000\n\n\
             2\n00:00:03,000 --> 00:00:04,000\nKept.\n",
        );

        assert_eq!(outcome.track.len(), 1);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::EmptyCue);
    }

    #[test]
    fn reads_cues_that_are_not_separated_by_a_blank_line() {
        let outcome = parse_srt_text(
            "1\n00:00:01,000 --> 00:00:02,000\nFirst.\n\
             2\n00:00:03,000 --> 00:00:04,000\nSecond.\n",
        );

        assert_eq!(outcome.track.len(), 2);
        assert_eq!(outcome.track.cues()[0].text, "First.");
        assert_eq!(outcome.track.cues()[1].text, "Second.");
    }

    #[test]
    fn keeps_what_a_truncated_file_managed_to_write() {
        let outcome = parse_srt_text(
            "1\n00:00:01,000 --> 00:00:02,000\nComplete.\n\n\
             2\n00:00:03,000 --> 00:00:04,000\nCut off mid",
        );

        assert_eq!(outcome.track.len(), 2);
        assert_eq!(outcome.track.cues()[1].text, "Cut off mid");
    }

    #[test]
    fn reports_a_file_that_stops_after_an_index() {
        let outcome = parse_srt_text("1\n00:00:01,000 --> 00:00:02,000\nComplete.\n\n2");
        assert_eq!(outcome.track.len(), 1);
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::TruncatedCue);
    }

    #[test]
    fn strips_markup_and_keeps_the_position() {
        let outcome =
            parse_srt_text("1\n00:00:01,000 --> 00:00:02,000\n{\\an8}<i>Above the action</i>\n");

        let cue = &outcome.track.cues()[0];
        assert_eq!(cue.text, "Above the action");
        assert_eq!(cue.position, Some(CuePosition::TopCentre));
    }

    #[test]
    fn takes_the_encoding_from_the_bytes() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("1\n00:00:01,000 --> 00:00:02,000\nCafé.\n".as_bytes());

        let outcome = parse_srt(&bytes);
        assert_eq!(outcome.track.cues()[0].text, "Café.");
        assert_eq!(outcome.track.encoding(), "UTF-8");
    }

    #[test]
    fn an_empty_file_is_an_empty_track() {
        let outcome = parse_srt(b"");
        assert!(outcome.track.is_empty());
        assert!(!outcome.has_warnings());
    }

    #[test]
    fn rubbish_produces_warnings_rather_than_a_panic() {
        let outcome = parse_srt_text("this is not a subtitle file\nat all\n");
        assert!(outcome.track.is_empty());
        assert_eq!(outcome.warnings[0].kind, ParseWarningKind::UnexpectedLine);
        assert_eq!(outcome.warnings[0].count, 2);
    }
}
