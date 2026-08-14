//! How long it takes to work out what a track is out by.
//!
//! Two numbers matter. Building the cue signal happens before anything else and
//! has to disappear against the rest of the work, so it is measured at the five
//! thousand cues a long film carries. And the whole search, being six rate
//! candidates over a two hour film, has to come in under a second, since it runs
//! while somebody waits and the audio it is measured against costs a great deal
//! more than this does.

use std::hint::black_box;

use criterion::Criterion;
use subtext_align::{Signal, align};
use subtext_core::{Cue, Timestamp};

/// Two hours, which is a film.
const FILM_MS: u32 = 2 * 60 * 60 * 1_000;

/// What a long film's transcript runs to.
const CUES: u32 = 5_000;

/// How far the film's dialogue falls after the file claims it does.
const LATE_MS: u32 = 2_500;

fn main() {
    let cues = dialogue(CUES);
    let speech = Signal::from_cues(&spoken(&cues));

    let mut criterion = Criterion::default().configure_from_args();

    criterion.bench_function("build a cue signal from five thousand cues", |bencher| {
        bencher.iter(|| black_box(Signal::from_cues(black_box(&cues))));
    });

    // The claim this crate's share of an alignment rests on. Everything above
    // it, the decoding in particular, is measured elsewhere.
    criterion.bench_function("align two hours across six rate candidates", |bencher| {
        bencher.iter(|| black_box(align(black_box(&cues), black_box(&speech))));
    });

    criterion.final_summary();
}

/// A transcript spread across the film, in exchanges rather than evenly.
fn dialogue(count: u32) -> Vec<Cue> {
    let gap = FILM_MS / count;
    (0..count)
        .map(|at| {
            let start = at * gap + (at % 5) * 90;
            Cue {
                index: at + 1,
                start: Timestamp::from_millis(start),
                end: Timestamp::from_millis(start + 1_200 + (at % 3) * 500),
                text: "line".to_owned(),
                position: None,
            }
        })
        .collect()
}

/// The same dialogue where the film actually says it.
fn spoken(cues: &[Cue]) -> Vec<Cue> {
    cues.iter()
        .map(|cue| Cue {
            start: Timestamp::from_millis(cue.start.millis() + LATE_MS),
            end: Timestamp::from_millis(cue.end.millis() + LATE_MS),
            ..cue.clone()
        })
        .collect()
}
