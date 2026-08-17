//! What a run comes to, printed for a reader and written down for a machine.
//!
//! The printed table is for whoever ran it. The file beside it is so that two
//! runs can be compared without anybody reading columns, which is what a change
//! to the engine has to be judged by: the same directory of films, before and
//! after, and the difference in the three counts that matter.
//!
//! Everything the decision reads is written down as well as the decision
//! itself, which is what lets a report be judged again at a proposed threshold
//! and a proposed bar. The sweeps below do exactly that, and they are the
//! evidence those two numbers should be settled from.

use std::path::Path;

use serde::Serialize;

use crate::cases::{Family, Wanted};
use crate::judge::{self, Bars, Figures, Outcome, Verdict};

/// The thresholds a report is judged again at.
///
/// Spread by multiples rather than evenly, because confidence is a ratio: what
/// is worth knowing is which factor of two the separation sits in, not which
/// hundredth.
const THRESHOLDS: [f32; 7] = [0.02, 0.05, 0.10, 0.20, 0.30, 0.45, 0.60];

/// The bars a report is judged again at, from a quarter of a track landing to
/// nearly all of one.
const BARS: [f32; 6] = [0.2, 0.3, 0.4, 0.5, 0.6, 0.7];

/// How many films a report needs before its numbers are worth settling anything
/// on.
///
/// The plan's figure. A run over fewer is worth having, because it exercises the
/// engine end to end and will show a gross failure, but it is not evidence about
/// a threshold: the whole complaint that started this part is a number set from
/// one film.
const TELLING_FILMS: usize = 20;

/// One case, measured.
#[derive(Debug, Serialize)]
pub(crate) struct Row {
    pub(crate) film: String,
    pub(crate) family: &'static str,
    pub(crate) about: String,
    /// How far out an accepted answer could be and still be right, or nothing
    /// where the only right ending was a refusal.
    pub(crate) wanted_within_ms: Option<i64>,
    pub(crate) outcome: Outcome,
    pub(crate) verdict: Verdict,
    pub(crate) offset_ms: i32,
    pub(crate) rate: f64,
    /// How far the answer left the lines from where they belong, at the worst
    /// line in the film.
    pub(crate) worst_error_ms: Option<i64>,
    #[serde(flatten)]
    pub(crate) figures: Figures,
}

impl Row {
    /// What this case would have come to at another threshold and another bar.
    fn verdict_at(&self, bars: Bars) -> Verdict {
        let wanted = self
            .wanted_within_ms
            .map_or(Wanted::Refusal, Wanted::Within);
        judge::verdict(
            judge::outcome(self.figures, bars),
            wanted,
            self.worst_error_ms,
        )
    }
}

/// A whole run.
#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub(crate) bars: Bars,
    /// The threshold the rows below were judged at, which is not the one the
    /// rows above were.
    pub(crate) reference_threshold: f32,
    pub(crate) films: Vec<Measured>,
    pub(crate) skipped: Vec<Skip>,
    pub(crate) rows: Vec<Row>,
    /// The same cases measured against a text track inside the film rather than
    /// against its soundtrack.
    ///
    /// Kept apart rather than mixed in, for two reasons. They are judged at a
    /// threshold of their own, since a correlation of two sets of authored
    /// timings scores on a different scale from one where a speech reading is a
    /// side. And the sweeps above exist to settle the numbers the audio path is
    /// decided by, which these would only blur.
    pub(crate) reference_rows: Vec<Row>,
}

