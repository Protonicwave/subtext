//! What a scan promises about a folder of films.

// The fixture stops a test outright when the library does not hold what the
// test is about to ask it for.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::sync::Mutex;

use subtext_core::{ParseWarningKind, Timestamp};
use subtext_index::{SearchOptions, TrackMatch};
use subtext_scan::{ScanProgress, ScanStage, Silent};

use crate::common::{Fixture, exists, names};

#[test]
fn a_folder_of_films_becomes_a_library() {
    let library = Fixture::new();
    library.film("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
    library.subtitle(
        "The Matrix (1999).srt",
        &["Wake up, Neo.", "The Matrix has you."],
    );
    // A film in a folder of its own, which is how most collections are kept.
    library.film("Nineteen Nineties/Heat.1995.1080p.mkv");
    library.subtitle(
        "Nineteen Nineties/Heat.1995.en.srt",
        &["Don't let yourself"],
    );

    let outcome = library.scan();

    assert_eq!(outcome.films_found, 2);
    assert_eq!(outcome.subtitles_found, 2);
    assert_eq!(outcome.films_paired, 2);
    assert_eq!(outcome.subtitles_read, 2);
    assert_eq!(outcome.cues_indexed, 3);
    assert!(outcome.unpaired_subtitles.is_empty());
    assert!(outcome.films_without_subtitles.is_empty());
    assert!(outcome.warnings.is_empty());

    assert_eq!(
        library.dialogue("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv"),
        ["Wake up, Neo.", "The Matrix has you."]
    );
    let matrix = library
        .database()
        .films()
        .by_path(&library.path("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv"))
        .unwrap()
        .unwrap();
    assert_eq!(matrix.title, "The Matrix");
    assert_eq!(matrix.year, Some(1_999));

    let track = &library
        .database()
        .tracks()
        .for_film(library.film_id("Nineteen Nineties/Heat.1995.1080p.mkv"))
        .unwrap()[0];
    assert_eq!(track.language.as_deref(), Some("en"));
    assert_eq!(track.match_kind, TrackMatch::Exact);
    assert_eq!(track.encoding, "UTF-8");
}

#[test]
fn what_could_not_be_paired_is_reported_rather_than_guessed_at() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.film("Ronin.1998.mkv");
    library.subtitle("Heat.1995.srt", &["A line"]);
    // Nothing in the folder this could belong to.
    library.subtitle("Sicario.2015.srt", &["Another line"]);

    let outcome = library.scan();

    assert_eq!(outcome.films_paired, 1);
    assert_eq!(names(&outcome.unpaired_subtitles), ["Sicario.2015.srt"]);
    assert_eq!(names(&outcome.films_without_subtitles), ["Ronin.1998.mkv"]);
}

#[test]
fn two_films_of_the_same_name_leave_the_choice_to_a_person() {
    let library = Fixture::new();
    library.film("Boxes/Solaris.mkv");
    library.film("Shelf/Solaris.mp4");
    library.subtitle("Solaris.srt", &["A line"]);

    let outcome = library.scan();

    assert_eq!(outcome.films_found, 2);
    assert_eq!(outcome.films_paired, 0);
    assert_eq!(names(&outcome.unpaired_subtitles), ["Solaris.srt"]);
    assert_eq!(outcome.cues_indexed, 0);
}

#[test]
fn rescanning_an_unchanged_folder_reads_nothing_again() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["A line", "And another"]);
    assert_eq!(library.scan().cues_indexed, 2);

    // Something only the player knows, which a scan that rewrote the rows
    // underneath it would lose.
    let film_id = library.film_id("Heat.1995.mkv");
    library
        .database()
        .films()
        .set_duration(film_id, Timestamp::from_seconds(10_000))
        .unwrap();

    let again = library.scan();

    assert_eq!(again.films_found, 1);
    assert_eq!(again.subtitles_read, 0);
    assert_eq!(again.cues_indexed, 0);
    assert_eq!(again.tracks_removed, 0);
    assert_eq!(again.films_missing, 0);
    assert_eq!(library.dialogue("Heat.1995.mkv").len(), 2);
    assert_eq!(
        library
            .database()
            .films()
            .by_id(film_id)
            .unwrap()
            .unwrap()
            .duration,
        Some(Timestamp::from_seconds(10_000))
    );
}

