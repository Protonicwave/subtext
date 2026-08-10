//! The parser against a corpus of subtitle files.
//!
//! Every file in `tests/fixtures` is a shape that real subtitle files take.
//! The sweep at the end holds the parser to its two promises across all of
//! them: it never panics, and whatever comes out is usable.

#![allow(clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use subtext_core::{CuePosition, ParseOutcome, ParseWarningKind, Timestamp, parse_srt};

fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn parse(name: &str) -> ParseOutcome {
    let path = directory().join(name);
    let bytes =
        fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    parse_srt(&bytes)
}

fn texts(name: &str) -> Vec<String> {
    parse(name)
        .track
        .cues()
        .iter()
        .map(|cue| cue.text.clone())
        .collect()
}

fn kinds(name: &str) -> Vec<ParseWarningKind> {
    parse(name)
        .warnings
        .iter()
        .map(|warning| warning.kind)
        .collect()
}

/// The three cues shared by the fixtures that differ only in how they are
/// written to disk.
fn the_usual_three() -> Vec<String> {
    vec![
        "It is a curious thing, this.".to_owned(),
        "Watching a film\nand reading it at once.".to_owned(),
        "Quite.".to_owned(),
    ]
}

#[test]
fn a_well_formed_file() {
    let outcome = parse("well-formed.srt");
    assert!(!outcome.has_warnings());

    let cues = outcome.track.cues();
    assert_eq!(cues.len(), 3);
    assert_eq!(cues[0].start, Timestamp::from_millis(1_000));
    assert_eq!(cues[0].end, Timestamp::from_millis(4_000));
    assert_eq!(cues[2].start, Timestamp::from_millis(60_000));
    assert_eq!(cues[2].end, Timestamp::from_millis(62_500));
    assert_eq!(texts("well-formed.srt"), the_usual_three());
}

#[test]
fn line_endings_make_no_difference() {
    for name in [
        "windows-line-endings.srt",
        "carriage-return-line-endings.srt",
        "no-trailing-newline.srt",
    ] {
        assert_eq!(texts(name), the_usual_three(), "{name}");
        assert!(kinds(name).is_empty(), "{name}");
    }
}

#[test]
fn a_file_with_a_mixture_of_line_endings() {
    assert_eq!(texts("mixed-line-endings.srt"), ["First.", "Second."]);
    assert!(kinds("mixed-line-endings.srt").is_empty());
}

#[test]
fn byte_order_marks_are_consumed_rather_than_read_as_text() {
    for (name, encoding) in [
        ("utf8-byte-order-mark.srt", "UTF-8"),
        ("utf16-little-endian.srt", "UTF-16LE"),
        ("utf16-big-endian.srt", "UTF-16BE"),
    ] {
        let outcome = parse(name);
        assert_eq!(outcome.track.encoding(), encoding, "{name}");
        assert_eq!(texts(name), the_usual_three(), "{name}");
        assert!(!outcome.has_warnings(), "{name}");
    }
}

#[test]
fn sixteen_bit_text_is_recognised_without_a_byte_order_mark() {
    let outcome = parse("utf16-no-byte-order-mark.srt");
    assert_eq!(outcome.track.encoding(), "UTF-16LE");
    assert_eq!(texts("utf16-no-byte-order-mark.srt"), the_usual_three());
}

#[test]
fn single_byte_encodings_are_guessed_from_the_bytes() {
    let western = parse("windows-1252.srt");
    assert_eq!(western.track.encoding(), "windows-1252");
    assert_eq!(
        texts("windows-1252.srt"),
        ["Voilà, un café à Paris.", "Très bien. That will be £5."]
    );

    let cyrillic = parse("windows-1251.srt");
    assert_eq!(cyrillic.track.encoding(), "windows-1251");
    assert_eq!(
        texts("windows-1251.srt"),
        ["Здравствуйте, как дела?", "Хорошо, спасибо."]
    );
}

