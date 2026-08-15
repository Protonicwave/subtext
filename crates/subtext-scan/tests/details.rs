//! What a scan records about the films themselves.
//!
//! Every fact a film sheet shows is read once, while the film is open for the
//! dialogue inside it, and held on the row from then on. These are the tests
//! that say so, and that say what a film which cannot be read reports instead.

// A test that cannot get at the film it is about to check has nothing to say,
// so it stops rather than passing quietly.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use subtext_container::fixture::Entry;
use subtext_core::Timestamp;
use subtext_index::FilmRecord;

use crate::common::Fixture;

/// The tracks a film of the ordinary shape carries.
fn tracks() -> Vec<Entry> {
    vec![
        Entry::video(1)
            .sized(1920, 1080)
            .at_bit_depth(10)
            .at_frame_rate(23.976),
        Entry::audio(2).in_language("eng").with_channels(6),
        Entry::audio(3).in_language("fre").with_channels(2),
        Entry::subtitle(4, "S_TEXT/UTF8").in_language("eng"),
    ]
}

fn film(fixture: &Fixture, relative: &str) -> FilmRecord {
    fixture
        .database()
        .films()
        .by_id(fixture.film_id(relative))
        .unwrap()
        .expect("the film to be in the library")
}

#[test]
fn a_film_is_described_by_the_scan_that_finds_it() {
    let fixture = Fixture::new();
    fixture.matroska_running_for("Heat.1995.mkv", tracks(), 10_260_000);
    fixture.scan();

    let heat = film(&fixture, "Heat.1995.mkv");
    assert_eq!(heat.container.as_deref(), Some("Matroska"));
    assert_eq!(heat.duration, Some(Timestamp::from_millis(10_260_000)));

    let picture = heat.video.expect("a picture");
    assert_eq!(picture.codec, "V_MPEG4/ISO/AVC");
    assert_eq!(picture.width, Some(1_920));
    assert_eq!(picture.height, Some(1_080));
    assert_eq!(picture.bit_depth, Some(10));
    assert!((picture.frame_rate.unwrap_or_default() - 23.976).abs() < 0.001);
}

#[test]
fn every_sound_track_a_film_carries_is_recorded() {
    let fixture = Fixture::new();
    fixture.matroska("Heat.1995.mkv", tracks());
    fixture.scan();

    let sound = fixture
        .database()
        .details()
        .audio(fixture.film_id("Heat.1995.mkv"))
        .unwrap();

    assert_eq!(sound.len(), 2);
    assert_eq!(sound[0].stream_number, 2);
    assert_eq!(sound[0].codec, "A_AAC");
    assert_eq!(sound[0].channels, Some(6));
    assert_eq!(sound[0].language.as_deref(), Some("en"));
    assert_eq!(sound[1].stream_number, 3);
    assert_eq!(sound[1].channels, Some(2));
    assert_eq!(sound[1].language.as_deref(), Some("fr"));
}

/// A file this application does not parse, which is most of what is in an
/// ordinary library.
#[test]
fn a_film_that_is_not_matroska_reports_its_container_and_nothing_else() {
    let fixture = Fixture::new();
    fixture.film("Heat.1995.mp4");
    fixture.scan();

    let heat = film(&fixture, "Heat.1995.mp4");
    assert_eq!(heat.container.as_deref(), Some("MP4"));
    // Not zero, and not a guess. Nothing read the file, so nothing is claimed
    // about what is in it.
    assert_eq!(heat.video, None);
    assert_eq!(heat.duration, None);
    assert!(
        fixture
            .database()
            .details()
            .audio(heat.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_film_with_no_sound_at_all_says_nothing_rather_than_nought() {
    let fixture = Fixture::new();
    fixture.matroska(
        "Silent.1928.mkv",
        vec![Entry::video(1).sized(1440, 1080), Entry::audio(2)],
    );
    fixture.scan();

    let sound = fixture
        .database()
        .details()
        .audio(fixture.film_id("Silent.1928.mkv"))
        .unwrap();

    // The sound track is there and says nothing about how many channels it
    // carries, which is different from carrying none.
    assert_eq!(sound.len(), 1);
    assert_eq!(sound[0].channels, None);

    let silent = film(&fixture, "Silent.1928.mkv");
    let picture = silent.video.expect("a picture");
    assert_eq!(picture.bit_depth, None);
    assert_eq!(picture.frame_rate, None);
}

#[test]
fn describing_a_film_costs_a_rescan_nothing() {
    let fixture = Fixture::new();
    fixture.matroska_running_for("Heat.1995.mkv", tracks(), 10_260_000);
    fixture.subtitle("Heat.1995.srt", &["the action is the juice"]);

    assert_eq!(fixture.scan().films_probed, 1);

    // Nothing has moved, so nothing is opened and nothing is described again.
    let again = fixture.scan();
    assert_eq!(again.films_probed, 0);
    assert_eq!(again.subtitles_read, 0);

    // And what was recorded is still there.
    assert_eq!(
        film(&fixture, "Heat.1995.mkv").duration,
        Some(Timestamp::from_millis(10_260_000))
    );
}

/// A film re-encoded in place, which is the case a description has to be
/// replaced rather than added to.
#[test]
fn a_film_that_has_been_replaced_is_described_again() {
    let fixture = Fixture::new();
    fixture.matroska("Heat.1995.mkv", tracks());
    fixture.scan();

    fixture.matroska(
        "Heat.1995.mkv",
        vec![
            Entry::video(1).sized(3840, 2160).at_bit_depth(10),
            Entry::audio(2).in_language("eng").with_channels(8),
        ],
    );
    assert_eq!(fixture.scan().films_probed, 1);

    let picture = film(&fixture, "Heat.1995.mkv").video.expect("a picture");
    assert_eq!(picture.width, Some(3_840));

    let sound = fixture
        .database()
        .details()
        .audio(fixture.film_id("Heat.1995.mkv"))
        .unwrap();
    assert_eq!(sound.len(), 1);
    assert_eq!(sound[0].channels, Some(8));
}

/// The running time is the one fact two things can supply, and the player's
/// answer is not thrown away by a container that never stated one.
#[test]
fn a_running_time_the_player_found_survives_a_rescan() {
    let fixture = Fixture::new();
    fixture.film("Heat.1995.mp4");
    fixture.scan();

    let heat = fixture.film_id("Heat.1995.mp4");
    fixture
        .database()
        .films()
        .set_duration(heat, Timestamp::from_millis(10_260_000))
        .unwrap();
    fixture.scan();

    assert_eq!(
        film(&fixture, "Heat.1995.mp4").duration,
        Some(Timestamp::from_millis(10_260_000))
    );
}
