//! Reading real files, and the awkward ones.
//!
//! The corpus covers what a folder of films actually holds: soundtracks in both
//! containers, at the rates and channel counts films are mixed at, films with no
//! soundtrack, films in codecs this build has no decoder for, and files that
//! stop half way through because the download did. None of them may panic, and
//! the ones that can be read have to come back saying somebody is talking at the
//! moment somebody is talking.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use subtext_speech::fixture::Film;
use subtext_speech::{BIN_MS, Reading, Refusal, Signal, speech_of, speech_of_with};
use tempfile::TempDir;

/// How far a burst's edges may land from where the fixture wrote them.
///
/// Two bins, being twenty milliseconds. A bin is placed from the packet it
/// arrived in, so an edge can only move by the rounding at either end of one.
const SLACK: usize = 2;

/// A film written to a file, kept alive by its directory.
struct Written {
    _dir: TempDir,
    path: PathBuf,
}

fn write(name: &str, bytes: &[u8]) -> Written {
    let dir = TempDir::new().expect("a temporary directory");
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("writing the film");
    Written { _dir: dir, path }
}

/// Where a signal says somebody is talking, as runs of bins.
fn runs(signal: &Signal, length: usize) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut from = None;
    for bin in 0..length {
        match (signal.is_active(bin), from) {
            (true, None) => from = Some(bin),
            (false, Some(start)) => {
                runs.push((start, bin));
                from = None;
            }
            _ => {}
        }
    }
    if let Some(start) = from {
        runs.push((start, length));
    }
    runs
}

/// Checks that a film was heard talking where it was written talking.
fn assert_speech(path: &Path, film: &Film) {
    let signal = speech_of(path).expect("reading the film");
    let expected = film.spoken_bins(BIN_MS);
    let found = runs(&signal, signal.len());

    assert_eq!(
        found.len(),
        expected.len(),
        "expected {expected:?} but found {found:?}",
    );
    for ((from, to), (want_from, want_to)) in found.iter().zip(&expected) {
        assert!(
            from.abs_diff(*want_from) <= SLACK && to.abs_diff(*want_to) <= SLACK,
            "expected {expected:?} but found {found:?}",
        );
    }
}

#[test]
fn speech_is_found_in_a_matroska_film() {
    let film = Film::new(20_000)
        .speaking(3_000, 6_000)
        .speaking(11_500, 14_000);
    let written = write("a film.mkv", &film.matroska());
    assert_speech(&written.path, &film);
}

#[test]
fn speech_is_found_in_an_mp4_film() {
    let film = Film::new(20_000)
        .speaking(3_000, 6_000)
        .speaking(11_500, 14_000);
    let written = write("a film.mp4", &film.mp4());
    assert_speech(&written.path, &film);
}

#[test]
fn the_rate_and_the_channels_a_film_was_mixed_at_change_nothing() {
    for (rate, channels) in [(48_000, 2), (44_100, 2), (22_050, 1), (16_000, 1)] {
        let film = Film::new(12_000)
            .recorded(rate, channels)
            .speaking(4_000, 8_000);
        let written = write("a film.mkv", &film.matroska());
        assert_speech(&written.path, &film);
    }
}

#[test]
fn a_film_with_nobody_talking_says_so() {
    let film = Film::new(10_000);
    let written = write("a silent film.mkv", &film.matroska());
    let signal = speech_of(&written.path).expect("reading the film");
    assert_eq!(signal.active(), 0);
}

#[test]
fn a_film_with_no_soundtrack_is_refused_as_such() {
    let film = Film::new(10_000).without_audio();
    let written = write("a film.mkv", &film.matroska());
    assert_eq!(speech_of(&written.path), Err(Refusal::NoAudio));
}

