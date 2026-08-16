//! Manufacturing cases whose right answer is known.
//!
//! A film that carries its own text subtitle track carries timings written
//! against those exact frames. Taking them as truth and then moving them by a
//! known amount produces a file whose correct answer is known exactly, which is
//! the whole trick: no case has to be labelled by hand, and every film in
//! somebody's library is a test.
//!
//! Truth does not have to be perfect for this to work. An embedded track that
//! was itself a little out shifts the case and its answer by the same amount,
//! and what is measured is the recovery of the perturbation rather than the
//! absolute position of the lines.
//!
//! Every case is built by moving the truth through a map and keeping, beside
//! each moved line, the moment it actually belongs at. The error of an answer is
//! then read off directly: apply the correction to the line as the file wrote
//! it, and compare it against where the film says that line is spoken. Nothing
//! here has to invert a correction to work out what the answer should have been.

use subtext_align::RATES;
use subtext_core::{Cue, Timestamp};

/// The shifts a mistimed file is out by.
///
/// Half a second is the smallest anybody complains about, a couple of seconds
/// is the ordinary case of a different intro, and a minute is a file from a
/// release with a studio logo on the front of it. The largest sits well inside
/// the two minute window the engine searches, since a case outside that window
/// would be measuring the window rather than the engine.
const OFFSETS_MS: [i32; 8] = [-60_000, -10_000, -2_000, -500, 500, 2_000, 10_000, 60_000];

/// A stretch and a shift together, which is what a file taken from another
/// release with a different framerate and a different intro carries.
const BOTH: [(usize, i32); 2] = [(1, 2_500), (2, -4_000)];

/// How far into a film a break is cut, and how much time it inserts.
///
/// Ninety seconds at two fifths of the way through, which is roughly what an
/// advertisement break in a recorded broadcast does to a film. No single offset
/// and rate can answer this, which is the point of including it.
const SPLICE_AT: f64 = 0.4;
const SPLICE_MS: u32 = 90_000;

/// How far out an accepted answer may be and still be right, for a case that
/// only needs shifting.
///
/// Fifty milliseconds, from the table in the plan. It is under two frames and
/// below anything anybody watching can tell.
const SHIFT_SLACK_MS: i64 = 50;

/// The same for a case that needs stretching.
///
/// A stretch is recovered from a list of fixed candidates, so a right answer is
/// exact in the rate and carries whatever the offset rounded to. A hundred
/// milliseconds covers that at the far end of a long film.
const STRETCH_SLACK_MS: i64 = 100;

/// What kind of file a case stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    /// Out by the same amount from beginning to end.
    Shift,
    /// Out by a growing amount, because the release it was written against ran
    /// at a different framerate.
    Stretch,
    /// Both at once.
    Both,
    /// A film with time cut into the middle of it, which no single correction
    /// can answer.
    Splice,
    /// A subtitle belonging to another film altogether.
    Mismatched,
}

impl Family {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Shift => "shift",
            Self::Stretch => "stretch",
            Self::Both => "both",
            Self::Splice => "splice",
            Self::Mismatched => "mismatched",
        }
    }
}

/// What the right ending for a case is.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Wanted {
    /// A correction that puts every line within this many milliseconds of where
    /// the film speaks it.
    Within(i64),
    /// A refusal. Nothing that could be written here would be worth writing.
    Refusal,
}

impl Wanted {
    pub(crate) fn slack(self) -> Option<i64> {
        match self {
            Self::Within(slack) => Some(slack),
            Self::Refusal => None,
        }
    }
}

/// One file to measure, and what measuring it should come to.
#[derive(Debug)]
pub(crate) struct Case {
    pub(crate) family: Family,
    /// What was done to the truth, in the terms somebody would describe a file
    /// in.
    pub(crate) about: String,
    /// The lines as the manufactured file wrote them.
    pub(crate) authored: Vec<Cue>,
    /// Where each of those lines is actually spoken, parallel to `authored`.
    /// Empty where the case has no right answer to be measured against.
    pub(crate) belongs_ms: Vec<u32>,
    /// Which film's audio the case is measured against.
    pub(crate) film: usize,
    pub(crate) wanted: Wanted,
}