#[test]
fn bytes_that_belong_to_no_encoding_are_reported() {
    let outcome = parse("undecodable-bytes.srt");
    assert_eq!(outcome.track.len(), 2);
    assert_eq!(outcome.track.cues()[0].text, "Mostly readable.");
    assert_eq!(outcome.track.cues()[1].text, "\u{fffd}Damaged in transit.");
    assert!(kinds("undecodable-bytes.srt").contains(&ParseWarningKind::UndecodableBytes));
}

#[test]
fn numbering_is_recovered_when_it_is_absent_or_wrong() {
    assert_eq!(
        texts("no-indices.srt"),
        ["First without a number.", "Second without a number."]
    );
    assert_eq!(kinds("no-indices.srt"), [ParseWarningKind::MissingIndex]);

    assert_eq!(texts("non-numeric-indices.srt"), ["First.", "Second."]);
    assert_eq!(
        kinds("non-numeric-indices.srt"),
        [ParseWarningKind::MalformedIndex]
    );

    // The numbering in the file is ignored, so repeats are not a problem.
    assert_eq!(
        texts("repeated-indices.srt"),
        ["First.", "Second.", "Third."]
    );
    assert!(kinds("repeated-indices.srt").is_empty());

    assert_eq!(texts("index-on-the-timing-line.srt"), ["First.", "Second."]);
    assert_eq!(
        kinds("index-on-the-timing-line.srt"),
        [ParseWarningKind::MissingIndex]
    );
}

#[test]
fn cues_are_found_however_they_are_separated() {
    assert_eq!(texts("no-blank-lines.srt"), ["First.", "Second.", "Third."]);
    assert!(kinds("no-blank-lines.srt").is_empty());

    assert_eq!(texts("extra-blank-lines.srt"), ["First.", "Second."]);
    assert!(kinds("extra-blank-lines.srt").is_empty());
}

#[test]
fn cues_come_back_in_playback_order() {
    let outcome = parse("out-of-order.srt");
    assert_eq!(
        texts("out-of-order.srt"),
        ["First by time.", "Second by time.", "Third by time."]
    );
    assert_eq!(kinds("out-of-order.srt"), [ParseWarningKind::OutOfOrder]);
    assert_eq!(outcome.track.cues()[0].index, 1);
    assert_eq!(outcome.track.cues()[2].index, 3);
}

#[test]
fn overlapping_cues_are_kept_and_can_still_be_looked_up() {
    let outcome = parse("overlapping.srt");
    assert_eq!(outcome.track.len(), 3);
    assert_eq!(
        kinds("overlapping.srt"),
        [ParseWarningKind::OverlappingCues]
    );

    // The sign runs underneath the dialogue, so both are on screen at 2.5s and
    // the later one wins, but the sign is still there once the line has gone.
    let track = &outcome.track;
    assert_eq!(
        track.cue_at(Timestamp::from_millis(2_500)).unwrap().text,
        "Someone speaking over it."
    );
    assert_eq!(
        track.cue_at(Timestamp::from_millis(3_500)).unwrap().text,
        "A sign on the wall."
    );
}

#[test]
fn timings_are_read_in_all_the_forms_files_use() {
    let full_stops = parse("full-stop-milliseconds.srt");
    assert_eq!(
        full_stops.track.cues()[0].end,
        Timestamp::from_millis(2_500)
    );
    assert_eq!(
        full_stops.track.cues()[1].start,
        Timestamp::from_millis(3_250)
    );
    assert!(!full_stops.has_warnings());

    let short = parse("short-timings.srt");
    assert_eq!(short.track.cues()[0].start, Timestamp::from_millis(1_500));
    assert_eq!(short.track.cues()[0].end, Timestamp::from_millis(2_250));
    assert_eq!(short.track.cues()[1].start, Timestamp::from_millis(3_000));
    assert!(!short.has_warnings());

    let coordinates = parse("display-coordinates.srt");
    assert_eq!(
        coordinates.track.cues()[0].end,
        Timestamp::from_millis(2_000)
    );
    assert!(!coordinates.has_warnings());
}

#[test]
fn markup_is_reduced_to_text() {
    assert_eq!(
        texts("html-tags.srt"),
        ["Whispered.", "Shouted.", "Six < seven > five."]
    );
    assert_eq!(texts("entities.srt"), ["Tom & Jerry", "It's <here>"]);
}