#[test]
fn a_subtitle_file_that_has_been_edited_is_read_again() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["The first cut"]);
    library.scan();

    library.subtitle("Heat.1995.srt", &["The second cut", "with another line"]);
    let again = library.scan();

    assert_eq!(again.subtitles_read, 1);
    assert_eq!(
        library.dialogue("Heat.1995.mkv"),
        ["The second cut", "with another line"]
    );
}

#[test]
fn a_film_that_disappears_keeps_its_place_and_its_transcript() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["A line"]);
    library.scan();

    let film_id = library.film_id("Heat.1995.mkv");
    library
        .database()
        .positions()
        .save(film_id, Timestamp::from_seconds(2_400), None, false)
        .unwrap();

    // The drive is unplugged, taking the film and its subtitle with it.
    library.remove("Heat.1995.mkv");
    library.remove("Heat.1995.srt");
    let outcome = library.scan();

    assert_eq!(outcome.films_missing, 1);
    assert_eq!(
        outcome.tracks_removed, 0,
        "a missing film keeps its cues, so plugging the drive back in is free"
    );
    let film = library.database().films().by_id(film_id).unwrap().unwrap();
    assert!(film.is_missing());
    assert_eq!(
        library
            .database()
            .positions()
            .get(film_id)
            .unwrap()
            .unwrap()
            .position,
        Timestamp::from_seconds(2_400)
    );
    assert_eq!(library.dialogue("Heat.1995.mkv"), ["A line"]);

    // And back it comes.
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["A line"]);
    library.scan();
    assert!(
        !library
            .database()
            .films()
            .by_id(film_id)
            .unwrap()
            .unwrap()
            .is_missing()
    );
}

#[test]
fn a_subtitle_file_that_is_deleted_takes_its_lines_with_it() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["A line about a helicopter"]);
    library.scan();

    library.remove("Heat.1995.srt");
    let outcome = library.scan();

    assert_eq!(outcome.tracks_removed, 1);
    assert!(library.dialogue("Heat.1995.mkv").is_empty());
    let found = library
        .database()
        .search()
        .find("helicopter", &SearchOptions::default())
        .unwrap();
    assert!(found.films.is_empty());
}

#[test]
fn a_pairing_made_by_hand_survives_a_rescan() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.film("Ronin.1998.mkv");
    library.subtitle("Heat.1995.srt", &["A line"]);
    library.scan();

    let ronin = library.film_id("Ronin.1998.mkv");
    let track = library
        .database()
        .tracks()
        .by_path(&library.path("Heat.1995.srt"))
        .unwrap()
        .unwrap();
    library.database().tracks().attach(track.id, ronin).unwrap();

    library.scan();

    let attached = library.database().tracks().for_film(ronin).unwrap();
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0].match_kind, TrackMatch::ByHand);
    assert!(
        library
            .database()
            .tracks()
            .for_film(library.film_id("Heat.1995.mkv"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_subtitle_moves_when_a_better_film_arrives_beside_it() {
    let library = Fixture::new();
    library.film("Solaris Part Two.mkv");
    library.subtitle("Solaris.srt", &["A line"]);
    library.scan();
    assert_eq!(library.dialogue("Solaris Part Two.mkv").len(), 1);

    // The film it should have belonged to all along.
    library.film("Solaris.mkv");
    let outcome = library.scan();

    assert_eq!(
        outcome.subtitles_read, 0,
        "the file has not changed, so only the pairing needs writing"
    );
    assert_eq!(library.dialogue("Solaris.mkv"), ["A line"]);
    assert!(library.dialogue("Solaris Part Two.mkv").is_empty());
}

#[test]
fn hidden_and_system_folders_are_left_alone() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["A line"]);
    library.film("$RECYCLE.BIN/Ronin.1998.mkv");
    library.film(".hidden/Sicario.2015.mkv");
    library.film("Extras/._Heat.1995.mkv");
    library.write("Heat.1995.nfo", b"not a subtitle");

    let outcome = library.scan();

    assert_eq!(outcome.films_found, 1);
    // The count is of files the walk looked at, so it includes the ones it had
    // no use for and excludes everything inside a folder it did not enter.
    assert_eq!(outcome.files_seen, 4);
    assert_eq!(library.database().films().list().unwrap().len(), 1);
}

#[test]
fn a_broken_subtitle_file_indexes_what_it_can_and_says_what_was_wrong() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.write(
        "Heat.1995.srt",
        b"1\n00:00:01,000 --> broken\nLost.\n\n\
          2\n00:00:05,000 --> 00:00:06,000\nKept.\n",
    );

    let outcome = library.scan();

    assert_eq!(outcome.cues_indexed, 1);
    assert_eq!(library.dialogue("Heat.1995.mkv"), ["Kept."]);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].warnings[0].kind,
        ParseWarningKind::MalformedTiming
    );
}

