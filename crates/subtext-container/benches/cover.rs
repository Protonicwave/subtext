//! How long it takes to take a film's cover off it.
//!
//! The number that matters is forty milliseconds for one film, whatever size
//! the film is, since only the attachments are read and the picture behind them
//! is seeked over rather than read. Both halves are measured: asking whether
//! there is a cover, which is what a scan does once per film, and reading the
//! image, which is what happens once when a poster is drawn.
//!
//! The files are written to disk once at the start, because a read that is not
//! opening and seeking a real file is not measuring what the application does.

// A benchmark that cannot write its own files has nothing to measure, so it
// stops where it stands rather than reporting a number for something else.
#![allow(clippy::expect_used)]

use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::Criterion;
use subtext_container::{cover, cover_image, fixture};
use tempfile::TempDir;

/// A cluster of frames behind the header. Small next to a real film, and large
/// enough that reading it rather than seeking past it would show.
const PICTURE: usize = 64 << 20;

/// A cover of the size a real one is: a few hundred kilobytes of JPEG.
const IMAGE: usize = 320 << 10;

/// A font of the kind a film with a signs track carries, which the read has to
/// step over on its way to the picture.
const FONT: usize = 512 << 10;

fn main() {
    let folder = TempDir::new().expect("a temporary folder to write films into");
    let film = write(folder.path(), "film.mkv", &carrying().bytes(), PICTURE);

    let mut criterion = Criterion::default().configure_from_args();

    criterion.bench_function("ask a film whether it carries a cover", |bencher| {
        bencher.iter(|| black_box(cover(black_box(&film)).expect("the file to be readable")));
    });

    criterion.bench_function("take the cover off a film", |bencher| {
        bencher.iter(|| black_box(cover_image(black_box(&film)).expect("the file to be readable")));
    });

    criterion.final_summary();
}

/// A film of the shape most Matroska releases are, carrying a cover and the
/// font its signs track is set in.
fn carrying() -> fixture::Container {
    fixture::Container::new(vec![
        fixture::Entry::video(1),
        fixture::Entry::audio(2),
        fixture::Entry::subtitle(3, "S_TEXT/ASS").in_language("eng"),
    ])
    .with_seek_head()
    .with_declared_cluster(PICTURE as u64)
    .with_attachments(vec![
        fixture::Attachment::new("Roboto.ttf", &vec![0; FONT])
            .of_type("application/x-truetype-font"),
        fixture::Attachment::new("cover.jpg", &vec![0xFF; IMAGE]).of_type("image/jpeg"),
    ])
}

fn write(folder: &Path, name: &str, header: &[u8], picture: usize) -> PathBuf {
    let path = folder.join(name);
    let mut bytes = header.to_vec();
    bytes.resize(bytes.len() + picture, 0);
    std::fs::write(&path, &bytes).expect("a film to be written");
    path
}