/// A film the cases were built from.
///
/// Enough to find it again and to see what it brought: two runs over a
/// directory that has changed should be told apart by this rather than by
/// somebody remembering.
#[derive(Debug, Serialize)]
pub(crate) struct Measured {
    pub(crate) title: String,
    pub(crate) path: String,
    /// Whether the timings taken as truth came from inside the film or from a
    /// file beside it, which is the difference between a clock and a claim that
    /// had to be checked.
    pub(crate) truth_from: &'static str,
    /// How much of that truth lands on the film's own speech untouched.
    ///
    /// Absent, along with everything else below that was read off the
    /// soundtrack, for a film whose soundtrack this build cannot read. Such a
    /// film is kept for its own timings and supplies no audio cases.
    pub(crate) truth_lands: Option<f32>,
    /// What the engine made of this film's truth before anything was perturbed,
    /// which every error in the table is measured against rather than against
    /// truth itself. A large value here is truth disagreeing with the film, not
    /// the engine failing.
    pub(crate) baseline_offset_ms: Option<i32>,
    pub(crate) baseline_rate: Option<f64>,
    /// How many lines its own track holds, which is the size of the truth every
    /// case built from it rests on.
    pub(crate) lines: usize,
    /// How many separate times somebody starts talking, which is the number of
    /// places a line has to land on.
    pub(crate) utterances: Option<u32>,
    pub(crate) minutes: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Skip {
    pub(crate) film: String,
    pub(crate) why: String,
}

/// How a set of cases came out.
#[derive(Clone, Copy, Debug, Default, Serialize)]
struct Tally {
    cases: usize,
    right: usize,
    wrong: usize,
    missed: usize,
}

impl Tally {
    fn count(&mut self, verdict: Verdict) {
        self.cases += 1;
        match verdict {
            Verdict::Right => self.right += 1,
            Verdict::Wrong => self.wrong += 1,
            Verdict::Missed => self.missed += 1,
        }
    }
}

impl Report {
    /// The whole run, on the screen.
    pub(crate) fn print(&self) {
        self.print_cases();
        self.print_families();
        self.print_sweeps();
        self.print_distances();
        self.print_reference();
        self.print_what_was_not_measured();
    }

    /// The same cases again, measured against the film's own timings.
    ///
    /// Where a film carries a text subtitle track, that track was written
    /// against these frames, and a second track can be measured against it
    /// without a sample being decoded. What this section says is how well that
    /// path recovers a perturbation and whether it refuses a track belonging to
    /// another film, which are the two questions asked of the audio path above.
    ///
    /// It is worth reading for what it is. Both sides here are authored timings
    /// with nothing estimated on either, so a right answer should come back to
    /// the bin rather than to the tenth of a second, and a wide margin against
    /// the mismatched cases is expected rather than impressive. What it cannot
    /// say is anything about a film whose own track is itself out, since the
    /// case and its answer are both built from that track.
    fn print_reference(&self) {
        println!();
        println!("Against a text track inside the film");
        if self.reference_rows.is_empty() {
            println!("  No film here carries one, so nothing in this run says anything about");
            println!("  measuring a subtitle against the timings a film brought with it.");
            return;
        }

        let mut tally = Tally::default();
        let mut worst = 0_i64;
        for row in &self.reference_rows {
            tally.count(row.verdict);
            // Only the ones that were written. A refused case is allowed to be
            // as far out as it likes, and counting it here would report the
            // error of a file the run correctly declined to touch.
            if row.outcome == Outcome::Accepted {
                worst = worst.max(row.worst_error_ms.unwrap_or(0));
            }
        }
        let films = self
            .films
            .iter()
            .filter(|film| film.truth_from == crate::films::Origin::Inside.name())
            .count();
        println!(
            "  {} cases over {films} film{}, judged at a threshold of {:.4} and a bar of {:.2}",
            tally.cases,
            if films == 1 { "" } else { "s" },
            self.reference_threshold,
            self.bars.bar
        );
        println!(
            "  {} right, {} wrong, {} missed, and the worst accepted answer out by {worst}ms",
            tally.right, tally.wrong, tally.missed
        );
    }

    /// What a run could not put a number on.
    ///
    /// Printed last and in full sentences, because a report that is quiet about
    /// its gaps reads as a report with none. The figure that matters most in the
    /// plan is how often a subtitle belonging to another film is written to this
    /// one, and that figure cannot be taken at all from a single film.
    fn print_what_was_not_measured(&self) {
        let mismatched = self
            .rows
            .iter()
            .filter(|row| row.family == Family::Mismatched.name())
            .count();
        if mismatched > 0 && self.films.len() >= TELLING_FILMS {
            return;
        }

        println!();
        println!("What this run does not say");
        if mismatched == 0 {
            println!("  No mismatched case could be built, because that means one film's timings");
            println!("  against another film's audio and there is only one film here. Nothing in");
            println!("  this report says anything about refusing a subtitle for another film.");
        }
        if self.films.len() < TELLING_FILMS {
            println!(
                "  {} films is too few to settle a threshold or a bar on. The plan asks for",
                self.films.len()
            );
            println!("  at least {TELLING_FILMS}, of different origins, mixes and languages.");
        }
    }