/// Every case a film's truth can be turned into, apart from the mismatched one,
/// which needs a second film.
pub(crate) fn manufacture(film: usize, truth: &[Cue]) -> Vec<Case> {
    let mut cases = Vec::new();

    for offset in OFFSETS_MS {
        let (authored, belongs_ms) = from_truth(truth, |at| {
            u32::try_from(i64::from(at) - i64::from(offset)).ok()
        });
        cases.push(Case {
            family: Family::Shift,
            about: format!("out by {}", seconds(offset)),
            authored,
            belongs_ms,
            film,
            wanted: Wanted::Within(SHIFT_SLACK_MS),
        });
    }

    for rate in RATES {
        let (authored, belongs_ms) = from_truth(truth, |at| millis_of(f64::from(at) / rate));
        cases.push(Case {
            family: Family::Stretch,
            about: format!("stretched by {rate:.4}"),
            authored,
            belongs_ms,
            film,
            // The identity is among the candidates, so one of these is a file
            // that needs nothing done to it. Leaving it alone is an answer
            // within any slack, and writing a stretch to it is not.
            wanted: Wanted::Within(STRETCH_SLACK_MS),
        });
    }

    for (candidate, offset) in BOTH {
        let Some(rate) = RATES.get(candidate).copied() else {
            continue;
        };
        let (authored, belongs_ms) = from_truth(truth, |at| {
            millis_of((f64::from(at) - f64::from(offset)) / rate)
        });
        cases.push(Case {
            family: Family::Both,
            about: format!("stretched by {rate:.4} and out by {}", seconds(offset)),
            authored,
            belongs_ms,
            film,
            wanted: Wanted::Within(STRETCH_SLACK_MS),
        });
    }

    if let Some(case) = spliced(film, truth) {
        cases.push(case);
    }

    cases
}

/// A film with time cut into the middle of it.
///
/// The lines before the break are right where they were and the lines after it
/// are early by the length of the break, so a single correction can answer one
/// half or the other and never both. A refusal is therefore the right ending
/// today, and this case is here to say how often the engine gives one and what
/// it does when it does not.
fn spliced(film: usize, truth: &[Cue]) -> Option<Case> {
    let last = truth.iter().map(|cue| cue.end.millis()).max()?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let cut = (f64::from(last) * SPLICE_AT) as u32;
    if cut <= SPLICE_MS {
        return None;
    }

    let (authored, belongs_ms) = from_truth(truth, |at| {
        if at < cut {
            Some(at)
        } else {
            at.checked_sub(SPLICE_MS)
        }
    });
    Some(Case {
        family: Family::Splice,
        about: format!(
            "{} inserted at {}",
            seconds(i32::try_from(SPLICE_MS).unwrap_or(i32::MAX)),
            seconds(i32::try_from(cut).unwrap_or(i32::MAX))
        ),
        authored,
        belongs_ms,
        film,
        wanted: Wanted::Refusal,
    })
}

/// One film's truth measured against another film's audio.
///
/// Nothing is done to the timings. They are correct, for a film this is not,
/// and the only right ending is to say so.
pub(crate) fn mismatched(truth: &[Cue], against: usize, title: &str) -> Case {
    let (authored, _) = from_truth(truth, Some);
    Case {
        family: Family::Mismatched,
        about: format!("against {title}"),
        authored,
        // There is no moment any of these lines belongs at in that film, so
        // there is no error to measure. What is being asked is only whether it
        // was refused.
        belongs_ms: Vec::new(),
        film: against,
        wanted: Wanted::Refusal,
    }
}

/// The truth moved through `moved`, with the moment each line belongs at kept
/// beside it.
///
/// A line the map puts before the beginning of the file is dropped rather than
/// held at zero. Clamping would pile the opening lines on top of each other and
/// manufacture a case with an answer nothing could recover, which would be
/// measuring the manufacture rather than the engine.
fn from_truth(truth: &[Cue], moved: impl Fn(u32) -> Option<u32>) -> (Vec<Cue>, Vec<u32>) {
    let mut authored = Vec::with_capacity(truth.len());
    let mut belongs = Vec::with_capacity(truth.len());

    for cue in truth {
        let (Some(start), Some(end)) = (moved(cue.start.millis()), moved(cue.end.millis())) else {
            continue;
        };
        authored.push(Cue {
            index: u32::try_from(authored.len() + 1).unwrap_or(u32::MAX),
            start: Timestamp::from_millis(start),
            end: Timestamp::from_millis(end.max(start)),
            ..cue.clone()
        });
        belongs.push(cue.start.millis());
    }

    (authored, belongs)
}

/// A count of milliseconds as a position in a film, or nothing where it is not
/// one.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn millis_of(at: f64) -> Option<u32> {
    let rounded = at.round();
    if rounded < 0.0 || rounded > f64::from(u32::MAX) {
        return None;
    }
    Some(rounded as u32)
}

/// A duration in the terms a person would say it out loud.
fn seconds(millis: i32) -> String {
    format!("{:+.1}s", f64::from(millis) / 1_000.0)
}

