//! Which cues are somebody talking.
//!
//! A subtitle for the hearing impaired captions the sounds a film makes as well
//! as the words in it. A cue reading `[DOOR SLAMS]` marks a moment when nobody
//! is speaking, and the signal it is about to become is measured against a
//! reading that looks for voices. That is not noise which averages out. It is an
//! error pointing one way, and it falls hardest on the tracks most likely to
//! need the help.
//!
//! So a cue that is only a description, only a lyric, only the name of whoever
//! is about to speak, or only punctuation is left out of the signal. A cue that
//! carries a description alongside a line of dialogue stays, because somebody
//! still starts speaking in it and that moment is what is being lined up.
//!
//! The rule reads the cue and not the track's label. Whether a file says it is
//! for the hearing impaired is a claim about the file, and a plain track is as
//! likely to caption a phone ringing as a labelled one is to caption nothing at
//! all. Whether somebody speaks in a cue is a property of that cue and can be
//! read from it.
//!
//! Nothing here changes what is drawn. It is applied where a cue signal is built
//! and nowhere else, so the player, the library and the file itself carry every
//! cue the file wrote, descriptions and all.

use subtext_core::Cue;

/// Whether somebody speaks in this cue.
pub(crate) fn is_spoken(cue: &Cue) -> bool {
    // Brackets are counted across the whole cue rather than a line at a time,
    // because a description long enough to wrap leaves its closing bracket on
    // the second line, and a line ending in one would otherwise read as an
    // ordinary line of dialogue.
    let mut inside = 0_u32;
    cue.text.lines().any(|line| speech_in(line, &mut inside))
}

/// Whether this line has somebody speaking on it, with `inside` carrying how
/// many descriptions are still open when it begins and when it ends.
fn speech_in(line: &str, inside: &mut u32) -> bool {
    // Something has been said once a letter or a digit falls outside every
    // description and after any name the line is attributed to.
    let mut said = false;
    // Whether what has been read since the last colon is written the way a
    // sentence is, which is what tells a name from an ordinary line.
    let mut lower = false;
    // Whether the line has yet reached anything that is not a space.
    let mut fresh = true;

    for character in line.chars() {
        if is_opening(character) {
            *inside += 1;
            fresh = false;
            continue;
        }
        if is_closing(character) {
            // Saturating rather than wrapping, since a file is perfectly
            // capable of closing a bracket it never opened and the honest
            // reading of that is that nothing was open.
            *inside = inside.saturating_sub(1);
            fresh = false;
            continue;
        }
        if *inside > 0 {
            continue;
        }

        if is_lyric(character, fresh) {
            return false;
        }
        if !character.is_whitespace() {
            fresh = false;
        }

        if is_label_end(character) {
            // A name, and then a colon, and then what that person says. Only
            // where nothing before the colon was written in lower case: a
            // sentence ending in one is a sentence, and a line that reads
            // "and then he said:" is somebody talking.
            if !lower {
                said = false;
            }
            lower = false;
            continue;
        }
        if character.is_alphanumeric() {
            said = true;
            if character.is_lowercase() {
                lower = true;
            }
        }
    }

    said
}

/// The brackets a description is written inside.
///
/// The square and round conventions, in the widths both are written in, which
/// covers what subtitle files actually use. Braces are not among them: they
/// carry substation alpha styling rather than description, and the parser has
/// already taken those out by the time a cue reaches this crate.
fn is_opening(character: char) -> bool {
    matches!(character, '[' | '(' | '［' | '（')
}

fn is_closing(character: char) -> bool {
    matches!(character, ']' | ')' | '］' | '）')
}

/// Whether the line is being sung rather than spoken.
///
/// A note anywhere on a line marks the whole of it, since a file that marks
/// lyrics marks both ends of them and a continuation carries one. The hash is
/// the same convention where a file cannot write a note, and it only counts at
/// the start of a line, where nothing else puts one: mid-line it is a number.
fn is_lyric(character: char, fresh: bool) -> bool {
    matches!(character, '♪' | '♫' | '♬' | '♩') || (character == '#' && fresh)
}

