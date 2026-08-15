//! What the library database promises the rest of the application.

// One test stops a scan the way a closed application does, which takes a panic.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use subtext_core::{Cue, CuePosition, SubtitleLabel, Timestamp};
use subtext_index::{Database, NewFilm, NewTrack, TrackMatch, TrackOrigin};

use crate::common::{Library, cues};

#[test]
fn a_new_file_is_migrated_and_reopening_it_changes_nothing() {
    let library = Library::new();
    let folder = library.watch();
    library.add_film(folder, "Heat");

    // A restart runs the migrations again, which must find nothing to do and
    // leave what is already there alone.
    let reopened = library.reopen();
    assert_eq!(reopened.films().list().unwrap().len(), 1);
    assert_eq!(Database::schema_version(), 6);
}

#[test]
fn watching_the_same_folder_twice_watches_it_once() {
    let library = Library::new();
    let first = library.database.folders().add(&library.root).unwrap();
    let second = library.database.folders().add(&library.root).unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.added_at, second.added_at);
    assert_eq!(library.database.folders().list().unwrap().len(), 1);
}

#[test]
fn rescanning_an_unchanged_library_writes_nothing() {
    let library = Library::new();
    let folder = library.watch();
    let film = NewFilm {
        folder_id: folder,
        path: &library.root.join("Heat.mkv"),
        title: "Heat",
        year: Some(1_995),
        size_bytes: 4_000,
        modified_at: 1_700_000_000_000,
    };

    let first = library.database.films().upsert(&film).unwrap();
    assert!(first.changed);

    let second = library.database.films().upsert(&film).unwrap();
    assert_eq!(second.id, first.id);
    assert!(
        !second.changed,
        "a film whose file has not changed must not be written again"
    );

    // The same for subtitle files, which is what stops a rescan reparsing a
    // thousand of them for nothing.
    let track = NewTrack {
        film_id: first.id,
        path: &library.root.join("Heat.srt"),
        label: SubtitleLabel::default(),
        origin: TrackOrigin::Sidecar,
        stream_number: 0,
        codec: "subrip",
        match_kind: TrackMatch::Exact,
        encoding: "UTF-8",
        size_bytes: 60_000,
        modified_at: 1_700_000_000_000,
    };
    assert!(library.database.tracks().upsert(&track).unwrap().changed);
    assert!(!library.database.tracks().upsert(&track).unwrap().changed);
}

#[test]
fn a_file_that_has_been_written_to_is_read_again() {
    let library = Library::new();
    let folder = library.watch();
    let path = library.root.join("Heat.mkv");
    let mut film = NewFilm {
        folder_id: folder,
        path: &path,
        title: "Heat",
        year: Some(1_995),
        size_bytes: 4_000,
        modified_at: 1_700_000_000_000,
    };
    let first = library.database.films().upsert(&film).unwrap();

    film.modified_at = 1_700_000_500_000;
    let second = library.database.films().upsert(&film).unwrap();

    assert_eq!(second.id, first.id);
    assert!(second.changed);
    let stored = library.database.films().by_path(&path).unwrap().unwrap();
    assert_eq!(stored.modified_at, 1_700_000_500_000);
}

#[test]
fn a_film_that_disappears_is_marked_rather_than_forgotten() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    library
        .database
        .positions()
        .save(
            film_id,
            Timestamp::from_seconds(2_400),
            Some(Timestamp::from_seconds(10_000)),
            false,
        )
        .unwrap();

    assert_eq!(
        library.database.films().mark_missing(&[film_id]).unwrap(),
        1
    );

    let film = library.database.films().by_id(film_id).unwrap().unwrap();
    assert!(film.is_missing());
    let position = library.database.positions().get(film_id).unwrap().unwrap();
    assert_eq!(position.position, Timestamp::from_seconds(2_400));

    // Marking it again keeps the date it first went missing.
    let first_seen_missing = film.missing_since;
    assert_eq!(
        library.database.films().mark_missing(&[film_id]).unwrap(),
        0
    );
    assert_eq!(
        library
            .database
            .films()
            .by_id(film_id)
            .unwrap()
            .unwrap()
            .missing_since,
        first_seen_missing
    );

    // The drive comes back.
    library.add_film(folder, "Heat");
    assert!(
        !library
            .database
            .films()
            .by_id(film_id)
            .unwrap()
            .unwrap()
            .is_missing()
    );
}