    fn print_cases(&self) {
        println!();
        println!("Cases");
        println!(
            "{:<22} {:<11} {:<28} {:<10} {:>7} {:>7} {:>7} {:>9}  verdict",
            "film", "family", "case", "outcome", "score", "before", "after", "worst"
        );

        for row in &self.rows {
            println!(
                "{:<22} {:<11} {:<28} {:<10} {:>7.4} {:>6.0}% {:>6.0}% {:>9}  {}",
                cut(&row.film, 22),
                row.family,
                cut(&row.about, 28),
                row.outcome.name(),
                row.figures.confidence,
                f64::from(row.figures.landed_before) * 100.0,
                f64::from(row.figures.landed_after) * 100.0,
                row.worst_error_ms
                    .map_or_else(|| "-".to_owned(), |worst| format!("{worst}ms")),
                row.verdict.name(),
            );
        }
    }

    fn print_families(&self) {
        println!();
        println!(
            "By family, at a threshold of {:.4} and a bar of {:.2}",
            self.bars.threshold, self.bars.bar
        );
        println!(
            "{:<12} {:>6} {:>7} {:>7} {:>7}",
            "family", "cases", "right", "wrong", "missed"
        );

        for family in [
            Family::Shift,
            Family::Stretch,
            Family::Both,
            Family::Splice,
            Family::Recut,
            Family::Mismatched,
        ] {
            let mut tally = Tally::default();
            for row in self.rows.iter().filter(|row| row.family == family.name()) {
                tally.count(row.verdict);
            }
            if tally.cases == 0 {
                continue;
            }
            println!(
                "{:<12} {:>6} {:>7} {:>7} {:>7}",
                family.name(),
                tally.cases,
                tally.right,
                tally.wrong,
                tally.missed
            );
        }

        let whole = self.tally_at(self.bars);
        println!(
            "{:<12} {:>6} {:>7} {:>7} {:>7}",
            "all", whole.cases, whole.right, whole.wrong, whole.missed
        );
    }

    /// What the run would have come to at other values, which is the evidence
    /// the two numbers should be settled from.
    ///
    /// A wrong acceptance is the column to read first. It is the failure with
    /// nothing underneath it: somebody is handed a film that is wrong from
    /// beginning to end with no reason to suspect the tool rather than the file.
    /// A missed case leaves them where they were, with the keys they already
    /// had, so the two are not weighed against each other evenly.
    fn print_sweeps(&self) {
        println!();
        println!("At other thresholds, with the bar at {:.2}", self.bars.bar);
        println!(
            "{:<12} {:>7} {:>7} {:>7}",
            "threshold", "right", "wrong", "missed"
        );
        for threshold in THRESHOLDS {
            let tally = self.tally_at(Bars {
                threshold,
                bar: self.bars.bar,
            });
            println!(
                "{threshold:<12.4} {:>7} {:>7} {:>7}",
                tally.right, tally.wrong, tally.missed
            );
        }

        println!();
        println!(
            "At other bars, with the threshold at {:.4}",
            self.bars.threshold
        );
        println!(
            "{:<12} {:>7} {:>7} {:>7}",
            "bar", "right", "wrong", "missed"
        );
        for bar in BARS {
            let tally = self.tally_at(Bars {
                threshold: self.bars.threshold,
                bar,
            });
            println!(
                "{bar:<12.2} {:>7} {:>7} {:>7}",
                tally.right, tally.wrong, tally.missed
            );
        }
    }