/// The colon a speaker's name is followed by, in both the widths files use.
fn is_label_end(character: char) -> bool {
    matches!(character, ':' | '：')
}

#[cfg(test)]
mod tests {
    use super::is_spoken;
    use subtext_core::{Cue, Timestamp};

    fn saying(text: &str) -> Cue {
        Cue {
            index: 1,
            start: Timestamp::from_millis(1_000),
            end: Timestamp::from_millis(3_000),
            text: text.to_owned(),
            position: None,
        }
    }

    #[test]
    fn a_description_on_its_own_is_not_speech() {
        for text in [
            "[DOOR SLAMS]",
            "[ sighs ]",
            "(GUNFIRE)",
            "(laughs)",
            "［ため息］",
            "（ドアが閉まる）",
            "[BOTH GRUNTING\nAND SHOUTING]",
            "[MAN ON RADIO]\n[STATIC]",
        ] {
            assert!(!is_spoken(&saying(text)), "kept {text:?}");
        }
    }

    #[test]
    fn a_lyric_on_its_own_is_not_speech() {
        for text in [
            "♪ Somewhere over the rainbow ♪",
            "♪ Way up high",
            "♫ La la la ♫",
            "# And the band played on #",
            "♪♪",
        ] {
            assert!(!is_spoken(&saying(text)), "kept {text:?}");
        }
    }

    #[test]
    fn a_name_with_nothing_after_it_is_not_speech() {
        for text in ["MAN:", "NARRATOR :", "- WOMAN:", "MAN (O.S.):", "男:"] {
            assert!(!is_spoken(&saying(text)), "kept {text:?}");
        }
    }

    #[test]
    fn punctuation_on_its_own_is_not_speech() {
        for text in ["...", "-", "- -", "?!", "「……」", "。", "  "] {
            assert!(!is_spoken(&saying(text)), "kept {text:?}");
        }
    }

    #[test]
    fn a_description_beside_a_line_of_dialogue_is_speech() {
        for text in [
            "[SIGHS] I'm fine.",
            "(quietly) Get down.",
            "[DOOR SLAMS]\nWhere have you been?",
            "MAN: Get down!",
            "- WOMAN: Run.\n- MAN: Where?",
            "男: 逃げろ",
        ] {
            assert!(is_spoken(&saying(text)), "dropped {text:?}");
        }
    }

    /// Brackets, colons and numbers all turn up inside ordinary dialogue, and
    /// none of them says a cue is anything other than somebody talking.
    #[test]
    fn dialogue_that_happens_to_carry_the_marks_is_still_speech() {
        for text in [
            "It cost five pounds (about six dollars).",
            "And then he said:",
            "I'll be there at 8:30.",
            "Room 4 is that way.",
            "The number of the room is #4.",
            "¿Qué haces aquí?",
            "こんにちは",
        ] {
            assert!(is_spoken(&saying(text)), "dropped {text:?}");
        }
    }

    /// A bracket closed but never opened leaves the line alone, which is what a
    /// file that lost its opening bracket needs. One opened and never closed
    /// runs to the end of the cue, which is the cost of reading a description
    /// that wraps onto a second line, and it is the cheaper mistake of the two:
    /// files wrap descriptions constantly and leave brackets open rarely.
    #[test]
    fn a_bracket_that_does_not_match_is_read_as_best_it_can_be() {
        assert!(is_spoken(&saying("Get down!]")));
        assert!(!is_spoken(&saying("[MAN SHOUTING")));
        assert!(!is_spoken(&saying("[MAN SHOUTING\nGet down!")));
    }

    #[test]
    fn a_cue_with_no_text_at_all_is_not_speech() {
        assert!(!is_spoken(&saying("")));
        assert!(!is_spoken(&saying("\n\n")));
    }
}
