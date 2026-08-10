//! What the parser had to work around in a file.

use core::fmt;

/// Something the parser recovered from, with how often it happened.
///
/// Occurrences are grouped by kind rather than listed one by one. A file that
/// is broken at all is usually broken the same way throughout, and the import
/// sheet wants to say "forty malformed timings, the first on line 12" rather
/// than print forty lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseWarning {
    pub kind: ParseWarningKind,
    /// The line it was first seen on, counted from one. Zero when it concerns
    /// the file as a whole rather than one place in it.
    pub first_line: usize,
    pub count: usize,
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if self.count > 1 {
            write!(f, " ({} times)", self.count)?;
        }
        if self.first_line > 0 {
            write!(f, ", first on line {}", self.first_line)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParseWarningKind {
    /// Bytes that no encoding could account for, replaced on the way in.
    UndecodableBytes,
    /// A cue that began with its timing, with no number in front of it.
    MissingIndex,
    /// A cue numbered with something other than a number.
    MalformedIndex,
    /// A timing line that could not be read, taking its cue with it.
    MalformedTiming,
    /// A line belonging to no cue.
    UnexpectedLine,
    /// The file stopped part way through a cue.
    TruncatedCue,
    /// A cue whose text was blank once markup was removed.
    EmptyCue,
    /// A cue that ended before it started, held at its start instead.
    NegativeDuration,
    /// Cues that were not in playback order, since put in order.
    OutOfOrder,
    /// Cues that are on screen at the same time.
    OverlappingCues,
}

impl fmt::Display for ParseWarningKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::UndecodableBytes => "some bytes could not be decoded",
            Self::MissingIndex => "a cue had no number",
            Self::MalformedIndex => "a cue was numbered with something other than a number",
            Self::MalformedTiming => "a timing could not be read",
            Self::UnexpectedLine => "a line belonged to no cue",
            Self::TruncatedCue => "the file stopped part way through a cue",
            Self::EmptyCue => "a cue had no text",
            Self::NegativeDuration => "a cue ended before it started",
            Self::OutOfOrder => "the cues were not in order",
            Self::OverlappingCues => "some cues are on screen at once",
        };
        f.write_str(description)
    }
}

/// Gathers warnings, folding repeats of the same kind together.
#[derive(Debug, Default)]
pub(crate) struct WarningSink {
    warnings: Vec<ParseWarning>,
}

impl WarningSink {
    pub(crate) fn push(&mut self, line: usize, kind: ParseWarningKind) {
        if let Some(existing) = self
            .warnings
            .iter_mut()
            .find(|warning| warning.kind == kind)
        {
            existing.count += 1;
            return;
        }
        self.warnings.push(ParseWarning {
            kind,
            first_line: line,
            count: 1,
        });
    }

    pub(crate) fn into_warnings(self) -> Vec<ParseWarning> {
        self.warnings
    }
}

#[cfg(test)]
mod tests {
    use super::{ParseWarning, ParseWarningKind, WarningSink};

    #[test]
    fn repeats_fold_into_one_entry() {
        let mut sink = WarningSink::default();
        sink.push(4, ParseWarningKind::MalformedTiming);
        sink.push(9, ParseWarningKind::MalformedTiming);
        sink.push(11, ParseWarningKind::EmptyCue);

        let warnings = sink.into_warnings();
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].first_line, 4);
        assert_eq!(warnings[0].count, 2);
    }

    #[test]
    fn reads_as_a_sentence() {
        let warning = ParseWarning {
            kind: ParseWarningKind::EmptyCue,
            first_line: 12,
            count: 3,
        };
        assert_eq!(
            warning.to_string(),
            "a cue had no text (3 times), first on line 12"
        );

        let whole_file = ParseWarning {
            kind: ParseWarningKind::OutOfOrder,
            first_line: 0,
            count: 1,
        };
        assert_eq!(whole_file.to_string(), "the cues were not in order");
    }
}
