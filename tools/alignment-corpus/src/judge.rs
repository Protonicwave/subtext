//! Whether a measurement would have been written, and whether it should have
//! been.
//!
//! The application decides in `src-tauri/src/align.rs`, against a threshold on
//! how clearly the correlation chose and a bar on how many lines the answer puts
//! on the talking. The numbers are repeated here rather than shared, because the
//! application has no business exporting them and this has no business depending
//! on the application. They must be kept in step: a change in either place is a
//! change in both, and the whole use of this harness is that it decides the same
//! way the product does.
//!
//! Both are also settable from the command line, so that a report can be taken
//! at a proposed value without editing anything, which is what settling them
//! from a run of real films actually takes.

use serde::Serialize;
use subtext_align::Alignment;

use crate::cases::Wanted;

/// How sure the correlation has to be. The application's value.
pub(crate) const THRESHOLD: f32 = 0.015;

/// How much of a track has to land on the talking. The application's value.
pub(crate) const BAR: f32 = 0.4;

/// How sure a measurement against another subtitle track has to be. The
/// application's value.
///
/// A separate number from [`THRESHOLD`] for the reason the application gives at
/// greater length: the two are read off correlations of different kinds and the
/// whole scale moves between them. One side of a speech correlation is an
/// estimate, so a correct answer on a real film reaches about a quarter of a
/// perfect match; against another subtitle track both sides are authored timings
/// and a correct answer reaches most of one. On the first real film measured
/// here a correct answer scored 0.455 and another film's subtitle scored 0.0040.
pub(crate) const REFERENCE_THRESHOLD: f32 = 0.08;

/// The two numbers a run decides by.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Bars {
    pub(crate) threshold: f32,
    pub(crate) bar: f32,
}

impl Default for Bars {
    fn default() -> Self {
        Self {
            threshold: THRESHOLD,
            bar: BAR,
        }
    }
}

/// Everything about a measurement that the decision reads.
///
/// Kept apart from the measurement itself so that a report can be re-decided at
/// a different threshold and a different bar without the films being read again.
/// That is what makes a sweep over candidate values cost nothing.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Figures {
    pub(crate) confidence: f32,
    pub(crate) peak: f32,
    pub(crate) margin: f32,
    pub(crate) landed_before: f32,
    pub(crate) landed_after: f32,
    pub(crate) median_before_ms: u32,
    pub(crate) median_after_ms: u32,
    pub(crate) examined: u32,
}

impl Figures {
    pub(crate) fn of(found: &Alignment) -> Self {
        Self {
            confidence: found.confidence().score(),
            peak: found.confidence().peak(),
            margin: found.confidence().margin(),
            landed_before: found.as_written().fraction(),
            landed_after: found.landing().fraction(),
            median_before_ms: found.as_written().median_ms(),
            median_after_ms: found.landing().median_ms(),
            examined: found.landing().examined(),
        }
    }
}

/// How a measurement ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Outcome {
    /// Written to the track.
    Accepted,
    /// The correlation was not clear enough to act on.
    Uncertain,
    /// Clear enough, and it would not have put the lines on the talking.
    NoBetter,
}

impl Outcome {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Uncertain => "uncertain",
            Self::NoBetter => "no better",
        }
    }
}

/// What the application would do with this measurement.
///
/// The order and the conditions are the application's, and the comments on why
/// each one is there live with it rather than being restated here.
pub(crate) fn outcome(figures: Figures, bars: Bars) -> Outcome {
    if figures.confidence < bars.threshold {
        return Outcome::Uncertain;
    }
    if figures.examined == 0
        || figures.landed_after < figures.landed_before
        || figures.landed_after < bars.bar
    {
        return Outcome::NoBetter;
    }
    Outcome::Accepted
}

/// Whether the ending was the right one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Verdict {
    /// A correctable file corrected, or an uncorrectable one refused.
    Right,
    /// A correction written that should not have been: either to a file
    /// nothing could put right, or one that left the lines somewhere other than
    /// where they belong. This is the only column that matters, and the only
    /// failure with nothing underneath it to catch it.
    Wrong,
    /// A file that could have been put right and was refused. Somebody is left
    /// where they were, with the keys they already had.
    Missed,
}

impl Verdict {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Wrong => "wrong",
            Self::Missed => "missed",
        }
    }
}

