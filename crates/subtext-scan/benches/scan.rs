//! How long a folder of a thousand films takes to scan and to index.
//!
//! Three numbers matter, and all three are about the first time somebody points
//! the application at their collection. Walking a thousand files has to come
//! back in under two seconds, reading and indexing them in under ten, and a
//! rescan of a library nobody has touched has to cost almost nothing at all.
//!
//! The folder is built on disk once at the start of the run, because a scan
//! that is not reading real files is not measuring anything. Building it is not
//! part of any measurement.

// A benchmark that cannot write its own folder has nothing to measure, so it
// stops where it stands rather than reporting a number for something else.
#![allow(clippy::expect_used, clippy::cast_precision_loss)]

use std::fmt::Write as _;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use criterion::Criterion;
use subtext_container::fixture::{Container, Entry, Line};
use subtext_core::ParsedName;
use subtext_index::Database;
use subtext_scan::{FoundFile, Scanner, Silent, discover, on_disk};
use tempfile::TempDir;

/// The thousand file folder the targets are written against.
const FILMS: usize = 1_000;

/// Roughly what a feature film comes to, which puts the folder at a million
/// lines of dialogue.
const CUES_PER_FILM: usize = 1_000;

/// How many lines each film carries inside it as well, and how much picture
/// there is between them.
///
/// Fewer than the file beside it, and with frames between them, because what
/// this is here to price is the stepping over: a film with dialogue and nothing
/// else in it would let a reader that read every block wholesale look as quick
/// as one that did not. A corpus of full length films with real bitrates would
/// be several hundred gigabytes, so the count is small and the shape is right.
const EMBEDDED_CUES: usize = 100;
const FRAMES_PER_CLUSTER: usize = 6;
const FRAME: usize = 1_500;

/// How many of the films sit in a subfolder of their own rather than loose in
/// the root, since most collections are some of each.
const IN_SUBFOLDERS: usize = 400;

/// The library the cover decision is priced against, which is larger than the
/// one above because covers are the part of a scan that runs against every film
/// every time and the target in the plan is written at this size.
const COVER_FILMS: usize = 2_000;

/// Three pictures a film, which is roughly what a library that has been through
/// a media manager carries: a poster, a backdrop and one more besides.
const IMAGES_PER_FILM: usize = 3;

const WORDS: &[&str] = &[
    "the",
    "you",
    "and",
    "that",
    "what",
    "know",
    "here",
    "come",
    "look",
    "will",
    "have",
    "this",
    "there",
    "think",
    "want",
    "going",
    "right",
    "never",
    "tell",
    "back",
    "time",
    "good",
    "away",
    "night",
    "money",
    "help",
    "wait",
    "hear",
    "leave",
    "please",
    "sorry",
    "listen",
    "harbour",
    "lantern",
    "orchestra",
    "telegram",
    "cathedral",
    "phosphorus",
];

/// A predictable stand-in for randomness, so that two runs measure the same
/// folder.
struct Rolling(u64);

impl Rolling {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0 >> 33
    }

    fn word(&mut self) -> &'static str {
        let span = u64::try_from(WORDS.len()).unwrap_or(1);
        let biased = (self.next() % span).min(self.next() % span);
        WORDS
            .get(usize::try_from(biased).unwrap_or(0))
            .copied()
            .unwrap_or("the")
    }

    fn line(&mut self) -> String {
        let words = 4 + usize::try_from(self.next() % 6).unwrap_or(0);
        let mut line = String::with_capacity(words * 7);
        for at in 0..words {
            if at > 0 {
                line.push(' ');
            }
            line.push_str(self.word());
        }
        line
    }
}

/// One subtitle file, as it would be on disk.
fn srt(seed: u64) -> String {
    let mut rolling = Rolling(seed);
    let mut text = String::with_capacity(CUES_PER_FILM * 60);
    for at in 0..CUES_PER_FILM {
        let start = at * 3;
        let _ = writeln!(
            text,
            "{}\n{} --> {}\n{}\n",
            at + 1,
            timing(start),
            timing(start + 2),
            rolling.line()
        );
    }
    text
}

