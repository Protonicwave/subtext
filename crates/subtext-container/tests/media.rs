//! Reading what a film is, against the shapes a real one takes.
//!
//! The corpus covers what a folder of films actually holds: one picture track
//! and several sound tracks, films with no sound at all, headers that say very
//! little about themselves, and files that stop half way through because the
//! download did. None of them may panic, and none of them may report a fact the
//! file never stated.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::Cursor;

use subtext_container::fixture::{Container, Entry};
use subtext_container::{MediaStreams, media, media_in};
use tempfile::TempDir;

fn read(container: &Container) -> MediaStreams {
    media_in(Cursor::new(container.bytes()))
}

/// A film of the shape most are: one picture track, its own language of sound,
/// a commentary, and subtitles.
fn film() -> Container {
    Container::new(vec![
        Entry::video(1)
            .sized(1920, 1080)
            .at_bit_depth(8)
            .at_frame_rate(23.976),
        Entry::audio(2).in_language("eng").with_channels(6),
        Entry::audio(3).in_language("fre").with_channels(2),
        Entry::subtitle(4, "S_TEXT/UTF8").in_language("eng"),
    ])
    .running_for(10_260_000)
}

#[test]
fn reads_what_the_picture_is() {
    let picture = read(&film().with_seek_head()).video.expect("a picture");

    assert_eq!(picture.codec, "V_MPEG4/ISO/AVC");
    assert_eq!(picture.width, Some(1920));
    assert_eq!(picture.height, Some(1080));
    assert_eq!(picture.bit_depth, Some(8));
    assert!((picture.frame_rate.unwrap_or_default() - 23.976).abs() < 0.001);
}

#[test]
fn reads_every_sound_track_the_film_carries() {
    let sound = read(&film().with_seek_head()).audio;

    assert_eq!(sound.len(), 2);
    assert_eq!(sound[0].number, 2);
    assert_eq!(sound[0].codec, "A_AAC");
    assert_eq!(sound[0].channels, Some(6));
    assert_eq!(sound[0].language, Some("en"));
    assert!(sound[0].default);

    assert_eq!(sound[1].number, 3);
    assert_eq!(sound[1].channels, Some(2));
    assert_eq!(sound[1].language, Some("fr"));
}

#[test]
fn reads_how_long_the_film_runs() {
    assert_eq!(read(&film()).duration_ms, Some(10_260_000));

    // A file counting its timestamps in something other than milliseconds,
    // which is legal and rare, and still runs for the same length of time.
    let unusual = film().with_timestamp_scale(100_000);
    assert_eq!(read(&unusual).duration_ms, Some(10_260_000));
}

#[test]
fn sound_tracks_come_back_in_the_order_the_file_numbers_them() {
    let film = Container::new(vec![
        Entry::audio(9).in_language("fre"),
        Entry::audio(2).in_language("eng"),
    ]);

    let numbers: Vec<u64> = read(&film).audio.iter().map(|track| track.number).collect();
    assert_eq!(numbers, [2, 9]);
}

#[test]
fn a_film_with_no_sound_says_so_rather_than_guessing() {
    let silent = read(&Container::new(vec![Entry::video(1).sized(1920, 800)]));

    assert!(silent.audio.is_empty());
    assert!(silent.video.is_some());
    assert_eq!(silent.duration_ms, None);
}

#[test]
fn a_header_that_says_little_reports_little() {
    // A track entry with a codec and nothing else, which plenty of files
    // written by hand carry.
    let sparse = read(&Container::new(vec![Entry::video(1), Entry::audio(2)]));

    let picture = sparse.video.expect("a picture");
    assert_eq!(picture.codec, "V_MPEG4/ISO/AVC");
    assert_eq!(picture.width, None);
    assert_eq!(picture.height, None);
    assert_eq!(picture.bit_depth, None);
    assert_eq!(picture.frame_rate, None);
    assert_eq!(sparse.audio[0].channels, None);
}

#[test]
fn a_film_that_says_nothing_at_all_is_empty() {
    assert!(read(&Container::new(Vec::new())).is_empty());
}

#[test]
fn a_file_with_no_index_is_walked_instead() {
    assert_eq!(read(&film()), read(&film().with_seek_head()));
}

#[test]
fn a_file_that_stops_part_way_through_reads_as_what_was_there() {
    let whole = film().with_seek_head().bytes();

    for at in 0..whole.len() {
        let found = media_in(Cursor::new(whole[..at].to_vec()));
        // Either the header was complete before the cut or it was not. What
        // matters is that neither answer is a panic, and that no sound track is
        // invented out of the bytes that happened to be there.
        assert!(found.audio.len() <= 2, "{at} bytes read as too much sound");
    }
}

#[test]
fn nothing_in_a_damaged_file_is_taken_as_a_fact() {
    let whole = film().with_seek_head().bytes();

    for at in 0..whole.len() {
        for damage in [0x00, 0x01, 0xFF, 0x7F] {
            let mut damaged = whole.clone();
            damaged[at] = damage;
            let found = media_in(Cursor::new(damaged));
            assert!(found.audio.len() <= 2, "byte {at} set to {damage:#x}");
        }
    }
}

#[test]
fn something_that_is_not_a_container_at_all_reads_as_nothing() {
    // An MP4, which this does not parse and must not pretend to.
    let mp4 = b"\x00\x00\x00\x20ftypisom".to_vec();
    assert!(media_in(Cursor::new(mp4)).is_empty());

    assert!(media_in(Cursor::new(b"not a film at all".to_vec())).is_empty());
    assert!(media_in(Cursor::new(Vec::new())).is_empty());
}

#[test]
fn a_film_on_disk_is_read_the_same_way() {
    let folder = TempDir::new().unwrap();
    let path = folder.path().join("Heat.1995.mkv");
    std::fs::write(&path, film().with_seek_head().bytes()).unwrap();

    let found = media(&path).expect("a film that is there to be readable");
    assert_eq!(found.duration_ms, Some(10_260_000));
    assert_eq!(found.audio.len(), 2);

    // A path with nothing at it is the one thing this reports as an error,
    // since it is the only one a scan can do anything about.
    assert!(media(&folder.path().join("gone.mkv")).is_err());
}