#[test]
fn a_subtitle_file_in_another_encoding_is_read_as_what_it_is() {
    let library = Fixture::new();
    library.film("Amelie.2001.mkv");
    // Windows-1252, which is what a great many subtitle files are.
    let mut bytes = b"1\n00:00:01,000 --> 00:00:02,000\nUn caf".to_vec();
    bytes.extend_from_slice(&[0xE9]);
    bytes.extend_from_slice(b" au comptoir.\n");
    library.write("Amelie.2001.srt", &bytes);

    library.scan();

    assert_eq!(
        library.dialogue("Amelie.2001.mkv"),
        ["Un café au comptoir."]
    );
}

#[test]
fn a_folder_large_enough_to_index_in_one_pass_is_still_searchable() {
    let library = Fixture::new();
    // Over the threshold at which the search index is built once at the end
    // rather than kept in step, which is the path a first scan takes.
    for at in 0..40 {
        library.film(&format!("Film {at:02}.2001.mkv"));
        library.subtitle(
            &format!("Film {at:02}.2001.srt"),
            &[&format!("the {at:02} horsemen of somewhere")],
        );
    }

    let outcome = library.scan();
    assert_eq!(outcome.subtitles_read, 40);
    assert_eq!(outcome.cues_indexed, 40);

    let found = library
        .database()
        .search()
        .find("horsemen", &SearchOptions::default())
        .unwrap();
    assert_eq!(found.films.len(), 40);
}

#[test]
fn progress_moves_forwards_and_ends_where_it_says_it_will() {
    let library = Fixture::new();
    for at in 0..5 {
        library.film(&format!("Film {at}.2001.mkv"));
        library.subtitle(&format!("Film {at}.2001.srt"), &["A line", "And another"]);
    }

    let seen = Mutex::new(Vec::new());
    let sink = |progress: &ScanProgress| {
        if let Ok(mut seen) = seen.lock() {
            seen.push(*progress);
        }
    };
    library.scanner.scan(&library.folder, &sink).unwrap();

    let seen = seen.into_inner().unwrap_or_default();
    assert_eq!(
        seen.first().map(|first| first.stage),
        Some(ScanStage::Discovering)
    );
    let last = seen.last().copied().unwrap();
    assert_eq!(last.stage, ScanStage::Finished);
    assert_eq!(last.films_found, 5);
    assert_eq!(last.subtitles_to_read, 5);
    assert_eq!(last.subtitles_read, 5);
    assert_eq!(last.cues_indexed, 10);
    assert!((last.fraction_read() - 1.0).abs() < f32::EPSILON);

    for pair in seen.windows(2) {
        assert!(
            pair[1].cues_indexed >= pair[0].cues_indexed,
            "counts may only grow: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn only_the_folder_a_change_happened_in_is_rescanned() {
    let library = Fixture::new();
    library.film("Heat.1995.mkv");
    library.subtitle("Heat.1995.srt", &["A line"]);
    library.scan();

    let elsewhere = library.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let other = library.scanner.add_folder(&elsewhere).unwrap();

    let scanned = library
        .scanner
        .scan_containing(&[library.path("Ronin.1998.mkv")], &Silent)
        .unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].folder_id, library.folder.id);

    // A path in no watched folder is nothing to do with us.
    assert!(
        library
            .scanner
            .scan_containing(&["/somewhere/else/Film.mkv".into()], &Silent)
            .unwrap()
            .is_empty()
    );
    assert_eq!(library.scanner.scan_all(&Silent).unwrap().len(), 2);

    // Removing a folder forgets what was found in it, and only that.
    assert!(library.scanner.remove_folder(other.id).unwrap());
    assert_eq!(library.database().films().list().unwrap().len(), 1);
    assert!(exists(&library.path("Heat.1995.mkv")));
}

#[test]
fn an_empty_folder_is_a_scan_that_finds_nothing() {
    let library = Fixture::new();
    let outcome = library.scan();

    assert_eq!(outcome.files_seen, 0);
    assert_eq!(outcome.films_found, 0);
    assert_eq!(outcome.cues_indexed, 0);
    assert!(outcome.unreadable.is_empty());
    assert!(outcome.unpaired_subtitles.is_empty());
}
