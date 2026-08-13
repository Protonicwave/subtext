//! How long the library takes to index and to search.
//!
//! Two numbers matter. Indexing a thousand films means writing something like a
//! million cues, and the target for that is ten seconds. Searching that million
//! has to come back in under fifty milliseconds, because the palette runs the
//! query on every keystroke.
//!
//! The million cue corpus is built once at the start of the run, and building
//! it is itself the ingest measurement. Each search prints how many lines it
//! matched, since a query matching a fifth of the library is a different
//! question from one matching a few thousand lines.

// A benchmark that cannot open its own database has nothing to measure, so it
// stops where it stands rather than reporting a number for something else.
#![allow(clippy::expect_used, clippy::cast_precision_loss)]

use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use criterion::{Criterion, Throughput};
use subtext_core::{Correction, Cue, SubtitleLabel, Timestamp};
use subtext_index::{Database, NewFilm, NewTrack, SearchOptions, TrackMatch, TrackOrigin};
use tempfile::TempDir;

/// Roughly what a feature film comes to.
const CUES_PER_FILM: usize = 2_000;
/// Five hundred films of two thousand cues is the million the targets are
/// written against.
const FILMS: usize = 500;

/// Dialogue drawn from a small vocabulary, ordered roughly by how often the
/// words are spoken. The generator picks from the front far more often than the
/// back, so the first entries behave like the words that are in every other
/// line and the last like the ones that turn up in a single scene.
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
    "father",
    "mother",
    "brother",
    "sister",
    "morning",
    "water",
    "window",
    "kitchen",
    "street",
    "letter",
    "train",
    "hotel",
    "doctor",
    "captain",
    "garden",
    "winter",
    "silence",
    "distance",
    "harbour",
    "lantern",
    "orchestra",
    "telegram",
    "cathedral",
    "phosphorus",
    "cartography",
    "meridian",
];

/// A predictable stand-in for randomness, so that two runs measure the same
/// corpus.
struct Rolling(u64);

impl Rolling {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0 >> 33
    }

    /// Picks a word, heavily favouring the common end of the list.
    fn word(&mut self) -> &'static str {
        let span = u64::try_from(WORDS.len()).unwrap_or(1);
        let biased = (self.next() % span)
            .min(self.next() % span)
            .min(self.next() % span);
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

fn cues(seed: u64) -> Vec<Cue> {
    let mut rolling = Rolling(seed);
    (0..CUES_PER_FILM)
        .map(|at| {
            let start = u32::try_from(at).unwrap_or(u32::MAX).saturating_mul(2_500);
            Cue {
                index: u32::try_from(at).unwrap_or(u32::MAX) + 1,
                start: Timestamp::from_millis(start),
                end: Timestamp::from_millis(start + 2_000),
                text: rolling.line(),
                position: None,
            }
        })
        .collect()
}

struct Corpus {
    database: Database,
    /// A track already in the library, for measuring what one more film costs.
    track_id: i64,
    // Held so that the directory outlives the database.
    _directory: TempDir,
}

impl Corpus {
    /// Builds the corpus, reporting how long the cues took to go in.
    fn build() -> Self {
        let directory = TempDir::new().expect("a temporary directory");
        let database = Database::open(directory.path().join("library.db")).expect("a database");
        let folder = database
            .folders()
            .add(Path::new("/films"))
            .expect("a watched folder");

        let corpus: Vec<Vec<Cue>> = (0..FILMS).map(|film| cues(film as u64 + 1)).collect();
        let mut last_track = 0;

        let started = Instant::now();
        database
            .bulk_ingest(|| {
                for (at, cues) in corpus.iter().enumerate() {
                    let film = database.films().upsert(&NewFilm {
                        folder_id: folder.id,
                        path: Path::new(&format!("/films/film-{at}.mkv")),
                        title: "A film",
                        year: Some(1_999),
                        size_bytes: 4_000_000_000,
                        modified_at: 1_700_000_000_000,
                    })?;
                    let track = database.tracks().upsert(&NewTrack {
                        film_id: film.id,
                        path: Path::new(&format!("/films/film-{at}.srt")),
                        label: SubtitleLabel::default(),
                        origin: TrackOrigin::Sidecar,
                        stream_number: 0,
                        codec: "subrip",
                        match_kind: TrackMatch::Exact,
                        encoding: "UTF-8",
                        size_bytes: 60_000,
                        modified_at: 1_700_000_000_000,
                    })?;
                    database.tracks().replace_cues(track.id, cues)?;
                    last_track = track.id;
                }
                Ok(())
            })
            .expect("the corpus");
        let elapsed = started.elapsed();

        let total = FILMS * CUES_PER_FILM;
        println!(
            "\ningest: {total} cues across {FILMS} films in {:.2}s ({:.0} cues/s)",
            elapsed.as_secs_f64(),
            total as f64 / elapsed.as_secs_f64()
        );

        Self {
            database,
            track_id: last_track,
            _directory: directory,
        }
    }

