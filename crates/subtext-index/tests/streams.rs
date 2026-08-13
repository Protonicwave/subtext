//! Subtitle tracks that live inside the film rather than beside it.

// A test that cannot get at the film it is about to check has nothing to say,
// so it stops rather than passing quietly.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;

use subtext_core::SubtitleLabel;
use subtext_index::{NewTrack, TrackChoice, TrackMatch, TrackOrigin, TrackRecord};

use crate::common::Library;

/// A track as a probe of a film's header would describe it.
fn stream<'a>(film_id: i64, path: &'a Path, number: u64, codec: &'a str) -> NewTrack<'a> {
    NewTrack {
        film_id,
        path,
        origin: TrackOrigin::Stream,
        stream_number: number,
        codec,
        label: SubtitleLabel {
            language: Some("en"),
            forced: false,
            hearing_impaired: false,
        },
        match_kind: TrackMatch::Exact,
        // A stream is not a file that had to be guessed at, and the
        // specification settles the question anyway.
        encoding: "UTF-8",
        size_bytes: 0,
        modified_at: 0,
    }
}

fn tracks_of(library: &Library, film_id: i64) -> Vec<TrackRecord> {
    library.database.tracks().for_film(film_id).unwrap()
}

#[test]
fn several_tracks_of_one_film_share_its_path() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let path = library.root.join("Heat.mkv");

    library
        .database
        .tracks()
        .write_streams(&[(
            film,
            vec![
                stream(film, &path, 2, "subrip"),
                stream(film, &path, 3, "pgs"),
            ],
        )])
        .unwrap();

    let tracks = tracks_of(&library, film);
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().all(|track| track.origin.is_stream()));
    assert!(tracks.iter().all(|track| track.path == path));
    assert_eq!(tracks[0].stream_number, 2);
    assert_eq!(tracks[0].codec, "subrip");
    assert_eq!(tracks[1].codec, "pgs");

    // Nothing has been read out of them, which is a separate piece of work.
    assert!(tracks.iter().all(|track| track.cue_count == 0));
}

#[test]
fn a_film_probed_again_keeps_the_tracks_it_still_carries() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let path = library.root.join("Heat.mkv");
    let found = vec![
        stream(film, &path, 2, "subrip"),
        stream(film, &path, 3, "subrip"),
    ];

    library
        .database
        .tracks()
        .write_streams(&[(film, found.clone())])
        .unwrap();
    let before = tracks_of(&library, film);

    library
        .database
        .tracks()
        .write_streams(&[(film, found)])
        .unwrap();
    let after = tracks_of(&library, film);

    // The identifiers are what a chosen track and a correction point at, so a
    // second probe finding the same tracks must not hand out new ones.
    assert_eq!(
        before.iter().map(|track| track.id).collect::<Vec<_>>(),
        after.iter().map(|track| track.id).collect::<Vec<_>>()
    );
}

#[test]
fn a_track_the_film_no_longer_carries_is_taken_away() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let path = library.root.join("Heat.mkv");

    library
        .database
        .tracks()
        .write_streams(&[(
            film,
            vec![
                stream(film, &path, 2, "subrip"),
                stream(film, &path, 3, "subrip"),
            ],
        )])
        .unwrap();

    // The film replaced by an encode carrying one subtitle instead of two.
    library
        .database
        .tracks()
        .write_streams(&[(film, vec![stream(film, &path, 2, "subrip")])])
        .unwrap();
    let tracks = tracks_of(&library, film);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].stream_number, 2);

    // And one carrying none at all, which is the empty list rather than the
    // film being left out.
    library
        .database
        .tracks()
        .write_streams(&[(film, Vec::new())])
        .unwrap();
    assert!(tracks_of(&library, film).is_empty());
}

#[test]
fn a_subtitle_file_beside_the_film_is_left_alone_by_all_of_it() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let sidecar = library.add_track(film, "Heat");
    let path = library.root.join("Heat.mkv");

    library
        .database
        .tracks()
        .write_streams(&[(film, vec![stream(film, &path, 2, "subrip")])])
        .unwrap();
    library
        .database
        .tracks()
        .write_streams(&[(film, Vec::new())])
        .unwrap();

    let tracks = tracks_of(&library, film);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, sidecar);

    // And a rescan compares the folder against the files only. A track inside
    // the film is not a file that has been deleted.
    let pairings = library.database.tracks().pairings(folder).unwrap();
    assert_eq!(pairings.len(), 1);
    assert_eq!(pairings[0].id, sidecar);
}

#[test]
fn a_chosen_track_that_goes_leaves_the_film_choosing_again() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let path = library.root.join("Heat.mkv");

    library
        .database
        .tracks()
        .write_streams(&[(film, vec![stream(film, &path, 2, "subrip")])])
        .unwrap();
    let chosen = tracks_of(&library, film)[0].id;
    library
        .database
        .films()
        .set_choice(film, TrackChoice::Track(chosen))
        .unwrap();

    library
        .database
        .tracks()
        .write_streams(&[(film, Vec::new())])
        .unwrap();

    let film = library.database.films().by_id(film).unwrap().unwrap();
    assert_eq!(film.choice, TrackChoice::Unset);
}

#[test]
fn a_film_is_looked_inside_once_and_then_left_alone() {
    let library = Library::new();
    let folder = library.watch();
    let first = library.add_film(folder, "Heat");
    let second = library.add_film(folder, "Ronin");

    let unprobed = library.database.films().unprobed(folder).unwrap();
    assert_eq!(unprobed.len(), 2);

    library.database.films().mark_probed(&[first]).unwrap();
    assert_eq!(
        library.database.films().unprobed(folder).unwrap(),
        vec![second]
    );

    // And it stays that way across a restart, which is what stops a library
    // being opened film by film every time the application starts.
    assert_eq!(library.reopen().films().unprobed(folder).unwrap(), [second]);
}
