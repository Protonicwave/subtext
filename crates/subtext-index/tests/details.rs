//! What each film's file turned out to be, written and read back.

// A test that cannot get at the film it is about to check has nothing to say,
// so it stops rather than passing quietly.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use subtext_core::Timestamp;
use subtext_index::{AudioDetails, MediaDetails, VideoDetails};

use crate::common::Library;

fn sound(number: u64, codec: &str, channels: Option<u8>, language: Option<&str>) -> AudioDetails {
    AudioDetails {
        stream_number: number,
        codec: codec.to_owned(),
        channels,
        language: language.map(ToOwned::to_owned),
        default: number == 1,
    }
}

/// What a probe of an ordinary film comes back with.
fn described() -> MediaDetails {
    MediaDetails {
        container: "Matroska".to_owned(),
        duration: Some(Timestamp::from_millis(10_260_000)),
        video: Some(VideoDetails {
            codec: "V_MPEG4/ISO/AVC".to_owned(),
            width: Some(1_920),
            height: Some(1_080),
            bit_depth: Some(8),
            frame_rate: Some(23.976),
        }),
        audio: vec![
            sound(1, "A_AC3", Some(6), Some("en")),
            sound(2, "A_AAC", Some(2), Some("fr")),
        ],
    }
}

#[test]
fn what_a_film_is_survives_a_restart() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");

    library
        .database
        .details()
        .record(&[(film_id, &described())])
        .unwrap();

    let reopened = library.reopen();
    let film = reopened.films().by_id(film_id).unwrap().expect("the film");

    assert_eq!(film.container.as_deref(), Some("Matroska"));
    assert_eq!(film.duration, Some(Timestamp::from_millis(10_260_000)));

    let picture = film.video.expect("a picture");
    assert_eq!(picture.codec, "V_MPEG4/ISO/AVC");
    assert_eq!(picture.width, Some(1_920));
    assert_eq!(picture.height, Some(1_080));
    assert_eq!(picture.bit_depth, Some(8));
    assert!((picture.frame_rate.unwrap_or_default() - 23.976).abs() < 0.001);

    let audio = reopened.details().audio(film_id).unwrap();
    assert_eq!(audio.len(), 2);
    assert_eq!(audio[0].codec, "A_AC3");
    assert_eq!(audio[0].channels, Some(6));
    assert_eq!(audio[0].language.as_deref(), Some("en"));
    assert!(audio[0].default);
    assert_eq!(audio[1].language.as_deref(), Some("fr"));
    assert!(!audio[1].default);
}

#[test]
fn a_film_nothing_has_looked_at_says_so() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");

    let film = library
        .database
        .films()
        .by_id(film_id)
        .unwrap()
        .expect("the film");
    assert_eq!(film.container, None);
    assert_eq!(film.video, None);

    // Which is what puts it in the way of the next scan.
    assert_eq!(
        library.database.films().undescribed(folder).unwrap(),
        [film_id]
    );

    library
        .database
        .details()
        .record(&[(film_id, &described())])
        .unwrap();
    assert!(
        library
            .database
            .films()
            .undescribed(folder)
            .unwrap()
            .is_empty()
    );
}

/// A file this application does not parse, which knows what container it is in
/// and nothing else.
#[test]
fn a_film_with_nothing_to_say_records_only_what_is_known() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");

    library
        .database
        .details()
        .record(&[(
            film_id,
            &MediaDetails {
                container: "MP4".to_owned(),
                ..MediaDetails::default()
            },
        )])
        .unwrap();

    let film = library
        .database
        .films()
        .by_id(film_id)
        .unwrap()
        .expect("the film");
    assert_eq!(film.container.as_deref(), Some("MP4"));
    assert_eq!(film.video, None);
    // Not zero, and not a guess. The file did not say.
    assert_eq!(film.duration, None);
    assert!(
        library
            .database
            .details()
            .audio(film_id)
            .unwrap()
            .is_empty()
    );

    // It has been looked at, so it is not asked about again.
    assert!(
        library
            .database
            .films()
            .undescribed(folder)
            .unwrap()
            .is_empty()
    );
}

/// A film re-encoded with different sound, which is what replacing rather than
/// merging is for.
#[test]
fn describing_a_film_again_replaces_what_it_carries() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");
    let details = library.database.details();

    details.record(&[(film_id, &described())]).unwrap();
    details
        .record(&[(
            film_id,
            &MediaDetails {
                audio: vec![sound(1, "A_OPUS", Some(2), Some("en"))],
                ..described()
            },
        )])
        .unwrap();

    let audio = details.audio(film_id).unwrap();
    assert_eq!(audio.len(), 1);
    assert_eq!(audio[0].codec, "A_OPUS");
}

/// The running time the player measured, which the container may not state and
/// which describing a film again must not take away.
#[test]
fn a_running_time_already_known_is_kept_where_the_container_says_nothing() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");

    library
        .database
        .films()
        .set_duration(film_id, Timestamp::from_millis(10_260_000))
        .unwrap();
    library
        .database
        .details()
        .record(&[(
            film_id,
            &MediaDetails {
                container: "MP4".to_owned(),
                ..MediaDetails::default()
            },
        )])
        .unwrap();

    let film = library
        .database
        .films()
        .by_id(film_id)
        .unwrap()
        .expect("the film");
    assert_eq!(film.duration, Some(Timestamp::from_millis(10_260_000)));
}

#[test]
fn the_whole_library_is_read_in_one_go() {
    let library = Library::new();
    let folder = library.watch();
    let heat = library.add_film(folder, "Heat");
    let ronin = library.add_film(folder, "Ronin");
    library.add_film(folder, "Silent");

    library
        .database
        .details()
        .record(&[(heat, &described()), (ronin, &described())])
        .unwrap();

    let all = library.database.details().all_audio().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get(&heat).map(Vec::len), Some(2));
    assert_eq!(all.get(&ronin).map(Vec::len), Some(2));
}

/// Removing a film takes its sound with it, since nothing else refers to it.
#[test]
fn forgetting_a_film_forgets_what_it_carried() {
    let library = Library::new();
    let folder = library.watch();
    let film_id = library.add_film(folder, "Heat");

    library
        .database
        .details()
        .record(&[(film_id, &described())])
        .unwrap();
    library.database.films().remove(film_id).unwrap();

    assert!(library.database.details().all_audio().unwrap().is_empty());
}
