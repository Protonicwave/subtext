//! The SRT parser.
//!
//! There is no specification for the format, only a convention, and files in
//! the wild depart from it constantly. The parser is therefore written to
//! recover rather than to reject: a block it cannot read is recorded as a
//! warning and skipped, and the rest of the file still arrives. It has no
//! failure case and no panic path, because a single bad file must not stop a
//! library from being indexed.

use super::tags;
use super::warning::{ParseWarningKind, WarningSink};
use crate::{Cue, ParseOutcome, SubtitleTrack, Timestamp};

/// The marker that separates the two halves of a timing line.
const ARROW: &str = "-->";

struct Line<'a> {
    /// Counted from one, for warnings that a person has to act on.
    number: usize,
    text: &'a str,
}

pub(crate) fn parse(text: &str, encoding: &'static str, replaced: bool) -> ParseOutcome {
    let mut warnings = WarningSink::default();
    if replaced {
        warnings.push(0, ParseWarningKind::UndecodableBytes);
    }

    let lines = split_lines(text);
    let mut cues = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        at = read_block(&lines, at, &mut cues, &mut warnings);
    }

    order(&mut cues, &mut warnings);
    ParseOutcome {
        track: SubtitleTrack::new(cues, encoding),
        warnings: warnings.into_warnings(),
    }
}

/// Splits on all three line endings, since a file may have been through tools
/// on more than one platform and may carry a mixture.
fn split_lines(text: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut number = 1;
    let mut rest = text;

    while let Some(at) = rest.find(['\n', '\r']) {
        lines.push(Line {
            number,
            text: &rest[..at],
        });
        let skip = if rest[at..].starts_with("\r\n") { 2 } else { 1 };
        rest = &rest[at + skip..];
        number += 1;
    }
    if !rest.is_empty() {
        lines.push(Line { number, text: rest });
    }

    lines
}

/// Reads one cue starting at `at` and returns where the next one begins.
///
/// Always advances, so a file of nothing but rubbish terminates.
fn read_block(
    lines: &[Line<'_>],
    at: usize,
    cues: &mut Vec<Cue>,
    warnings: &mut WarningSink,
) -> usize {
    if lines[at].text.trim().is_empty() {
        return at + 1;
    }

    let Some(timing) = locate_timing(lines, at, warnings) else {
        return at + 1;
    };

    let (text, next) = read_text(lines, timing + 1);

    let Some((start, end)) = read_timing(lines[timing].text) else {
        warnings.push(lines[timing].number, ParseWarningKind::MalformedTiming);
        return next;
    };

    let cleaned = tags::clean(&text);
    if cleaned.text.is_empty() {
        // A cue with no words is either a stray block or a deliberate gap. We
        // draw subtitles ourselves and honour the end time, so neither needs
        // to survive into the transcript.
        warnings.push(lines[timing].number, ParseWarningKind::EmptyCue);
        return next;
    }

    let end = if end < start {
        // The dialogue is still worth reading and searching, so the cue is kept
        // with the timing collapsed rather than thrown away.
        warnings.push(lines[timing].number, ParseWarningKind::NegativeDuration);
        start
    } else {
        end
    };

    cues.push(Cue {
        index: 0,
        start,
        end,
        text: cleaned.text,
        position: cleaned.position,
    });
    next
}

/// Finds the timing line of the block beginning at `at`.
///
/// A block conventionally opens with an index and then the timing, but the
/// index is frequently missing or is something other than a number, and both
/// are recoverable as long as a timing line is where it should be.
fn locate_timing(lines: &[Line<'_>], at: usize, warnings: &mut WarningSink) -> Option<usize> {
    if is_timing(lines[at].text) {
        warnings.push(lines[at].number, ParseWarningKind::MissingIndex);
        return Some(at);
    }
    if lines.get(at + 1).is_some_and(|line| is_timing(line.text)) {
        if !is_index(lines[at].text) {
            warnings.push(lines[at].number, ParseWarningKind::MalformedIndex);
        }
        return Some(at + 1);
    }

    let kind = if is_index(lines[at].text) && at + 1 == lines.len() {
        // The file stops after an index, which is what a download interrupted
        // part way through looks like.
        ParseWarningKind::TruncatedCue
    } else {
        ParseWarningKind::UnexpectedLine
    };
    warnings.push(lines[at].number, kind);
    None
}

/// Collects the text of a cue and returns where the next block begins.
fn read_text<'a>(lines: &[Line<'a>], from: usize) -> (Vec<&'a str>, usize) {
    let mut text = Vec::new();
    let mut at = from;

    while at < lines.len() {
        let line = lines[at].text;
        if line.trim().is_empty() {
            at += 1;
            break;
        }
        // Some files omit the blank line between cues, so the start of the next
        // block also ends this one.
        if is_timing(line) {
            break;
        }
        if is_index(line) && lines.get(at + 1).is_some_and(|next| is_timing(next.text)) {
            break;
        }
        text.push(line);
        at += 1;
    }

    (text, at)
}

fn is_timing(line: &str) -> bool {
    line.contains(ARROW)
}

fn is_index(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit())
}

/// Reads the two ends of a timing line.
///
/// The last token before the arrow is taken rather than the whole left side,
/// because some files put the index on the same line as the timing. Anything
/// after the end time, such as the rarely used display coordinates, is ignored.
fn read_timing(line: &str) -> Option<(Timestamp, Timestamp)> {
    let (left, right) = line.split_once(ARROW)?;
    let start = Timestamp::parse(left.split_whitespace().next_back()?)?;
    let end = Timestamp::parse(right.split_whitespace().next()?)?;
    Some((start, end))
}

/// Puts the cues in playback order and numbers them.
fn order(cues: &mut [Cue], warnings: &mut WarningSink) {
    if cues.windows(2).any(|pair| pair[1].start < pair[0].start) {
        warnings.push(0, ParseWarningKind::OutOfOrder);
        cues.sort_by_key(|cue| (cue.start, cue.end));
    }
    if cues.windows(2).any(|pair| pair[1].start < pair[0].end) {
        // Overlaps are legitimate, and the track handles them, but a file full
        // of them usually means the timings were shifted by hand.
        warnings.push(0, ParseWarningKind::OverlappingCues);
    }
    for (position, cue) in cues.iter_mut().enumerate() {
        cue.index = u32::try_from(position + 1).unwrap_or(u32::MAX);
    }
}
