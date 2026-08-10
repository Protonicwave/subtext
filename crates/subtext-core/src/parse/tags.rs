//! Reducing the markup found in subtitle files to plain text.
//!
//! Two kinds of markup turn up in SRT files: a small dialect of HTML for
//! italics, bold and colour, and substation alpha override blocks in braces.
//! Subtext draws subtitles itself and takes its typography from the appearance
//! settings, so all of it is discarded. The one thing worth keeping is where
//! the file asked for the cue to be drawn, since a caption placed at the top
//! is usually avoiding something on screen.

use crate::CuePosition;

pub(crate) struct Cleaned {
    pub text: String,
    pub position: Option<CuePosition>,
}

/// Joins the raw lines of one cue into plain text.
pub(crate) fn clean(lines: &[&str]) -> Cleaned {
    let mut text = String::new();
    let mut position = None;
    let mut buffer = String::new();

    for line in lines {
        buffer.clear();
        buffer.reserve(line.len());
        strip(line, &mut buffer, &mut position);

        for piece in buffer.split('\n') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(piece);
        }
    }

    Cleaned { text, position }
}

fn strip(line: &str, out: &mut String, position: &mut Option<CuePosition>) {
    let mut at = 0;
    while at < line.len() {
        let rest = &line[at..];
        let Some(character) = rest.chars().next() else {
            break;
        };
        let width = character.len_utf8();

        match character {
            '{' => {
                if let Some(close) = rest.find('}') {
                    let inner = &rest[1..close];
                    if is_override(inner) {
                        read_position(inner, position);
                        at += close + 1;
                        continue;
                    }
                }
                out.push('{');
                at += width;
            }
            '<' => {
                if opens_a_tag(rest)
                    && let Some(close) = rest.find('>')
                {
                    at += close + 1;
                    continue;
                }
                out.push('<');
                at += width;
            }
            '&' => {
                if let Some((decoded, length)) = entity(rest) {
                    out.push(decoded);
                    at += length;
                    continue;
                }
                out.push('&');
                at += width;
            }
            '\\' => {
                // Substation alpha escapes. A hard line break and a hard space
                // are the only two that survive into plain text.
                match rest.as_bytes().get(1) {
                    Some(b'N' | b'n') => {
                        out.push('\n');
                        at += 2;
                    }
                    Some(b'h') => {
                        out.push(' ');
                        at += 2;
                    }
                    _ => {
                        out.push('\\');
                        at += width;
                    }
                }
            }
            // A byte order mark repeated inside the file, and the zero width
            // space some tools use as padding.
            '\u{feff}' | '\u{200b}' => at += width,
            _ => {
                out.push(character);
                at += width;
            }
        }
    }
}

/// Whether a braced run is markup rather than dialogue.
///
/// Braces do appear in dialogue, so the content has to look like an override:
/// either a backslash command such as `{\an8}`, or the older `{y:i}` form.
fn is_override(inner: &str) -> bool {
    if inner.starts_with('\\') {
        return true;
    }
    let mut characters = inner.chars();
    match (characters.next(), characters.next()) {
        (Some(first), Some(':')) if first.is_ascii_alphabetic() => true,
        _ => inner.starts_with("an") && inner[2..].bytes().all(|byte| byte.is_ascii_digit()),
    }
}

/// Whether a less-than sign starts a tag rather than being spoken punctuation.
fn opens_a_tag(rest: &str) -> bool {
    let mut characters = rest.chars().skip(1);
    match characters.next() {
        Some('/') => characters
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic()),
        Some(next) => next.is_ascii_alphabetic(),
        None => false,
    }
}

fn read_position(inner: &str, position: &mut Option<CuePosition>) {
    if position.is_some() {
        return;
    }

    let bytes = inner.as_bytes();
    for at in 0..bytes.len() {
        if bytes[at] != b'a' || (at > 0 && bytes[at - 1] != b'\\') {
            continue;
        }
        // `\an` numbers the nine positions in reading order; the older `\a`
        // uses a different set of numbers for the same nine places.
        let modern = bytes.get(at + 1) == Some(&b'n');
        let start = if modern { at + 2 } else { at + 1 };
        let Some(digits) = bytes.get(start..) else {
            continue;
        };
        let end = start
            + digits
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
        if end == start {
            continue;
        }
        let Ok(alignment) = inner[start..end].parse::<u8>() else {
            continue;
        };
        *position = if modern {
            CuePosition::from_alignment(alignment)
        } else {
            CuePosition::from_legacy_alignment(alignment)
        };
        if position.is_some() {
            return;
        }
    }
}