#[test]
fn a_missing_film_is_still_something_to_carry_on_with() {
    let library = Library::new();
    let folder = library.watch();
    let watching = library.add_film(folder, "Heat");
    let finished = library.add_film(folder, "Ronin");

    library
        .database
        .positions()
        .save(watching, Timestamp::from_seconds(60), None, false)
        .unwrap();
    library
        .database
        .positions()
        .save(finished, Timestamp::from_seconds(9_000), None, true)
        .unwrap();
    library.database.films().mark_missing(&[watching]).unwrap();

    let resumable = library.database.positions().resumable(10).unwrap();
    assert_eq!(resumable.len(), 1);
    assert_eq!(resumable[0].film.id, watching);
    assert!(resumable[0].film.is_missing());
}

#[test]
fn a_position_keeps_the_running_time_it_was_given_once() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    let positions = library.database.positions();

    positions
        .save(
            film_id,
            Timestamp::from_seconds(30),
            Some(Timestamp::from_seconds(120)),
            false,
        )
        .unwrap();
    // The player saves on a throttle and does not resend the running time.
    positions
        .save(film_id, Timestamp::from_seconds(60), None, false)
        .unwrap();

    let position = positions.get(film_id).unwrap().unwrap();
    assert_eq!(position.duration, Some(Timestamp::from_seconds(120)));
    assert!((position.progress().unwrap() - 0.5).abs() < f32::EPSILON);

    assert!(positions.clear(film_id).unwrap());
    assert!(positions.get(film_id).unwrap().is_none());
}

#[test]
fn cues_come_back_as_they_went_in() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    let track_id = library.add_track(film_id, "Heat");

    let written = vec![
        Cue {
            index: 1,
            start: Timestamp::from_millis(1_000),
            end: Timestamp::from_millis(4_000),
            text: "Don't let yourself get attached to anything.".to_owned(),
            position: None,
        },
        Cue {
            index: 2,
            start: Timestamp::from_millis(5_000),
            end: Timestamp::from_millis(6_500),
            text: "A line\nover two rows.".to_owned(),
            position: Some(CuePosition::TopCentre),
        },
    ];
    assert_eq!(
        library
            .database
            .tracks()
            .replace_cues(track_id, &written)
            .unwrap(),
        2
    );

    let read = library.database.tracks().cues(track_id).unwrap();
    assert_eq!(read, written);

    let track = library.database.tracks().for_film(film_id).unwrap();
    assert_eq!(track.len(), 1);
    assert_eq!(track[0].cue_count, 2);
    assert_eq!(track[0].language.as_deref(), Some("en"));
    assert_eq!(track[0].match_kind, TrackMatch::Exact);
    assert_eq!(track[0].encoding, "UTF-8");
}

#[test]
fn replacing_the_cues_leaves_none_of_the_old_ones_behind() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    let track_id = library.add_track(film_id, "Heat");

    library
        .database
        .tracks()
        .replace_cues(track_id, &cues(&["a helicopter over the freeway"]))
        .unwrap();

    // A corrected subtitle file replaces the old one.
    library
        .database
        .tracks()
        .replace_cues(track_id, &cues(&["a lighthouse over the harbour"]))
        .unwrap();

    let read = library.database.tracks().cues(track_id).unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].text, "a lighthouse over the harbour");
}

#[test]
fn removing_a_folder_takes_everything_under_it() {
    let library = Library::new();
    let folder = library.watch();
    let film_id =
        library.add_film_with_dialogue(folder, "Heat", &["a helicopter over the freeway"]);
    let track_id = library.database.tracks().for_film(film_id).unwrap()[0].id;

    assert!(library.database.folders().remove(folder).unwrap());

    assert!(library.database.films().list().unwrap().is_empty());
    assert!(
        library.database.tracks().cues(track_id).unwrap().is_empty(),
        "the cues under a folder must go with it"
    );
}

#[test]
fn a_pairing_made_by_hand_survives_a_rescan() {
    let library = Library::new();
    let folder = library.watch();
    let wrong = library.add_film(folder, "Heat");
    let right = library.add_film(folder, "Ronin");
    let track_id = library.add_track(wrong, "Heat");

    library.database.tracks().attach(track_id, right).unwrap();

    // The scanner comes round again and still thinks the file belongs to the
    // film whose name it shares.
    library.add_track(wrong, "Heat");

    let track = library
        .database
        .tracks()
        .by_path(&library.root.join("Heat.srt"))
        .unwrap()
        .unwrap();
    assert_eq!(track.film_id, right);
    assert_eq!(track.match_kind, TrackMatch::ByHand);
}

