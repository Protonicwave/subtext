//! Subtitle tracks a scan finds inside the films themselves.

// The fixture stops a test outright when the library does not hold what the
// test is about to ask it for.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::path::Path;

use subtext_container::fixture::Entry;
use subtext_index::{TrackOrigin, TrackRecord};

use crate::common::Fixture;

/// A film of the shape most Matroska releases are.
fn entries() -> Vec<Entry> {
    vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
        Entry::subtitle(4, "S_TEXT/UTF8")
            .in_language("fre")
            .forced(),
    ]
}

fn tracks_of(library: &Fixture, relative: &str) -> Vec<TrackRecord> {
    library
        .database()
        .tracks()
        .for_film(library.film_id(relative))
        .unwrap()
}

#[test]
fn a_film_carrying_its_own_subtitles_says_so() {
    let library = Fixture::new();
    library.matroska("Heat.1995.1080p.mkv", entries());

    let outcome = library.scan();
    assert_eq!(outcome.films_probed, 1);
    assert_eq!(outcome.embedded_tracks, 2);

    let tracks = tracks_of(&library, "Heat.1995.1080p.mkv");
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().all(|track| track.origin.is_stream()));
    assert!(
        tracks
            .iter()
            .all(|track| track.path == library.path("Heat.1995.1080p.mkv"))
    );

    let english = tracks
        .iter()
        .find(|track| track.language.as_deref() == Some("en"))
        .unwrap();
    assert_eq!(english.stream_number, 3);
    assert_eq!(english.codec, "subrip");
    assert!(!english.forced);

    let french = tracks
        .iter()
        .find(|track| track.language.as_deref() == Some("fr"))
        .unwrap();
    assert!(french.forced);

    // Nothing has been read out of them yet, which is a separate piece of work
    // from knowing they are there.
    assert!(tracks.iter().all(|track| track.cue_count == 0));
}

#[test]
fn a_film_with_a_subtitle_file_beside_it_keeps_both() {
    let library = Fixture::new();
    library.matroska("Heat.1995.1080p.mkv", entries());
    library.subtitle("Heat.1995.1080p.srt", &["Don't let yourself"]);

    library.scan();

    let tracks = tracks_of(&library, "Heat.1995.1080p.mkv");
    assert_eq!(tracks.len(), 3);
    assert_eq!(
        tracks
            .iter()
            .filter(|track| track.origin == TrackOrigin::Sidecar)
            .count(),
        1
    );

    // And the file beside the film is the only one with anything in it.
    assert_eq!(
        library.dialogue("Heat.1995.1080p.mkv"),
        ["Don't let yourself"]
    );
}

#[test]
fn a_track_inside_a_film_is_not_a_subtitle_file_that_has_been_deleted() {
    let library = Fixture::new();
    library.matroska("Heat.1995.1080p.mkv", entries());
    library.scan();

    // The case that would break it: a rescan compares the folder against the
    // subtitle files it knows about, and a track inside a film is not one.
    let outcome = library.scan();
    assert_eq!(outcome.tracks_removed, 0);
    assert_eq!(tracks_of(&library, "Heat.1995.1080p.mkv").len(), 2);
}

#[test]
fn a_library_nobody_has_touched_is_not_opened_again() {
    let library = Fixture::new();
    library.matroska("Heat.1995.1080p.mkv", entries());
    library.matroska("Ronin.1998.1080p.mkv", entries());

    assert_eq!(library.scan().films_probed, 2);

    // The property the whole thing rests on: a rescan of a library of Matroska
    // files costs what a rescan of any other library costs.
    let again = library.scan();
    assert_eq!(again.films_probed, 0);
    assert_eq!(again.embedded_tracks, 0);
    assert_eq!(tracks_of(&library, "Heat.1995.1080p.mkv").len(), 2);
}

#[test]
fn a_film_that_has_been_replaced_is_opened_again() {
    let library = Fixture::new();
    library.matroska("Heat.1995.1080p.mkv", entries());
    library.scan();

    // A different encode of the same film, carrying one subtitle instead of
    // two, written over the top of the first.
    library.matroska(
        "Heat.1995.1080p.mkv",
        vec![
            Entry::video(1),
            Entry::subtitle(2, "S_TEXT/ASS").in_language("eng"),
        ],
    );
    touch(&library.path("Heat.1995.1080p.mkv"));

    let outcome = library.scan();
    assert_eq!(outcome.films_probed, 1);

    let tracks = tracks_of(&library, "Heat.1995.1080p.mkv");
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].stream_number, 2);
    assert_eq!(tracks[0].codec, "ass");
}

#[test]
fn a_track_of_pictures_is_recorded_and_named_for_what_it_is() {
    let library = Fixture::new();
    library.matroska(
        "Heat.1995.BluRay.mkv",
        vec![
            Entry::video(1),
            Entry::subtitle(2, "S_HDMV/PGS").in_language("eng"),
        ],
    );

    library.scan();

    let tracks = tracks_of(&library, "Heat.1995.BluRay.mkv");
    assert_eq!(tracks.len(), 1);
    // Reported rather than hidden, and named rather than pretended about.
    assert_eq!(tracks[0].codec, "pgs");
    assert_eq!(tracks[0].cue_count, 0);
}

#[test]
fn a_film_that_is_not_matroska_is_looked_at_once_and_left_alone() {
    let library = Fixture::new();
    library.film("Heat.1995.1080p.mp4");

    let outcome = library.scan();
    assert_eq!(outcome.films_probed, 1);
    assert_eq!(outcome.embedded_tracks, 0);
    assert!(tracks_of(&library, "Heat.1995.1080p.mp4").is_empty());

    // Being looked inside and found to hold nothing is an answer, and it is
    // what stops the film being opened on every scan for the rest of its life.
    assert_eq!(library.scan().films_probed, 0);
}

/// Moves a file's modification time on, as writing to it would.
fn touch(path: &Path) {
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
    file.set_modified(later).unwrap();
}
