//! How long it takes to find out what a film carries.
//!
//! Two numbers matter. One container has to be answered for in under fifteen
//! milliseconds, since a scan does this once per film and a slow probe would
//! undo the walk it rides along with. And a probe has to cost what a header
//! costs rather than what a film costs, so the same measurement is taken
//! against a file with a real cluster of frames behind the header.
//!
//! The files are written to disk once at the start, because a probe that is not
//! opening and seeking a real file is not measuring what a scan does.

// A benchmark that cannot write its own files has nothing to measure, so it
// stops where it stands rather than reporting a number for something else.
#![allow(clippy::expect_used)]

use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::Criterion;
use subtext_container::{fixture, probe};
use tempfile::TempDir;

/// How many files the batch measurement covers, which is the size the library
/// targets are written against.
const FILMS: usize = 1_000;

/// A cluster of frames behind the header. Small next to a real film, and large
/// enough that reading it rather than seeking past it would show.
const PICTURE: usize = 16 << 20;

fn main() {
    let folder = TempDir::new().expect("a temporary folder to write films into");
    let plain = write(folder.path(), "plain.mkv", &tracks().bytes(), 0);
    let with_index = write(
        folder.path(),
        "indexed.mkv",
        &tracks()
            .with_seek_head()
            .with_declared_cluster(PICTURE as u64)
            .bytes(),
        PICTURE,
    );
    let films: Vec<PathBuf> = (0..FILMS)
        .map(|at| {
            write(
                folder.path(),
                &format!("film-{at}.mkv"),
                &tracks().with_seek_head().bytes(),
                0,
            )
        })
        .collect();

    let mut criterion = Criterion::default().configure_from_args();

    criterion.bench_function("probe one container", |bencher| {
        bencher.iter(|| black_box(probe(black_box(&plain)).expect("the file to be readable")));
    });

    // The target is fifteen milliseconds for one container whatever is behind
    // the header, which is the claim that nothing reads the picture.
    criterion.bench_function("probe a container with a film behind it", |bencher| {
        bencher.iter(|| black_box(probe(black_box(&with_index)).expect("the file to be readable")));
    });

    // One thread, so the number divides by however many cores the machine
    // running the scan turns out to have.
    criterion.bench_function("probe a thousand containers", |bencher| {
        bencher.iter(|| {
            for film in &films {
                black_box(probe(black_box(film)).expect("the file to be readable"));
            }
        });
    });

    criterion.final_summary();
}

/// A film of the shape most are: a picture track, a sound track, and subtitles
/// in three languages with one of them forced.
fn tracks() -> fixture::Container {
    fixture::Container::new(vec![
        fixture::Entry::video(1),
        fixture::Entry::audio(2),
        fixture::Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
        fixture::Entry::subtitle(4, "S_TEXT/ASS").in_language("jpn"),
        fixture::Entry::subtitle(5, "S_TEXT/UTF8")
            .in_language("eng")
            .forced(),
        fixture::Entry::subtitle(6, "S_HDMV/PGS").in_language("eng"),
    ])
}

fn write(folder: &Path, name: &str, header: &[u8], picture: usize) -> PathBuf {
    let path = folder.join(name);
    let mut bytes = header.to_vec();
    bytes.resize(bytes.len() + picture, 0);
    std::fs::write(&path, &bytes).expect("a film to be written");
    path
}