fn timing(seconds: usize) -> String {
    format!(
        "{:02}:{:02}:{:02},000",
        seconds / 3_600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

struct Folder {
    films: PathBuf,
    root: PathBuf,
    _directory: TempDir,
}

impl Folder {
    /// Writes the folder out. Not measured: this is the collection, not the
    /// scanning of it.
    fn build() -> Self {
        let directory = TempDir::new().expect("a temporary directory");
        let root = directory.path().to_path_buf();
        let films = root.join("films");
        std::fs::create_dir_all(&films).expect("the folder");

        let started = Instant::now();
        for at in 0..FILMS {
            let folder = if at < IN_SUBFOLDERS {
                let nested = films.join(format!("Film {at:04}"));
                std::fs::create_dir_all(&nested).expect("a subfolder");
                nested
            } else {
                films.clone()
            };

            let name = format!("Film {at:04}.2001.1080p.BluRay.x264-GROUP");
            std::fs::write(folder.join(format!("{name}.mkv")), film(at as u64 + 1))
                .expect("a film");
            std::fs::write(
                folder.join(format!("Film {at:04}.2001.en.srt")),
                srt(at as u64 + 1),
            )
            .expect("a subtitle file");
        }

        println!(
            "\nfolder: {FILMS} films and {FILMS} subtitle files written in {:.2}s",
            started.elapsed().as_secs_f64()
        );

        Self {
            films,
            root,
            _directory: directory,
        }
    }

    fn library(&self) -> Database {
        Database::open(self.root.join("library.db")).expect("a database")
    }
}

/// A film, with dialogue inside it and frames of picture between the lines.
///
/// Real Matroska rather than a few bytes standing in for it, so that the cost
/// of opening a thousand films and reading what is in them is part of what the
/// scan is measured at rather than something the corpus quietly avoids.
fn film(seed: u64) -> Vec<u8> {
    let mut rolling = Rolling(seed);
    let dialogue = (0..EMBEDDED_CUES)
        .map(|at| {
            let start = at as u64 * 3_000;
            Line::new(3, start, start + 2_000, &rolling.line())
        })
        .collect();

    Container::new(vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
        Entry::subtitle(4, "S_HDMV/PGS").in_language("eng"),
    ])
    .with_seek_head()
    .with_picture(1, FRAMES_PER_CLUSTER, FRAME)
    .with_dialogue(dialogue)
    .bytes()
}

/// The three measurements the targets are written against, each done once.
fn against_the_targets(folder: &Folder) {
    let started = Instant::now();
    let found = discover(&folder.films);
    let walking = started.elapsed();
    println!(
        "cold scan: {} files, {} films and {} subtitle files in {:.2}s (target 2s)",
        found.files_seen,
        found.films.len(),
        found.subtitles.len(),
        walking.as_secs_f64()
    );

    let scanner = Scanner::new(folder.library());
    let watched = scanner.add_folder(&folder.films).expect("a watched folder");

    let started = Instant::now();
    let outcome = scanner.scan(&watched, &Silent).expect("a scan");
    let indexing = started.elapsed();
    println!(
        "index: {} films and {} cues in {:.2}s ({:.0} cues/s, target 10s)",
        outcome.films_found,
        outcome.cues_indexed,
        indexing.as_secs_f64(),
        outcome.cues_indexed as f64 / indexing.as_secs_f64()
    );

    println!(
        "inside the films: {} opened, {} tracks found in them (target 3s for 1,000)",
        outcome.films_probed, outcome.embedded_tracks
    );

    let started = Instant::now();
    let again = scanner.scan(&watched, &Silent).expect("a rescan");
    println!(
        "rescan, nothing changed: {} files read, {} films opened, {} cues written, in {:.2}s",
        again.subtitles_read,
        again.films_probed,
        again.cues_indexed,
        started.elapsed().as_secs_f64()
    );
}

/// The files a cover decision weighs, built in memory rather than on disk.
///
/// What is being priced here is the weighing, which is a question about names,
/// and the walk that produces these is measured on its own above. Writing
/// twenty thousand files out to time a comparison between their names would
/// measure the filesystem instead.
fn a_library_of_names() -> (
    Vec<FoundFile>,
    Vec<ParsedName>,
    Vec<FoundFile>,
    Vec<FoundFile>,
) {
    let mut films = Vec::with_capacity(COVER_FILMS);
    let mut images = Vec::with_capacity(COVER_FILMS * IMAGES_PER_FILM);
    let mut sidecars = Vec::with_capacity(COVER_FILMS);

    for at in 0..COVER_FILMS {
        // Half in folders of their own and half loose, which is what decides
        // whether a fixed name counts and so exercises both rules.
        let folder = if at % 2 == 0 {
            PathBuf::from("films").join(format!("Film {at:04} ({})", 1970 + at % 50))
        } else {
            PathBuf::from("films")
        };

        let name = format!("Film {at:04}.{}.1080p.BluRay.x264-GROUP", 1970 + at % 50);
        films.push(found(folder.join(format!("{name}.mkv"))));
        images.push(found(folder.join(format!("Film {at:04}-poster.jpg"))));
        images.push(found(folder.join(format!("Film {at:04}-fanart.jpg"))));
        images.push(found(folder.join("backdrop.png")));
        sidecars.push(found(folder.join(format!("Film {at:04}.nfo"))));
    }

    let names = films
        .iter()
        .map(|film| ParsedName::from_file_name(&film.file_name))
        .collect();

    (films, names, images, sidecars)
}

fn found(path: PathBuf) -> FoundFile {
    FoundFile {
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned(),
        path,
        size_bytes: 0,
        modified_at: 0,
    }
}

/// Deciding where two thousand films take their covers from, which happens at
/// the end of every scan and has to stay under 40ms.
fn covers(criterion: &mut Criterion) {
    let (films, names, images, sidecars) = a_library_of_names();
    println!(
        "
covers: {} films, {} images and {} sidecars (target 40ms)",
        films.len(),
        images.len(),
        sidecars.len()
    );

    let mut group = criterion.benchmark_group("covers");
    group.sample_size(50);

    group.bench_function("two thousand films", |bencher| {
        bencher.iter(|| {
            on_disk(
                black_box(&films),
                black_box(&names),
                black_box(&images),
                black_box(&sidecars),
            )
        });
    });

    group.finish();
}

/// Walking on its own, repeated, since it is the part that runs every time the
/// application starts.
fn walking(criterion: &mut Criterion, folder: &Folder) {
    let mut group = criterion.benchmark_group("walk");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("a thousand films", |bencher| {
        bencher.iter(|| discover(black_box(folder.films.as_path())));
    });

    group.finish();
}

/// A rescan that finds nothing to do, which is what starting the application
/// with an unchanged library costs.
fn rescanning(criterion: &mut Criterion, folder: &Folder) {
    let scanner = Scanner::new(folder.library());
    let watched = scanner
        .add_folder(Path::new(&folder.films))
        .expect("a watched folder");

    let mut group = criterion.benchmark_group("rescan");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("nothing changed", |bencher| {
        bencher.iter(|| {
            scanner
                .scan(black_box(&watched), &Silent)
                .expect("a rescan")
        });
    });

    group.finish();
}

fn main() {
    let folder = Folder::build();
    against_the_targets(&folder);

    let mut criterion = Criterion::default().configure_from_args();
    walking(&mut criterion, &folder);
    rescanning(&mut criterion, &folder);
    covers(&mut criterion);
    criterion.final_summary();
}
