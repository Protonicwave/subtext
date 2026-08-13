//! How long it takes to read the dialogue out of a film.
//!
//! The target is four seconds for a four gigabyte film, which is the size a
//! high bitrate feature is and the size that decides whether a first scan of a
//! library of them is something anyone waits through.
//!
//! Two things are measured. A film large enough to be worth timing is written
//! out and read once, against that target. And a smaller one is read repeatedly,
//! since a single run of anything says less than a distribution does.
//!
//! The large film is written to disk rather than made up in memory, because
//! what is being measured is a reader stepping over frames it does not want,
//! and a reader handed those frames for nothing is not doing the work.

// A benchmark that cannot write the film it is about to read has nothing to
// measure, so it stops where it stands rather than reporting a number for
// something else.
#![allow(clippy::cast_precision_loss, clippy::expect_used)]

use std::hint::black_box;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Instant;

use criterion::Criterion;
use subtext_container::fixture::{Container, Entry, Line};
use subtext_container::{extract, subtitles_of};
use tempfile::TempDir;

/// How large the film the target is written against is.
const FOUR_GIGABYTES: u64 = 4 << 30;

/// A frame of picture, and how many of them fall between one group of lines
/// and the next. Together they decide how many blocks a film holds, which is
/// what the reading actually costs.
const FRAME: usize = 24 << 10;
const FRAMES_PER_CLUSTER: usize = 64;

/// A line of dialogue every two seconds, which is about what a film has.
const EVERY: u64 = 2_000;

fn main() {
    let folder = TempDir::new().expect("a temporary folder to write films into");

    against_the_target(folder.path());
    repeatedly(folder.path());
}

/// The measurement the target in the plan is written against, done once.
fn against_the_target(folder: &Path) {
    let path = write(folder, "large.mkv", FOUR_GIGABYTES);
    let size = std::fs::metadata(&path)
        .expect("the film to be on disk")
        .len();

    let started = Instant::now();
    let tracks = extract(&path).expect("the film to be readable");
    let taken = started.elapsed();

    let cues: usize = tracks.iter().map(|track| track.cues.len()).sum();
    println!(
        "extract: {cues} lines from a film of {:.1}GB in {:.2}s (target 4s)",
        size as f64 / (1u64 << 30) as f64,
        taken.as_secs_f64()
    );

    std::fs::remove_file(&path).expect("the film to be removable");
}

/// A film small enough to read many times, so that the cost per byte stepped
/// over has a distribution behind it rather than one run.
fn repeatedly(folder: &Path) {
    let path = write(folder, "small.mkv", 256 << 20);
    let film = std::fs::read(&path).expect("the film to be readable");

    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("extract");
    group.sample_size(20);

    // From a file, which is what a scan does.
    group.bench_function("a film of 256MB", |bencher| {
        bencher.iter(|| black_box(extract(black_box(&path)).expect("the film to be readable")));
    });

    // And from memory, which takes the filesystem out of the number and leaves
    // what the walk itself costs.
    group.bench_function("the same film already in memory", |bencher| {
        bencher.iter(|| black_box(subtitles_of(std::io::Cursor::new(black_box(&film)))));
    });

    group.finish();
    criterion.final_summary();
}

/// Writes a film of roughly the given size, a cluster at a time.
fn write(folder: &Path, name: &str, size: u64) -> PathBuf {
    let per_cluster = (FRAME * FRAMES_PER_CLUSTER) as u64;
    let clusters = usize::try_from(size / per_cluster).unwrap_or(usize::MAX);
    let lines = (0..clusters * 8)
        .map(|at| {
            let start = at as u64 * EVERY;
            Line::new(3, start, start + 1_800, &format!("Line {at}"))
        })
        .collect();

    let path = folder.join(name);
    let file = std::fs::File::create(&path).expect("a film to write");
    let mut out = BufWriter::new(file);

    Container::new(vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
    ])
    .with_picture(1, FRAMES_PER_CLUSTER, FRAME)
    .with_dialogue(lines)
    .write_to(&mut out)
    .expect("a film to be written");

    path
}