#[test]
fn fingerprints_say_what_needs_reading_again() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    library.add_track(film_id, "Heat");

    let films = library.database.films().fingerprints(folder).unwrap();
    assert_eq!(films.len(), 1);
    assert!(films[0].matches(4_000, 1_700_000_000_000));
    assert!(!films[0].matches(4_000, 1_700_000_000_001));

    let tracks = library.database.tracks().pairings(folder).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].film_id, film_id);
    assert_eq!(tracks[0].match_kind, TrackMatch::Exact);
    assert!(tracks[0].matches(60_000, 1_700_000_000_000));
    assert!(!tracks[0].matches(60_001, 1_700_000_000_000));
}

#[test]
fn a_track_moves_to_a_better_film_unless_it_was_attached_by_hand() {
    let library = Library::new();
    let folder = library.watch();
    let first = library.add_film(folder, "Solaris");
    let second = library.add_film(folder, "Solaris Part Two");
    let track_id = library.add_track(first, "Solaris");

    assert!(
        library
            .database
            .tracks()
            .repoint(track_id, second, TrackMatch::Approximate)
            .unwrap()
    );
    let moved = library.database.tracks().for_film(second).unwrap();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].match_kind, TrackMatch::Approximate);

    // Once someone has said where it belongs, a rescan may not disagree.
    library.database.tracks().attach(track_id, first).unwrap();
    assert!(
        !library
            .database
            .tracks()
            .repoint(track_id, second, TrackMatch::Exact)
            .unwrap()
    );
    assert_eq!(library.database.tracks().for_film(first).unwrap().len(), 1);
}

#[test]
fn a_batch_writes_its_tracks_and_their_cues_together() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    let path = library.root.join("Heat.en.srt");
    let lines = cues(&["a helicopter over the freeway", "the city at night"]);

    let track = NewTrack {
        film_id,
        path: &path,
        label: SubtitleLabel {
            language: Some("en"),
            forced: false,
            hearing_impaired: false,
        },
        origin: TrackOrigin::Sidecar,
        stream_number: 0,
        codec: "subrip",
        match_kind: TrackMatch::Exact,
        encoding: "UTF-8",
        size_bytes: 120,
        modified_at: 1_700_000_000_000,
    };

    let stored = library
        .database
        .tracks()
        .write_batch(&[(track, &lines)])
        .unwrap();

    assert_eq!(stored.len(), 1);
    assert!(stored[0].changed);
    assert_eq!(
        library.database.tracks().cues(stored[0].id).unwrap().len(),
        2
    );
    let recorded = library.database.tracks().for_film(film_id).unwrap();
    assert_eq!(recorded[0].cue_count, 2);

    assert!(
        library
            .database
            .tracks()
            .write_batch(&[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn many_films_are_recorded_in_one_go() {
    let library = Library::new();
    let folder = library.watch();
    let paths: Vec<_> = ["Heat", "Ronin", "Collateral"]
        .iter()
        .map(|name| library.root.join(format!("{name}.mkv")))
        .collect();
    let films: Vec<_> = paths
        .iter()
        .map(|path| NewFilm {
            folder_id: folder,
            path,
            title: "Whichever",
            year: Some(1_999),
            size_bytes: 4_000,
            modified_at: 1_700_000_000_000,
        })
        .collect();

    let first = library.database.films().upsert_many(&films).unwrap();
    assert_eq!(first.len(), 3);
    assert!(first.iter().all(|stored| stored.changed));

    // The same films again, unchanged, write nothing and keep their rows.
    let second = library.database.films().upsert_many(&films).unwrap();
    assert!(second.iter().all(|stored| !stored.changed));
    assert_eq!(
        first.iter().map(|stored| stored.id).collect::<Vec<_>>(),
        second.iter().map(|stored| stored.id).collect::<Vec<_>>()
    );

    assert!(
        library
            .database
            .films()
            .upsert_many(&[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn preferences_survive_a_restart() {
    let library = Library::new();
    let preferences = library.database.preferences();

    preferences.set("subtitle.size", "medium").unwrap();
    preferences.set("subtitle.size", "large").unwrap();
    preferences.set("playback.rewind_ms", "5000").unwrap();

    assert_eq!(
        library.reopen().preferences().get("subtitle.size").unwrap(),
        Some("large".to_owned())
    );
    assert_eq!(preferences.all().unwrap().len(), 2);
    assert!(preferences.get("nothing.set").unwrap().is_none());

    assert!(preferences.remove("subtitle.size").unwrap());
    assert!(!preferences.remove("subtitle.size").unwrap());
}

#[test]
fn a_path_that_is_not_text_is_refused_rather_than_mangled() {
    // Only reachable where the platform allows it; on Windows every path is
    // text, so there is nothing to refuse.
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::Path;

        let library = Library::new();
        let path = Path::new(OsStr::from_bytes(b"/films/\xff\xfe.mkv"));
        assert!(library.database.folders().add(path).is_err());
    }
}
