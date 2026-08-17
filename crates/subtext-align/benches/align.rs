//! How long it takes to work out what a track is out by.
//!
//! Four numbers matter. Building the cue signal happens before anything else and
//! has to disappear against the rest of the work, so it is measured at the five
//! thousand cues a long film carries.
//!
//! Then the whole search, twice. It has two stages: a correlation over the whole
//! film at each candidate stretch, which is held to a second, and then the film
//! in stretches, each measured against each surviving candidate, which is held
//! to two. The local stage has no caller of its own to be measured through, so
//! what is timed here is an alignment end to end, once on a film that needs only
//! shifting and once on a film that needs stretching. The second is the worst
//! case: every candidate has to be carried through the local stage before any of
//! them can be ruled out, which is the whole reason that stage exists.
//!
//! The fourth is the landing figure, which the search reads twice, and which has
//! to stay small enough that checking an answer is never a reason not to check
//! it. Five milliseconds over a three thousand line track is the bar.

use std::hint::black_box;

use criterion::Criterion;
use subtext_align::{BIN_MS, Signal, align, landing_of};
use subtext_core::{Correction, Cue, Timestamp};

/// Two hours, which is a film.
const FILM_MS: u32 = 2 * 60 * 60 * 1_000;

/// What a long film's transcript runs to.
const CUES: u32 = 5_000;

/// The track length the landing figure is held to.
const MEASURED: u32 = 3_000;

/// How far the film's dialogue falls after the file claims it does.
const LATE_MS: u32 = 2_500;

/// The stretch a release taken from twenty five frames a second to 23.976
/// carries, which is the largest of the candidates and the one that puts every
/// other candidate furthest out.
const STRETCH: f64 = 25.0 / 23.976;

fn main() {
    let cues = dialogue(CUES);
    let speech = Signal::from_cues(&spoken(&cues));

    let mut criterion = Criterion::default().configure_from_args();

    criterion.bench_function("build a cue signal from five thousand cues", |bencher| {
        bencher.iter(|| black_box(Signal::from_cues(black_box(&cues))));
    });

    // The claim this crate's share of an alignment rests on. Everything above
    // it, the decoding in particular, is measured elsewhere.
    criterion.bench_function("align two hours of a film that needs shifting", |bencher| {
        bencher.iter(|| black_box(align(black_box(&cues), black_box(&speech))));
    });

    // The same again where the film needs stretching as well, which is the case
    // no candidate can be ruled out of cheaply.
    let stretched = Signal::from_cues(&spoken_at(&cues, STRETCH));
    criterion.bench_function(
        "align two hours of a film that needs stretching",
        |bencher| {
            bencher.iter(|| black_box(align(black_box(&cues), black_box(&stretched))));
        },
    );

    // Measured where it is worst. A line with an utterance under it is answered
    // by the first bin the search looks in; a line with nothing anywhere near it
    // costs the whole reach, both ways. So the film here talks through its first
    // few minutes and says nothing for the rest of its length, which puts almost
    // every one of the three thousand lines into the expensive case.
    let track = dialogue(MEASURED);
    let quiet_film = talks_early(&track);
    criterion.bench_function("measure where three thousand lines land", |bencher| {
        bencher.iter(|| {
            black_box(landing_of(
                black_box(&track),
                black_box(&quiet_film),
                Correction::of_offset(i32::try_from(LATE_MS).unwrap_or_default()),
            ))
        });
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

/// A film the length of the others that talks only over its opening minutes.
fn talks_early(cues: &[Cue]) -> Signal {
    let bin = |ms: u32| (ms + LATE_MS) as usize / BIN_MS as usize;
    let mut bins = vec![false; (FILM_MS / BIN_MS) as usize];
    for cue in cues.iter().take(cues.len() / 20) {
        bins[bin(cue.start.millis())..bin(cue.end.millis())].fill(true);
    }
    Signal::from_bins(bins)
}

/// The same dialogue where the film actually says it.
fn spoken(cues: &[Cue]) -> Vec<Cue> {
    spoken_at(cues, 1.0)
}

/// The same again for a film running at a different framerate, which is the
/// stretch a release taken from twenty five frames to 23.976 carries.
fn spoken_at(cues: &[Cue], rate: f64) -> Vec<Cue> {
    let late = Correction::new(i32::try_from(LATE_MS).unwrap_or_default(), rate);
    cues.iter()
        .map(|cue| Cue {
            start: late.apply(cue.start),
            end: late.apply(cue.end),
            ..cue.clone()
        })
        .collect()
}