#[test]
fn a_codec_this_build_cannot_read_is_refused_by_name() {
    // What a disc rip carries. Each of these has to be named where somebody can
    // read it, rather than producing a bar that ends in an empty answer.
    for (codec, name) in [
        ("A_AC3", "AC-3"),
        ("A_EAC3", "E-AC-3"),
        ("A_DTS", "DTS"),
        ("A_TRUEHD", "TrueHD"),
    ] {
        let film = Film::new(10_000).speaking(1_000, 5_000).claiming(codec);
        let written = write("a disc rip.mkv", &film.matroska());
        assert_eq!(
            speech_of(&written.path),
            Err(Refusal::Codec {
                name: Some(name.to_owned())
            }),
            "{codec}",
        );
    }
}

#[test]
fn a_film_that_stops_half_way_through_is_a_shorter_film() {
    let film = Film::new(20_000)
        .speaking(2_000, 5_000)
        .speaking(15_000, 18_000);
    let bytes = film.matroska();
    let written = write("half a film.mkv", &bytes[..bytes.len() / 2]);

    // Either the header survived the truncation and the film reads as far as it
    // goes, or it did not and the file is refused. Neither may panic, and
    // neither may claim dialogue that is not there.
    if let Ok(signal) = speech_of(&written.path) {
        assert!(signal.len() <= 20_000 / BIN_MS as usize + 1);
        let found = runs(&signal, signal.len());
        assert!(
            found
                .first()
                .is_none_or(|(from, _)| from.abs_diff(200) <= SLACK)
        );
    }
}

#[test]
fn a_file_that_is_not_a_film_is_refused_rather_than_read() {
    let written = write("not a film.mkv", &[0x42; 4_096]);
    assert!(matches!(
        speech_of(&written.path),
        Err(Refusal::Unreadable(_))
    ));
}

#[test]
fn a_film_that_is_not_there_is_refused() {
    assert!(matches!(
        speech_of(Path::new("no such film.mkv")),
        Err(Refusal::Unreadable(_))
    ));
}

#[test]
fn a_film_long_enough_to_divide_between_cores_is_read_the_same_way() {
    // Past the length at which the reading is split, so that every run has to
    // seek to its own start, settle, and place its bins against the film's clock
    // rather than against its own. Mixed at a rate that keeps the file small
    // enough to write in a test.
    let film = Film::new(11 * 60 * 1_000)
        .recorded(8_000, 1)
        .speaking(30_000, 45_000)
        .speaking(4 * 60 * 1_000, 4 * 60 * 1_000 + 20_000)
        .speaking(9 * 60 * 1_000, 9 * 60 * 1_000 + 30_000);
    let written = write("a long film.mkv", &film.matroska());
    assert_speech(&written.path, &film);
}

#[test]
fn progress_climbs_from_nothing_to_all_of_it() {
    let film = Film::new(20_000).speaking(3_000, 6_000);
    let written = write("a film.mkv", &film.matroska());

    let reported = Mutex::new(Vec::new());
    let watch = |fraction: f32| {
        reported.lock().unwrap().push(fraction);
        Reading::Continue
    };
    speech_of_with(&written.path, &watch).expect("reading the film");

    let reported = reported.lock().unwrap();
    assert!(reported.len() > 1);
    assert!(reported.windows(2).all(|pair| pair[1] >= pair[0]));
    assert!(reported.first().is_some_and(|first| *first < 0.5));
    assert!(reported.last().is_some_and(|last| *last > 0.9));
}

#[test]
fn a_reading_stops_when_it_is_asked_to() {
    let film = Film::new(60_000).speaking(3_000, 6_000);
    let written = write("a film.mkv", &film.matroska());

    let asked = AtomicUsize::new(0);
    let watch = |_fraction: f32| {
        if asked.fetch_add(1, Ordering::Relaxed) >= 2 {
            Reading::Stop
        } else {
            Reading::Continue
        }
    };

    assert_eq!(speech_of_with(&written.path, &watch), Err(Refusal::Stopped));
    // Stopped where it was asked to rather than after finishing the film and
    // reporting the fact.
    assert!(asked.load(Ordering::Relaxed) < 10);
}
