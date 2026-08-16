//! Measuring the alignment engine against real films.
//!
//! There is a number for how fast an alignment runs and, until this, no number
//! at all for how often it is right. The acceptance bar was set from one film,
//! and the comment on it asks whoever revisits it to measure a second before
//! moving it. This is how that is done.
//!
//! Point it at a directory of films. Every film that carries a text subtitle
//! track of its own carries timings written against those exact frames, so
//! those timings are taken as truth, and moving them by a known amount
//! manufactures a file whose correct answer is known exactly. One film's truth
//! measured against another film's audio manufactures a case that must be
//! refused. The engine is run over all of them and what comes back is a table:
//! what was accepted, what was refused, what was accepted wrongly, how far out
//! the worst line of each accepted case was left, and how the lines landed
//! before and after.
//!
//! ```text
//! cargo run --release -p alignment-corpus -- "D:\Films"
//! ```
//!
//! It is not part of the application and nothing in the application depends on
//! it. It has no library in it to be depended on, it lives outside `crates`,
//! and a test below walks the workspace to say so. Nothing here reaches the
//! network, and no film is written to: the perturbations are made in memory and
//! the only file produced is the report.

mod cases;
mod films;
mod judge;
mod report;

use std::path::PathBuf;
use std::process::ExitCode;

use subtext_core::Correction;

use crate::cases::Case;
use crate::judge::{Bars, Figures};
use crate::report::{Report, Row, Skip};

/// Where the machine readable report goes when nobody says otherwise.
const DEFAULT_REPORT: &str = "alignment-corpus.json";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let Some(arguments) = arguments()? else {
        return Ok(());
    };

    let gathered = films::gather(&arguments.films, arguments.limit, arguments.truth_lands)?;
    let films = gathered.films;
    if films.is_empty() {
        return Err("no film in that directory could be taken as truth".to_owned());
    }

    let mut rows = Vec::new();
    println!();
    for (at, film) in films.iter().enumerate() {
        let mut built = cases::manufacture(at, &film.truth);

        // The next film round the ring, so every film supplies one mismatched
        // case and no film is measured against itself. A single film supplies
        // none: measuring a subtitle against the film it was written for is not
        // a mismatch, and inventing one by shuffling its own timings would be
        // measuring the shuffle. The report says this is missing rather than
        // leaving somebody to notice.
        if films.len() > 1 {
            let against = (at + 1) % films.len();
            built.push(cases::mismatched(
                &film.truth,
                against,
                &films[against].title,
            ));
        }

        println!("Measuring {} over {} cases", film.title, built.len());
        for case in &built {
            let speech = &films[case.film].speech;
            rows.push(measure(&film.title, case, speech, arguments.bars));
        }
    }

    let report = Report {
        bars: arguments.bars,
        films: films.iter().map(films::measured).collect(),
        skipped: gathered
            .skipped
            .into_iter()
            .map(|(film, why)| Skip {
                film,
                why: why.saying(),
            })
            .collect(),
        rows,
    };

    report.print();
    report.write(&arguments.report)?;
    println!();
    println!("Written to {}", arguments.report.display());
    Ok(())
}

/// One case, measured and judged.
fn measure(film: &str, case: &Case, speech: &subtext_align::Signal, bars: Bars) -> Row {
    let found = subtext_align::align(&case.authored, speech);
    let figures = Figures::of(&found);
    let outcome = judge::outcome(figures, bars);

    // Measured whatever the outcome was, so that a report can be judged again
    // at a threshold that would have accepted this and the error is already
    // there to judge it by.
    let worst_error_ms = worst_error(case, found.correction());
    let verdict = judge::verdict(outcome, case.wanted, worst_error_ms);

    Row {
        film: film.to_owned(),
        family: case.family.name(),
        about: case.about.clone(),
        wanted_within_ms: case.wanted.slack(),
        outcome,
        verdict,
        offset_ms: found.correction().offset_ms(),
        rate: found.correction().rate(),
        worst_error_ms,
        figures,
    }
}

/// How far a correction leaves the lines from where the film speaks them, at
/// the worst line in it.
///
/// The worst rather than the middle. A correction that is right for an hour and
/// wrong for the rest is the shape of failure this whole measurement exists to
/// catch, and an average would report it as mostly fine.
fn worst_error(case: &Case, correction: Correction) -> Option<i64> {
    if case.belongs_ms.is_empty() {
        return None;
    }
    case.authored
        .iter()
        .zip(&case.belongs_ms)
        .map(|(cue, belongs)| i64::from(correction.apply(cue.start).millis()) - i64::from(*belongs))
        .map(i64::abs)
        .max()
}

/// What a run was asked for.
#[derive(Debug)]
struct Arguments {
    films: PathBuf,
    report: PathBuf,
    bars: Bars,
    /// How much of a subtitle file beside a film has to land on that film before
    /// it is taken as truth.
    truth_lands: f32,
    limit: Option<usize>,
}

