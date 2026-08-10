//! How long it takes to read a subtitle file.
//!
//! Parsing is the one genuinely expensive thing Subtext does: a library of a
//! thousand films is a thousand of these, and the target for the whole lot is
//! ten seconds across eight cores. That leaves a typical two thousand cue file
//! with a budget of five milliseconds on one core.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use subtext_core::parse_srt;

/// Roughly what a feature film comes to.
const TYPICAL_CUES: usize = 2_000;

/// Writes a subtitle file of the given size.
///
/// Cue text is drawn from a handful of lines of differing lengths so that the
/// measurement is not dominated by one string length, and every fourth cue
/// runs to two lines, which is how real files read.
fn synthetic(cues: usize, markup: bool) -> String {
    const LINES: &[&str] = &[
        "Yes.",
        "I know what you mean.",
        "It was the sort of afternoon that gets away from you.",
        "Nobody said anything for a while after that.",
        "Are you coming or not?",
        // Not everything is ASCII, and a single accent is what forces the
        // encoding to be worked out rather than assumed.
        "Après tout, on prend un café.",
    ];

    let mut text = String::with_capacity(cues * 80);
    for cue in 0..cues {
        let start = u32::try_from(cue).unwrap_or(u32::MAX).saturating_mul(2_500);
        let end = start.saturating_add(2_000);

        text.push_str(&(cue + 1).to_string());
        text.push('\n');
        text.push_str(&timestamp(start));
        text.push_str(" --> ");
        text.push_str(&timestamp(end));
        text.push('\n');

        let line = LINES[cue % LINES.len()];
        if markup {
            text.push_str("<i>");
            text.push_str(line);
            text.push_str("</i>\n");
        } else {
            text.push_str(line);
            text.push('\n');
        }
        if cue % 4 == 0 {
            text.push_str("A second line, spoken by someone else.\n");
        }
        text.push('\n');
    }
    text
}

fn timestamp(millis: u32) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        millis / 3_600_000,
        (millis / 60_000) % 60,
        (millis / 1_000) % 60,
        millis % 1_000
    )
}

fn parsing(criterion: &mut Criterion) {
    let plain = synthetic(TYPICAL_CUES, false).into_bytes();
    let with_markup = synthetic(TYPICAL_CUES, true).into_bytes();
    // The same file in a single byte encoding, which has to be sniffed and
    // converted before a single cue can be read.
    let western = encoding_rs::WINDOWS_1252
        .encode(&synthetic(TYPICAL_CUES, false))
        .0
        .into_owned();

    let mut group = criterion.benchmark_group("parse");
    group.throughput(Throughput::Elements(
        u64::try_from(TYPICAL_CUES).unwrap_or(u64::MAX),
    ));

    group.bench_function("plain", |bencher| {
        bencher.iter(|| parse_srt(black_box(&plain)));
    });
    group.bench_function("markup", |bencher| {
        bencher.iter(|| parse_srt(black_box(&with_markup)));
    });
    group.bench_function("windows-1252", |bencher| {
        bencher.iter(|| parse_srt(black_box(&western)));
    });

    group.finish();
}

criterion_group!(benches, parsing);
criterion_main!(benches);
