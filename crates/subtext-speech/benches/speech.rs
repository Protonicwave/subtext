//! How long it takes to hear where a film's dialogue is.
//!
//! The target is fifteen seconds for a two hour film on eight cores, which is
//! the length somebody will wait through once, having asked for it, while the
//! film they asked about carries on playing.
//!
//! The film measured here is uncompressed, for the reason the fixture module
//! gives: writing an encoder to produce a compressed one would be a great deal
//! of code testing itself. That makes this a measurement of everything the
//! reading does except the codec, which is the part this crate is responsible
//! for and the part that would quietly get slower. What a real film adds on top
//! is Symphonia's decoder, measured against a real film rather than guessed at
//! here.
//!
//! Two measurements. A film long enough to be divided between cores is read
//! once, against the target, which is the only way to see the split working at
//! all. And a short one is read repeatedly, since one run of anything says less
//! than a distribution does.

// A benchmark that cannot write the film it is about to read has nothing to
// measure, so it stops where it stands rather than reporting a number for
// something else.
#![allow(clippy::cast_precision_loss, clippy::expect_used)]

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use criterion::Criterion;
use subtext_speech::fixture::Film;
use subtext_speech::speech_of;
use tempfile::TempDir;

/// How long the film the target is written against runs for, in milliseconds.
///
/// Twenty minutes rather than the two hours of the target. Uncompressed audio at
/// the rate a film is mixed at runs to a quarter of a gigabyte for this much,
/// and the whole of it is built in memory before it is written; six times as
/// much would measure the machine's patience rather than the reading. It is
/// still long enough to be divided between cores, which is the part of the work
/// a shorter film would not show at all. The cost is fixed per sample, so the
/// two hour figure is this one multiplied out, and it is reported that way
/// rather than claimed as a measurement.
const LONG_MS: u32 = 20 * 60 * 1_000;

/// The target, in seconds, and the length of film it is written against.
const TARGET_S: f64 = 15.0;
const TARGET_MS: u32 = 2 * 60 * 60 * 1_000;

/// A film short enough to read many times over.
const SHORT_MS: u32 = 30 * 1_000;

/// A line every few seconds, which is what a film has.
const LINE_MS: u32 = 2_000;
const GAP_MS: u32 = 1_500;

fn main() {
    let folder = TempDir::new().expect("a temporary folder to write films into");

    against_the_target(folder.path());
    repeatedly(folder.path());
}

/// The measurement the target is written against, done once.
fn against_the_target(folder: &Path) {
    let path = write(folder, "long.mkv", LONG_MS);
    let size = std::fs::metadata(&path)
        .expect("the film to be on disk")
        .len();

    let started = Instant::now();
    let speech = speech_of(&path).expect("the film to be readable");
    let taken = started.elapsed();

    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let scaled = taken.as_secs_f64() * f64::from(TARGET_MS) / f64::from(LONG_MS);
    println!(
        "read {} minutes of film ({} MB) in {:.2}s on {cores} cores, \
         {} of it speech, which is {scaled:.2}s for two hours against a target of {TARGET_S}s",
        LONG_MS / 60_000,
        size >> 20,
        taken.as_secs_f64(),
        speech.active(),
    );

    std::fs::remove_file(&path).expect("clearing the film away");
}

/// The same reading, repeatedly, over a film short enough to allow it.
fn repeatedly(folder: &Path) {
    let path = write(folder, "short.mkv", SHORT_MS);

    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("speech");
    group.sample_size(20);
    group.bench_function("30s of film", |bencher| {
        bencher.iter(|| black_box(speech_of(black_box(&path))));
    });
    group.finish();

    criterion.final_summary();
}

/// A film with somebody talking through most of it, on disk.
fn write(folder: &Path, name: &str, length_ms: u32) -> PathBuf {
    let mut film = Film::new(length_ms);
    let mut at = 0;
    while at + LINE_MS < length_ms {
        film = film.speaking(at, at + LINE_MS);
        at += LINE_MS + GAP_MS;
    }

    let path = folder.join(name);
    std::fs::write(&path, film.matroska()).expect("writing the film");
    path
}
