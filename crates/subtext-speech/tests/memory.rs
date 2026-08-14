//! What a film costs to read, however long the film is.
//!
//! The claim this crate makes is that a three hour film costs what a three
//! minute one does, because the audio is measured as it goes past and never
//! held. It is the property that lets an alignment run while a film is playing
//! rather than competing with it for memory, and it is the sort of claim that
//! quietly stops being true, so it is measured rather than asserted in a
//! comment.
//!
//! Both films are written before anything is measured. Building one costs as
//! much memory as the film is long, and a measurement taken with that still
//! settling would be a measurement of the writing.
//!
//! What is compared is the rise from just before a read to the highest point
//! during it, taken through the same progress hook a caller would use, since
//! what matters is the high water mark while the work is happening and not what
//! is left over afterwards.
//!
//! One test, and the only one in this file, because it watches the memory of the
//! whole process and another test running beside it would be part of the answer.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use memory_stats::memory_stats;
use subtext_speech::fixture::Film;
use subtext_speech::{Reading, speech_of_with};
use tempfile::TempDir;

/// How much more memory the longer film may cost, in bytes.
///
/// Four megabytes, against a soundtrack more than forty megabytes longer. The
/// bound is loose because a process's memory is measured in pages the operating
/// system has decided to keep rather than in bytes anybody asked for, and it is
/// still a small fraction of the difference it would show if a film were held.
const ALLOWANCE: usize = 4 * 1_024 * 1_024;

fn held() -> usize {
    memory_stats().map_or(0, |stats| stats.physical_mem)
}

/// A film on disk, and how long it runs for.
fn write(dir: &TempDir, name: &str, length_ms: u32, talking: (u32, u32)) -> PathBuf {
    let film = Film::new(length_ms).speaking(talking.0, talking.1);
    let path = dir.path().join(name);
    std::fs::write(&path, film.matroska()).expect("writing the film");
    path
}

/// How much further the process's memory climbs while a film is read.
fn cost_of_reading(path: &Path) -> usize {
    let before = held();
    let peak = AtomicUsize::new(before);
    let watch = |_fraction: f32| {
        peak.fetch_max(held(), Ordering::Relaxed);
        Reading::Continue
    };
    speech_of_with(path, &watch).expect("reading the film");

    peak.load(Ordering::Relaxed).saturating_sub(before)
}

#[test]
fn a_long_film_costs_what_a_short_one_does() {
    let dir = TempDir::new().expect("a temporary directory");
    let short = write(&dir, "a short film.mkv", 60_000, (10_000, 40_000));
    // Four and a half times the film, which is forty two megabytes more audio to
    // decode. None of it may still be there at the end of a packet.
    let long = write(&dir, "a long film.mkv", 280_000, (10_000, 260_000));

    let brief = cost_of_reading(&short);
    let lengthy = cost_of_reading(&long);

    assert!(
        lengthy < brief + ALLOWANCE,
        "reading a film of 280s climbed by {lengthy} bytes against {brief} for one of 60s",
    );
}
