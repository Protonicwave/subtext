//! What watching a folder promises once the application is running.

#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use subtext_scan::{FolderWatcher, Silent};

use crate::common::Fixture;

/// Waits for a condition to become true, or gives up.
///
/// Watching is the platform's to schedule, not ours, so the tests wait for an
/// outcome rather than sleeping for a fixed time and hoping.
fn within(limit: Duration, mut ready: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    ready()
}

#[test]
fn a_film_arriving_in_chunks_is_indexed_once_it_has_finished_arriving() {
    let library = Fixture::new();
    let scanner = Arc::clone(&library.scanner);
    let scans = Arc::new(AtomicUsize::new(0));

    let counted = Arc::clone(&scans);
    let mut watcher = FolderWatcher::with_quiet(Duration::from_millis(400), move |paths| {
        counted.fetch_add(1, Ordering::Relaxed);
        let _ = scanner.scan_containing(&paths, &Silent);
    })
    .unwrap();
    watcher.watch(&library.films).unwrap();

    // A download, arriving the way downloads do.
    let film = library.path("Heat.1995.mkv");
    let mut writing = std::fs::File::create(&film).unwrap();
    for _ in 0..40 {
        writing.write_all(b"another chunk of a film").unwrap();
        writing.flush().unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(writing);
    library.subtitle("Heat.1995.srt", &["A helicopter over the freeway"]);

    // Waiting for the dialogue rather than for the film, since the film row is
    // written first and a scan is not finished until its cues are in.
    let indexed = within(Duration::from_secs(10), || {
        let database = library.database();
        let Some(recorded) = database.films().by_path(&film).unwrap() else {
            return false;
        };
        database
            .tracks()
            .for_film(recorded.id)
            .unwrap()
            .iter()
            .any(|track| track.cue_count > 0)
    });
    assert!(
        indexed,
        "the new film and its dialogue should be in the library"
    );
    assert_eq!(library.dialogue("Heat.1995.mkv").len(), 1);

    // A little longer than the quiet period, in case the platform is still
    // holding an event back.
    std::thread::sleep(Duration::from_millis(800));
    let scans = scans.load(Ordering::Relaxed);

    // The bound is loose on purpose. What is being tested is that a file
    // arriving in pieces is one ingest rather than one per piece, and how many
    // it comes to depends on when the platform chooses to deliver its events: a
    // machine busy enough to stall the writing for longer than the quiet period
    // legitimately settles more than once. The arithmetic of settling is pinned
    // exactly by the unit tests in `debounce`, against a clock rather than
    // against a filesystem.
    assert!(
        scans <= 5,
        "a file written in forty pieces should be a handful of ingests, not forty, and was {scans}"
    );
}

#[test]
fn changes_outside_a_watched_folder_are_nothing_to_do_with_us() {
    let library = Fixture::new();
    let scanner = Arc::clone(&library.scanner);
    let scans = Arc::new(AtomicUsize::new(0));

    let counted = Arc::clone(&scans);
    let mut watcher = FolderWatcher::with_quiet(Duration::from_millis(150), move |paths| {
        counted.fetch_add(
            scanner.scan_containing(&paths, &Silent).unwrap().len(),
            Ordering::Relaxed,
        );
    })
    .unwrap();

    let elsewhere = library.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    watcher.watch(&elsewhere).unwrap();

    std::fs::write(elsewhere.join("Ronin.1998.mkv"), b"not really a film").unwrap();

    assert!(
        !within(Duration::from_secs(2), || scans.load(Ordering::Relaxed) > 0),
        "a folder nobody asked to watch should not be scanned"
    );
    assert!(library.database().films().list().unwrap().is_empty());
}