const NAMED_ENTITIES: &[(&str, char)] = &[
    ("amp", '&'),
    ("apos", '\''),
    ("gt", '>'),
    ("ldquo", '“'),
    ("lsquo", '‘'),
    ("lt", '<'),
    ("nbsp", ' '),
    ("quot", '"'),
    ("rdquo", '”'),
    ("rsquo", '’'),
];

/// Reads one HTML entity, returning the character and the bytes it occupied.
fn entity(rest: &str) -> Option<(char, usize)> {
    // Long enough for the longest name and for a numeric reference, short
    // enough that a stray ampersand in dialogue does not scan the whole line.
    let limit = rest.len().min(12);
    let close = rest.as_bytes()[..limit]
        .iter()
        .position(|byte| *byte == b';')?;
    let body = &rest[1..close];

    if let Some(digits) = body.strip_prefix('#') {
        let code = match digits.strip_prefix(['x', 'X']) {
            Some(hexadecimal) => u32::from_str_radix(hexadecimal, 16).ok()?,
            None => digits.parse().ok()?,
        };
        return char::from_u32(code).map(|character| (character, close + 1));
    }

    NAMED_ENTITIES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(body))
        .map(|(_, character)| (*character, close + 1))
}

#[cfg(test)]
mod tests {
    use super::clean;
    use crate::CuePosition;

    fn text(lines: &[&str]) -> String {
        clean(lines).text
    }

    #[test]
    fn removes_html_styling() {
        assert_eq!(text(&["<i>Hello</i>"]), "Hello");
        assert_eq!(text(&["<font color=\"#ffff00\">Hello</font>"]), "Hello");
        assert_eq!(text(&["<b><u>Hello</u></b>"]), "Hello");
    }

    #[test]
    fn keeps_punctuation_that_only_looks_like_a_tag() {
        assert_eq!(text(&["3 < 5 and 5 > 3"]), "3 < 5 and 5 > 3");
        assert_eq!(text(&["I <3 you"]), "I <3 you");
        assert_eq!(text(&["An unclosed <i thing"]), "An unclosed <i thing");
    }

    #[test]
    fn removes_override_blocks() {
        assert_eq!(text(&["{\\i1}Hello{\\i0}"]), "Hello");
        assert_eq!(text(&["{y:i}Hello"]), "Hello");
        assert_eq!(text(&["{\\pos(192,230)}Hello"]), "Hello");
    }

    #[test]
    fn keeps_braces_that_are_dialogue() {
        assert_eq!(
            text(&["He typed {hello} and left"]),
            "He typed {hello} and left"
        );
        assert_eq!(text(&["An unclosed { brace"]), "An unclosed { brace");
    }

    #[test]
    fn keeps_the_position_hint() {
        assert_eq!(
            clean(&["{\\an8}Above"]).position,
            Some(CuePosition::TopCentre)
        );
        assert_eq!(
            clean(&["{an8}Above"]).position,
            Some(CuePosition::TopCentre)
        );
        assert_eq!(
            clean(&["{\\a6}Above"]).position,
            Some(CuePosition::TopCentre)
        );
        assert_eq!(
            clean(&["{\\pos(1,2)\\an1}Left"]).position,
            Some(CuePosition::BottomLeft)
        );
        assert_eq!(clean(&["Plain"]).position, None);
    }

    #[test]
    fn ignores_commands_that_merely_begin_with_the_alignment_letter() {
        assert_eq!(clean(&["{\\alpha&H80&}Faded"]).position, None);
        assert_eq!(text(&["{\\alpha&H80&}Faded"]), "Faded");
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(text(&["Tom &amp; Jerry"]), "Tom & Jerry");
        assert_eq!(text(&["&lt;shouting&gt;"]), "<shouting>");
        assert_eq!(text(&["It&#39;s here"]), "It's here");
        assert_eq!(text(&["It&#x27;s here"]), "It's here");
        assert_eq!(text(&["Fish & chips"]), "Fish & chips");
    }

    #[test]
    fn treats_the_hard_break_as_a_line_break() {
        assert_eq!(text(&["First\\Nsecond"]), "First\nsecond");
        assert_eq!(text(&["Wide\\hspace"]), "Wide space");
    }

    #[test]
    fn joins_lines_and_trims_them() {
        assert_eq!(text(&["  First  ", "", "  second  "]), "First\nsecond");
    }

    #[test]
    fn reduces_markup_only_cues_to_nothing() {
        assert!(text(&["{\\an8}", "<i></i>"]).is_empty());
    }
}
