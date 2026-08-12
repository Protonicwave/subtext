//! Which subtitle track a film is watched with, kept between sittings.

// A test that cannot get at the film it is about to check has nothing to say,
// so it stops rather than passing quietly.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use subtext_index::TrackChoice;

use crate::common::Library;

/// The film as the database now holds it, or a failed test.
fn choice_of(library: &Library, film_id: i64) -> TrackChoice {
    library
        .database
        .films()
        .by_id(film_id)
        .unwrap()
        .expect("the film should still be in the library")
        .choice
}

#[test]
fn a_film_nobody_has_chosen_for_says_nothing() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    library.add_track(film, "Heat");

    assert_eq!(choice_of(&library, film), TrackChoice::Unset);
}

#[test]
fn a_chosen_track_is_still_chosen_after_a_restart() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    library.add_track(film, "Heat.en");
    let wanted = library.add_track(film, "Heat.en.forced");

    assert!(
        library
            .database
            .films()
            .set_choice(film, TrackChoice::Track(wanted))
            .unwrap()
    );

    let reopened = library.reopen();
    let film = reopened.films().by_id(film).unwrap().unwrap();
    assert_eq!(film.choice, TrackChoice::Track(wanted));
}

/// Turning subtitles off is a decision, and one that has to outlast the sitting
/// it was made in. A film that reads back as unset would open with a track
/// again, which is the opposite of what was asked for.
#[test]
fn subtitles_turned_off_stay_off() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let track = library.add_track(film, "Heat");

    library
        .database
        .films()
        .set_choice(film, TrackChoice::Track(track))
        .unwrap();
    library
        .database
        .films()
        .set_choice(film, TrackChoice::Off)
        .unwrap();

    let reopened = library.reopen();
    let film = reopened.films().by_id(film).unwrap().unwrap();
    assert_eq!(film.choice, TrackChoice::Off);
    assert!(film.choice.is_off());
    assert_eq!(film.choice.track_id(), None);
}

#[test]
fn removing_the_chosen_track_leaves_the_film_choosing_again() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let track = library.add_track(film, "Heat");
    library
        .database
        .films()
        .set_choice(film, TrackChoice::Track(track))
        .unwrap();

    assert!(library.database.tracks().remove(track).unwrap());

    // Cleared rather than orphaned, so the film picks a track by the rule
    // instead of asking for one that is no longer there.
    assert_eq!(choice_of(&library, film), TrackChoice::Unset);
}

/// A rescan writes films and tracks again and must not undo a choice while it
/// is at it. This is the property that makes the setting worth keeping at all.
#[test]
fn rescanning_leaves_a_choice_alone() {
    let library = Library::new();
    let folder = library.watch();
    let film = library.add_film(folder, "Heat");
    let track = library.add_track(film, "Heat");
    library
        .database
        .films()
        .set_choice(film, TrackChoice::Track(track))
        .unwrap();

    library.add_film(folder, "Heat");
    library.add_track(film, "Heat");

    assert_eq!(choice_of(&library, film), TrackChoice::Track(track));
}

#[test]
fn choosing_for_a_film_that_is_gone_changes_nothing() {
    let library = Library::new();

    assert!(
        !library
            .database
            .films()
            .set_choice(404, TrackChoice::Off)
            .unwrap()
    );
}