/// What a case comes to, given how it ended and how far out it left the lines.
///
/// A case with no right answer is judged only on whether it was refused. A case
/// with one is judged on the worst line in the film rather than on an average,
/// because a correction that is right for an hour and wrong for the rest is the
/// failure this whole part exists to stop.
pub(crate) fn verdict(outcome: Outcome, wanted: Wanted, worst_ms: Option<i64>) -> Verdict {
    match (outcome, wanted.slack()) {
        (Outcome::Accepted, Some(slack)) => {
            if worst_ms.is_some_and(|worst| worst <= slack) {
                Verdict::Right
            } else {
                Verdict::Wrong
            }
        }
        (Outcome::Accepted, None) => Verdict::Wrong,
        (_, Some(_)) => Verdict::Missed,
        (_, None) => Verdict::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::{Bars, Figures, Outcome, Verdict, outcome, verdict};
    use crate::cases::Wanted;

    /// A measurement sure enough to act on, landing most of a track.
    fn good() -> Figures {
        Figures {
            confidence: 0.03,
            peak: 0.25,
            margin: 0.12,
            landed_before: 0.10,
            landed_after: 0.85,
            median_before_ms: 2_000,
            median_after_ms: 40,
            examined: 1_200,
        }
    }

    #[test]
    fn a_clear_measurement_that_lands_is_written() {
        assert_eq!(outcome(good(), Bars::default()), Outcome::Accepted);
    }

    #[test]
    fn a_measurement_nobody_could_be_sure_of_is_not_written() {
        let figures = Figures {
            confidence: 0.004,
            ..good()
        };
        assert_eq!(outcome(figures, Bars::default()), Outcome::Uncertain);
    }

    /// The two halves of the bar, which ask different questions. One refuses a
    /// measurement that would make a film worse, whatever it scored. The other
    /// refuses one that improves a film and still leaves most of it missing the
    /// talking, because both readings being poor means the pairing is wrong
    /// rather than the timing.
    #[test]
    fn a_measurement_is_written_only_where_it_helps_and_lands() {
        let worse = Figures {
            landed_before: 0.9,
            landed_after: 0.5,
            ..good()
        };
        assert_eq!(outcome(worse, Bars::default()), Outcome::NoBetter);

        let barely = Figures {
            landed_before: 0.05,
            landed_after: 0.2,
            ..good()
        };
        assert_eq!(outcome(barely, Bars::default()), Outcome::NoBetter);
    }

    #[test]
    fn a_measurement_of_nothing_is_never_written() {
        let nothing = Figures {
            examined: 0,
            landed_after: 0.0,
            landed_before: 0.0,
            ..good()
        };
        assert_eq!(outcome(nothing, Bars::default()), Outcome::NoBetter);
    }

    /// The whole point of taking the figures apart from the measurement: the
    /// same reading decides differently at a different bar, so a run can be
    /// judged again at a proposed value without a film being read twice.
    #[test]
    fn the_same_reading_can_be_judged_at_another_bar() {
        let bars = Bars {
            threshold: 0.05,
            bar: 0.4,
        };
        assert_eq!(outcome(good(), bars), Outcome::Uncertain);

        let bars = Bars {
            threshold: 0.015,
            bar: 0.9,
        };
        assert_eq!(outcome(good(), bars), Outcome::NoBetter);
    }

    #[test]
    fn an_answer_outside_the_slack_is_wrong_however_confident_it_was() {
        let wanted = Wanted::Within(50);
        assert_eq!(verdict(Outcome::Accepted, wanted, Some(20)), Verdict::Right);
        assert_eq!(
            verdict(Outcome::Accepted, wanted, Some(4_000)),
            Verdict::Wrong
        );
        assert_eq!(
            verdict(Outcome::Uncertain, wanted, Some(20)),
            Verdict::Missed
        );
    }

    /// A file nothing could put right is judged on one thing only, which is
    /// whether it was left alone.
    #[test]
    fn writing_anything_to_a_file_that_cannot_be_corrected_is_wrong() {
        assert_eq!(
            verdict(Outcome::Accepted, Wanted::Refusal, None),
            Verdict::Wrong
        );
        assert_eq!(
            verdict(Outcome::Uncertain, Wanted::Refusal, None),
            Verdict::Right
        );
        assert_eq!(
            verdict(Outcome::NoBetter, Wanted::Refusal, None),
            Verdict::Right
        );
    }
}