    /// How far a right answer actually leaves the lines from the talking.
    ///
    /// This is what the tolerance the landing figure is measured at should be
    /// settled against. If the middle line of a correctly aligned track sits
    /// well inside the tolerance on every film, the tolerance is wide enough to
    /// see a right answer; if it sits on top of it, the figure is measuring the
    /// tolerance rather than the timing.
    fn print_distances(&self) {
        let mut distances: Vec<u32> = self
            .rows
            .iter()
            .filter(|row| row.verdict == Verdict::Right && row.outcome == Outcome::Accepted)
            .map(|row| row.figures.median_after_ms)
            .collect();
        if distances.is_empty() {
            return;
        }
        distances.sort_unstable();

        println!();
        println!(
            "Middle distance from a line to the talking, across {} correct answers",
            distances.len()
        );
        for (name, at) in [("lowest", 0.0), ("middle", 0.5), ("highest", 1.0)] {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss,
                clippy::cast_sign_loss
            )]
            let at = ((distances.len() - 1) as f64 * at) as usize;
            println!("  {name}: {}ms", distances.get(at).copied().unwrap_or(0));
        }
    }

    fn tally_at(&self, bars: Bars) -> Tally {
        let mut tally = Tally::default();
        for row in &self.rows {
            tally.count(row.verdict_at(bars));
        }
        tally
    }

    /// The same run, in a file, so that two of them can be compared.
    ///
    /// # Errors
    ///
    /// Where the file cannot be written.
    pub(crate) fn write(&self, to: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|why| format!("the report could not be made: {why}"))?;
        std::fs::write(to, json)
            .map_err(|why| format!("{} could not be written: {why}", to.display()))
    }
}

/// A name at a width a column can hold, with the end kept where it will not fit.
///
/// The end rather than the beginning: films in a series share a beginning and
/// differ at the end, so cutting the front is what keeps two rows apart.
fn cut(text: &str, width: usize) -> String {
    let long = text.chars().count();
    if long <= width {
        return text.to_owned();
    }
    let kept: String = text.chars().skip(long - width + 1).collect();
    format!("…{kept}")
}

#[cfg(test)]
mod tests {
    use super::{Row, cut};
    use crate::judge::{Bars, Figures, Outcome, Verdict};

    fn row(confidence: f32, landed_after: f32, worst_error_ms: Option<i64>) -> Row {
        Row {
            film: "Heat".to_owned(),
            family: "shift",
            about: "out by +2.0s".to_owned(),
            wanted_within_ms: Some(50),
            outcome: Outcome::Accepted,
            verdict: Verdict::Right,
            offset_ms: 2_000,
            rate: 1.0,
            worst_error_ms,
            figures: Figures {
                confidence,
                agreement: 0.9,
                tightness: 0.8,
                separation: 0.68,
                landed_before: 0.30,
                landed_after,
                median_before_ms: 1_900,
                median_after_ms: 340,
                examined: 1_200,
                misfit: None,
            },
        }
    }

    /// The sweep is only worth printing if a row can be judged again without
    /// the film being read again, which is what keeping the figures beside the
    /// decision is for.
    #[test]
    fn a_row_is_judged_again_at_another_threshold_and_bar() {
        let row = row(0.49, 0.85, Some(20));

        assert_eq!(
            row.verdict_at(Bars {
                threshold: 0.10,
                bar: 0.58
            }),
            Verdict::Right
        );
        assert_eq!(
            row.verdict_at(Bars {
                threshold: 0.60,
                bar: 0.58
            }),
            Verdict::Missed
        );
        assert_eq!(
            row.verdict_at(Bars {
                threshold: 0.10,
                bar: 0.9
            }),
            Verdict::Missed
        );
    }

    /// An answer accepted and out by more than the case allows is the failure
    /// the whole report is written to count, and it stays wrong at every bar
    /// that would still accept it.
    #[test]
    fn an_answer_left_far_from_where_it_belongs_is_wrong_at_any_bar_that_takes_it() {
        let row = row(0.49, 0.85, Some(4_000));
        for bar in [0.2, 0.4, 0.6] {
            assert_eq!(
                row.verdict_at(Bars {
                    threshold: 0.10,
                    bar
                }),
                Verdict::Wrong,
                "at a bar of {bar}"
            );
        }
    }

    #[test]
    fn a_name_too_long_for_its_column_keeps_its_end() {
        assert_eq!(cut("Heat", 10), "Heat");
        assert_eq!(cut("The Long Good Friday", 10), "…od Friday");
        // Counted in characters rather than bytes, so a title that is not
        // English is cut where it reads rather than through a letter.
        assert_eq!(cut("Les Quatre Cents Coups", 8), "…s Coups");
    }
}