/// The command line, or nothing where all that was asked for was the usage.
fn arguments() -> Result<Option<Arguments>, String> {
    let mut films: Option<PathBuf> = None;
    let mut report = PathBuf::from(DEFAULT_REPORT);
    let mut bars = Bars::default();
    let mut truth_lands = films::TRUTH_LANDS;
    let mut limit = None;

    let mut given = std::env::args().skip(1);
    while let Some(argument) = given.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                usage();
                return Ok(None);
            }
            "--report" => report = PathBuf::from(value(&mut given, "--report")?),
            "--threshold" => bars.threshold = number(&mut given, "--threshold")?,
            "--bar" => bars.bar = number(&mut given, "--bar")?,
            "--truth-lands" => truth_lands = number(&mut given, "--truth-lands")?,
            "--limit" => limit = Some(number::<usize>(&mut given, "--limit")?),
            unknown if unknown.starts_with('-') => {
                return Err(format!("{unknown} is not an option this understands"));
            }
            directory => films = Some(PathBuf::from(directory)),
        }
    }

    let Some(films) = films else {
        usage();
        return Err("a directory of films to measure against is needed".to_owned());
    };
    if !films.is_dir() {
        return Err(format!("{} is not a directory", films.display()));
    }

    Ok(Some(Arguments {
        films,
        report,
        bars,
        truth_lands,
        limit,
    }))
}

fn value(given: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    given
        .next()
        .ok_or_else(|| format!("{option} needs a value after it"))
}

fn number<T: std::str::FromStr>(
    given: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<T, String> {
    value(given, option)?
        .parse()
        .map_err(|_| format!("{option} needs a number after it"))
}

fn usage() {
    println!("Measuring the subtitle alignment engine against a directory of real films.");
    println!();
    println!("  alignment-corpus <directory> [options]");
    println!();
    println!("  --report <file>    where the machine readable report goes");
    println!("  --threshold <n>    how sure a measurement has to be before it is written");
    println!("  --bar <n>          how much of a track has to land on the talking");
    println!("  --truth-lands <n>  how much of a subtitle beside a film has to land on it");
    println!("                     before it is taken as truth for that film");
    println!("  --limit <n>        measure at most this many films");
    println!();
    println!("Only the films are read. Nothing is written to any of them, and nothing");
    println!("here reaches the network.");
}

#[cfg(test)]
mod tests {
    // A workspace this cannot read is the failure the test is looking for, and
    // stopping there says so more plainly than an assertion would.
    #![allow(clippy::expect_used)]

    use std::path::Path;

    use super::worst_error;
    use crate::cases::{Case, Family, Wanted};
    use subtext_core::{Correction, Cue, Timestamp};

    fn case(starts: &[u32], belongs: &[u32]) -> Case {
        Case {
            family: Family::Shift,
            about: "out by +2.0s".to_owned(),
            authored: starts
                .iter()
                .enumerate()
                .map(|(at, start)| Cue {
                    index: u32::try_from(at + 1).unwrap_or(u32::MAX),
                    start: Timestamp::from_millis(*start),
                    end: Timestamp::from_millis(start + 1_000),
                    text: "line".to_owned(),
                    position: None,
                })
                .collect(),
            belongs_ms: belongs.to_vec(),
            film: 0,
            wanted: Wanted::Within(50),
        }
    }

    #[test]
    fn the_error_reported_is_the_one_at_the_worst_line() {
        let case = case(&[10_000, 20_000, 30_000], &[12_000, 22_000, 35_000]);
        let found = worst_error(&case, Correction::of_offset(2_000));
        assert_eq!(found, Some(3_000));
    }

    #[test]
    fn a_case_with_no_right_answer_has_no_error_to_report() {
        let case = case(&[10_000], &[]);
        assert_eq!(worst_error(&case, Correction::of_offset(2_000)), None);
    }

    /// Nothing the application builds may depend on this, and nothing it needs
    /// may be compiled into the application. The crate has no library in it, so
    /// it cannot be named as a dependency at all, and this is what says nobody
    /// has given it one.
    #[test]
    fn no_part_of_the_application_depends_on_the_harness() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");

        let mut manifests = vec![root.join("src-tauri").join("Cargo.toml")];
        let crates = std::fs::read_dir(root.join("crates")).expect("the application's crates");
        manifests.extend(
            crates
                .flatten()
                .map(|entry| entry.path().join("Cargo.toml"))
                .filter(|manifest| manifest.is_file()),
        );
        assert!(manifests.len() > 1, "no manifests were found to check");

        for manifest in manifests {
            let text = std::fs::read_to_string(&manifest).expect("a manifest in the workspace");
            assert!(
                !text.contains("alignment-corpus"),
                "{} names the harness",
                manifest.display()
            );
        }

        // And the crate itself has no library target, so a manifest that did
        // name it would not build either.
        assert!(
            !Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("lib.rs")
                .exists()
        );
    }
}