    /// How many lines a search matched before any limit was applied.
    fn matches(&self, term: &str) -> usize {
        self.database
            .search()
            .find(
                term,
                &SearchOptions {
                    limit: usize::MAX,
                    per_film: usize::MAX,
                    ..SearchOptions::default()
                },
            )
            .expect("results")
            .shown()
    }
}

fn searching(criterion: &mut Criterion, corpus: &Corpus) {
    // A word in a fifth of all dialogue, a word in a few thousand lines, and a
    // word in a handful of scenes.
    let terms = ["the", "window", "phosphorus"];
    for term in terms {
        println!(
            "search term {term:?} matches {} lines",
            corpus.matches(term)
        );
    }

    let options = SearchOptions::default();
    let mut group = criterion.benchmark_group("search");
    group.throughput(Throughput::Elements(
        u64::try_from(FILMS * CUES_PER_FILM).unwrap_or(u64::MAX),
    ));

    for term in terms {
        group.bench_function(term, |bencher| {
            bencher.iter(|| {
                corpus
                    .database
                    .search()
                    .find(black_box(term), &options)
                    .expect("results")
            });
        });
    }

    group.bench_function("phrase", |bencher| {
        bencher.iter(|| {
            corpus
                .database
                .search()
                .find(black_box("\"the money\""), &options)
                .expect("results")
        });
    });

    // What the palette runs while someone is still typing.
    group.bench_function("prefix", |bencher| {
        bencher.iter(|| {
            corpus
                .database
                .search()
                .find(black_box("pho"), &options)
                .expect("results")
        });
    });

    group.bench_function("within one film", |bencher| {
        let scoped = SearchOptions {
            film: Some(1),
            ..SearchOptions::default()
        };
        bencher.iter(|| {
            corpus
                .database
                .search()
                .find(black_box("window"), &scoped)
                .expect("results")
        });
    });

    group.finish();
}

fn writing(criterion: &mut Criterion, corpus: &Corpus) {
    let mut group = criterion.benchmark_group("one film");
    group.throughput(Throughput::Elements(
        u64::try_from(CUES_PER_FILM).unwrap_or(u64::MAX),
    ));

    // One film arriving in a library that is already indexed, which is what the
    // folder watcher causes and where the triggers keep the index in step.
    let cues = cues(9_999);
    group.bench_function("indexed on the way in", |bencher| {
        bencher.iter(|| {
            corpus
                .database
                .tracks()
                .replace_cues(black_box(corpus.track_id), &cues)
                .expect("the cues")
        });
    });

    // The transcript panel asks for a whole film's cues when it opens.
    group.bench_function("read back", |bencher| {
        bencher.iter(|| {
            corpus
                .database
                .tracks()
                .cues(black_box(corpus.track_id))
                .expect("cues")
        });
    });

    // The same read with a correction on the track, which is the claim worth
    // measuring: the arithmetic is two operations per timestamp against a query,
    // a row decode and a string allocation per cue, and it should not show up
    // against the read above.
    corpus
        .database
        .tracks()
        .set_correction(corpus.track_id, Correction::new(-1_200, 25.0 / 23.976))
        .expect("the correction");
    group.bench_function("read back, corrected", |bencher| {
        bencher.iter(|| {
            corpus
                .database
                .tracks()
                .cues(black_box(corpus.track_id))
                .expect("cues")
        });
    });
    corpus
        .database
        .tracks()
        .set_correction(corpus.track_id, Correction::IDENTITY)
        .expect("the correction");

    group.finish();
}

fn main() {
    let corpus = Corpus::build();
    let mut criterion = Criterion::default().configure_from_args();
    searching(&mut criterion, &corpus);
    writing(&mut criterion, &corpus);
    criterion.final_summary();
}