#[test]
fn position_hints_survive_the_markup_being_removed() {
    let outcome = parse("ass-tags.srt");
    let cues = outcome.track.cues();
    assert_eq!(cues[0].text, "A sign above the door.");
    assert_eq!(cues[0].position, Some(CuePosition::TopCentre));
    assert_eq!(cues[1].text, "Leaning in.");
    assert_eq!(cues[1].position, None);
    assert_eq!(cues[2].position, Some(CuePosition::BottomLeft));
    assert_eq!(cues[3].text, "Placed by hand.");
    assert_eq!(cues[3].position, None);
}

#[test]
fn cues_with_nothing_to_say_are_dropped() {
    assert_eq!(
        texts("blank-cues.srt"),
        ["Real dialogue.", "More real dialogue."]
    );
    assert_eq!(kinds("blank-cues.srt"), [ParseWarningKind::EmptyCue]);
    assert_eq!(parse("blank-cues.srt").warnings[0].count, 2);
}

#[test]
fn a_broken_block_does_not_take_the_rest_of_the_file_with_it() {
    assert_eq!(texts("malformed-timings.srt"), ["Kept.", "Kept as well."]);
    assert_eq!(
        kinds("malformed-timings.srt"),
        [
            ParseWarningKind::MalformedTiming,
            ParseWarningKind::UnexpectedLine
        ]
    );
}

#[test]
fn a_backwards_cue_keeps_its_words() {
    let outcome = parse("negative-duration.srt");
    let cue = &outcome.track.cues()[0];
    assert_eq!(cue.text, "Ends before it starts.");
    assert_eq!(cue.start, Timestamp::from_millis(5_000));
    assert_eq!(cue.end, cue.start);
    assert_eq!(
        kinds("negative-duration.srt"),
        [ParseWarningKind::NegativeDuration]
    );
}

#[test]
fn a_file_that_stops_early_keeps_what_it_managed_to_write() {
    assert_eq!(
        texts("truncated-mid-cue.srt"),
        ["Complete.", "Cut off part way thro"]
    );
    assert!(kinds("truncated-mid-cue.srt").is_empty());

    assert_eq!(texts("truncated-after-index.srt"), ["Complete."]);
    assert_eq!(
        kinds("truncated-after-index.srt"),
        [ParseWarningKind::TruncatedCue]
    );
}

#[test]
fn files_that_hold_no_subtitles_at_all() {
    assert!(parse("empty.srt").track.is_empty());
    assert!(!parse("empty.srt").has_warnings());

    assert!(parse("only-blank-lines.srt").track.is_empty());
    assert!(!parse("only-blank-lines.srt").has_warnings());

    assert!(parse("not-a-subtitle-file.srt").track.is_empty());
    assert_eq!(
        kinds("not-a-subtitle-file.srt"),
        [ParseWarningKind::UnexpectedLine]
    );
}

/// Holds every fixture to the promises the parser makes, whatever is in it.
#[test]
fn every_fixture_parses_to_something_usable() {
    let mut seen = 0;
    for entry in fs::read_dir(directory()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "srt") {
            continue;
        }
        seen += 1;

        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let outcome = parse_srt(&fs::read(&path).unwrap());
        let cues = outcome.track.cues();

        for pair in cues.windows(2) {
            assert!(pair[0].start <= pair[1].start, "{name}: cues out of order");
        }
        for (position, cue) in cues.iter().enumerate() {
            assert_eq!(
                cue.index as usize,
                position + 1,
                "{name}: numbering is not sequential"
            );
            assert!(cue.start <= cue.end, "{name}: cue ends before it starts");
            assert!(!cue.text.trim().is_empty(), "{name}: cue has no text");
            assert!(
                !cue.text.contains('\r'),
                "{name}: carriage return left in the text"
            );
            assert!(
                !cue.text.contains("-->"),
                "{name}: a timing line was read as text"
            );
        }
    }

    // The corpus is the point of this file, so a fixture disappearing should
    // fail rather than quietly shrink the coverage.
    assert!(
        seen >= 20,
        "expected at least twenty fixtures, found {seen}"
    );
}