#[cfg(test)]
mod tests {
    // A case the manufacture cannot build is the failure these tests look for,
    // and stopping there says so more plainly than an assertion would.
    #![allow(clippy::expect_used)]

    use super::{
        Family, OFFSETS_MS, SPLICE_MS, Wanted, from_truth, manufacture, millis_of, seconds,
    };
    use subtext_align::RATES;
    use subtext_core::{Correction, Cue, Timestamp};

    /// A film's worth of dialogue, in exchanges rather than at a fixed spacing.
    fn truth(count: u32) -> Vec<Cue> {
        (0..count)
            .map(|at| {
                let start = 30_000 + at * (1_800 + (at % 7) * 900);
                Cue {
                    index: at + 1,
                    start: Timestamp::from_millis(start),
                    end: Timestamp::from_millis(start + 1_200),
                    text: "line".to_owned(),
                    position: None,
                }
            })
            .collect()
    }

    /// How far a correction leaves a case's lines from where they belong, at
    /// the worst line in it.
    fn worst_error(case: &super::Case, answer: Correction) -> i64 {
        case.authored
            .iter()
            .zip(&case.belongs_ms)
            .map(|(cue, belongs)| i64::from(answer.apply(cue.start).millis()) - i64::from(*belongs))
            .map(i64::abs)
            .max()
            .unwrap_or(0)
    }

    fn case_about<'a>(cases: &'a [super::Case], about: &str) -> &'a super::Case {
        cases
            .iter()
            .find(|case| case.about == about)
            .expect("the case that was asked for")
    }

    /// The property every manufactured case rests on: the perturbation a case
    /// was built from, applied to the file it produced, gives the truth back.
    /// Without this the harness would be measuring its own arithmetic.
    #[test]
    fn the_answer_a_case_was_built_from_recovers_its_truth() {
        let cases = manufacture(0, &truth(400));

        for offset in OFFSETS_MS {
            let case = case_about(&cases, &format!("out by {}", seconds(offset)));
            let error = worst_error(case, Correction::of_offset(offset));
            assert_eq!(error, 0, "{} was out by {error}ms", case.about);
        }

        for rate in RATES {
            let case = case_about(&cases, &format!("stretched by {rate:.4}"));
            let error = worst_error(case, Correction::new(0, rate));
            // A millisecond, which is what the truth being rounded once on the
            // way out and once on the way back costs. Nothing perceives it and
            // no slack in this harness is near it.
            assert!(error <= 1, "{} was out by {error}ms", case.about);
        }
    }

    #[test]
    fn every_family_a_film_can_be_turned_into_is_manufactured() {
        let cases = manufacture(0, &truth(400));

        assert_eq!(
            cases
                .iter()
                .filter(|case| case.family == Family::Shift)
                .count(),
            OFFSETS_MS.len()
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.family == Family::Stretch)
                .count(),
            RATES.len()
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.family == Family::Splice)
                .count(),
            1
        );
    }

    /// A film with time cut into it is early after the break and untouched
    /// before it, which is what makes it unanswerable by one correction.
    #[test]
    fn a_splice_moves_the_second_half_and_leaves_the_first() {
        let truth = truth(400);
        let case = manufacture(0, &truth)
            .into_iter()
            .find(|case| case.family == Family::Splice)
            .expect("a film long enough to cut a break into");

        assert!(matches!(case.wanted, Wanted::Refusal));

        let moved = case
            .authored
            .iter()
            .zip(&case.belongs_ms)
            .filter(|(cue, belongs)| cue.start.millis() != **belongs)
            .count();
        assert!(moved > 0, "nothing was moved");
        assert!(moved < case.authored.len(), "everything was moved");

        for (cue, belongs) in case.authored.iter().zip(&case.belongs_ms) {
            let away = belongs - cue.start.millis();
            assert!(away == 0 || away == SPLICE_MS, "moved by {away}ms");
        }
    }

    /// A shift large enough to push the opening lines in front of the file
    /// drops them, so that the case does not carry lines piled at zero with no
    /// answer that could recover them.
    #[test]
    fn lines_a_shift_would_push_before_the_start_are_dropped() {
        let truth = truth(400);
        let (authored, belongs) =
            from_truth(&truth, |at| u32::try_from(i64::from(at) - 40_000).ok());

        assert!(authored.len() < truth.len());
        assert_eq!(authored.len(), belongs.len());
        assert!(authored.iter().all(|cue| cue.start.millis() > 0));
    }

    #[test]
    fn a_line_a_map_cannot_place_is_not_a_line() {
        assert_eq!(millis_of(-1.0), None);
        assert_eq!(millis_of(f64::from(u32::MAX) + 1.0), None);
        assert_eq!(millis_of(1_499.6), Some(1_500));
    }
}
