//! A subtitle that does not match its film, made to match it.

#![allow(clippy::unwrap_used)]

mod common;

use subtext_core::{Correction, Timestamp};
use subtext_index::SearchOptions;

use crate::common::{Library, cues};

/// A film whose subtitle runs two and a half seconds ahead of the dialogue.
fn mistimed() -> (Library, i64, i64) {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let track = library.add_track(film, "Heat");
    library
        .database
        .tracks()
        .replace_cues(
            track,
            &cues(&[
                "The action is the juice",
                "Don't let yourself get attached",
                "I do what I do best",
            ]),
        )
        .unwrap();

    (library, film, track)
}

fn starts(library: &Library, track: i64) -> Vec<u32> {
    library
        .database
        .tracks()
        .cues(track)
        .unwrap()
        .iter()
        .map(|cue| cue.start.millis())
        .collect()
}

#[test]
fn an_uncorrected_track_reads_back_exactly_as_it_was_written() {
    let (library, _, track) = mistimed();

    let read = library.database.tracks().cues(track).unwrap();
    assert_eq!(starts(&library, track), [0, 10_000, 20_000]);
    assert_eq!(read[0].end, Timestamp::from_millis(4_000));

    // Not merely equal to the identity, but the identity, so that nothing
    // downstream has to ask whether a correction was applied.
    let stored = library.database.tracks().by_id(track).unwrap().unwrap();
    assert!(stored.correction.is_identity());
}

#[test]
fn a_correction_moves_every_line_as_the_cues_are_read() {
    let (library, _, track) = mistimed();
    library
        .database
        .tracks()
        .set_correction(track, Correction::of_offset(2_500))
        .unwrap();

    assert_eq!(starts(&library, track), [2_500, 12_500, 22_500]);

    // The end of a line moves with its start, so a corrected line is on screen
    // for exactly as long as it was.
    let read = library.database.tracks().cues(track).unwrap();
    assert_eq!(read[0].end, Timestamp::from_millis(6_500));
}

/// Working out a correction means seeing the timings the file claims, since a
/// track measured through its own last answer would only ever yield what was
/// left over from it.
#[test]
fn the_authored_read_is_untouched_by_whatever_correction_is_in_force() {
    let (library, _, track) = mistimed();
    let before = library.database.tracks().authored_cues(track).unwrap();

    for correction in [
        Correction::of_offset(2_500),
        Correction::of_offset(-1_000),
        Correction::new(4_000, 25.0 / 24.0),
    ] {
        library
            .database
            .tracks()
            .set_correction(track, correction)
            .unwrap();

        let authored = library.database.tracks().authored_cues(track).unwrap();
        let starts: Vec<u32> = authored.iter().map(|cue| cue.start.millis()).collect();
        assert_eq!(starts, [0, 10_000, 20_000]);
        assert_eq!(authored[0].end, Timestamp::from_millis(4_000));

        // The same lines in the same order, so that the only difference between
        // the two reads is the arithmetic.
        assert_eq!(authored.len(), before.len());
        for (line, was) in authored.iter().zip(&before) {
            assert_eq!(line.index, was.index);
            assert_eq!(line.text, was.text);
        }
    }
}

#[test]
fn a_correction_outlives_a_restart() {
    let (library, film, track) = mistimed();
    library
        .database
        .tracks()
        .set_correction(track, Correction::new(-1_200, 25.0 / 23.976))
        .unwrap();

    let reopened = library.reopen();
    let stored = reopened.tracks().for_film(film).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].correction.offset_ms(), -1_200);
    assert!((stored[0].correction.rate() - 25.0 / 23.976).abs() < f64::EPSILON);

    // 10,000 stretched to 10,427 and then pulled back by the offset.
    assert_eq!(
        reopened.tracks().cues(track).unwrap()[1].start,
        Timestamp::from_millis(9_227)
    );
}

#[test]
fn a_search_result_opens_at_the_corrected_moment() {
    let (library, _, track) = mistimed();
    let uncorrected = library
        .database
        .search()
        .find("juice", &SearchOptions::default())
        .unwrap();
    assert_eq!(uncorrected.films[0].hits[0].start, Timestamp::ZERO);

    library
        .database
        .tracks()
        .set_correction(track, Correction::of_offset(2_500))
        .unwrap();

    // The point of the whole arrangement: the palette and the player agree
    // about where a line is, because both went through the same operation.
    let corrected = library
        .database
        .search()
        .find("attached", &SearchOptions::default())
        .unwrap();
    assert_eq!(
        corrected.films[0].hits[0].start,
        Timestamp::from_millis(12_500)
    );
    assert_eq!(
        corrected.films[0].hits[0].start,
        library.database.tracks().cues(track).unwrap()[1].start
    );
}

#[test]
fn a_track_given_to_another_film_forgets_the_correction() {
    let (library, film, track) = mistimed();
    let folder = library.watch();
    let other = library.add_film(folder, "Ronin");

    library
        .database
        .tracks()
        .set_correction(track, Correction::of_offset(2_500))
        .unwrap();

    // Attaching it where it already is changes nothing: somebody confirming a
    // pairing has not told us the timings are wrong.
    library.database.tracks().attach(track, film).unwrap();
    assert_eq!(
        library
            .database
            .tracks()
            .by_id(track)
            .unwrap()
            .unwrap()
            .correction
            .offset_ms(),
        2_500
    );

    // Attaching it to a different film does. The number was arrived at by ear
    // against one release and says nothing about another.
    library.database.tracks().attach(track, other).unwrap();
    assert!(
        library
            .database
            .tracks()
            .by_id(track)
            .unwrap()
            .unwrap()
            .correction
            .is_identity()
    );
}

#[test]
fn reading_a_file_again_leaves_its_correction_alone() {
    let (library, film, track) = mistimed();
    library
        .database
        .tracks()
        .set_correction(track, Correction::of_offset(2_500))
        .unwrap();

    // A subtitle file that has been written to is parsed again, and the cues
    // are replaced. The correction describes how the file sits against the
    // film, which editing the file does not answer either way.
    library.add_track(film, "Heat");
    library
        .database
        .tracks()
        .replace_cues(track, &cues(&["A different line"]))
        .unwrap();

    assert_eq!(starts(&library, track), [2_500]);
}
