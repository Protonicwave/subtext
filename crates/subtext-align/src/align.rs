//! The answer, and how much of it to believe.

use subtext_core::{Correction, Cue};

use crate::correlate::{Correlator, Peak};
use crate::rate::RATES;
use crate::signal::{self, BIN_MS, Signal};

/// How far either way a track may be moved to make it fit, in milliseconds.
///
/// Two minutes. A file out by more than that is not this film's subtitle with a
/// different intro on the front of it, it is the wrong file, and the honest
/// answer is to decline rather than to return the best of a bad set. Narrowing
/// the search this way is also what makes it quick.
const LAG_WINDOW_MS: u32 = 120_000;

/// How sure the engine is, in two parts and one number.
///
/// Reported rather than acted on. What counts as sure enough is a judgement
/// about when to touch somebody's file, which belongs with whoever is deciding
/// to touch it and not with the arithmetic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Confidence {
    peak: f32,
    margin: f32,
}

impl Confidence {
    /// Nothing was measurable, which is not the same as measuring badly.
    pub const NONE: Self = Self {
        peak: 0.0,
        margin: 0.0,
    };

    /// How well the two signals agree where they agree best, from nothing to a
    /// perfect match.
    #[must_use]
    pub fn peak(self) -> f32 {
        self.peak
    }

    /// How far the answer stands above the next best explanation, from level
    /// with it to alone.
    ///
    /// This is the part that catches a subtitle belonging to a different film. A
    /// wrong pairing still correlates a little at every lag, because dialogue is
    /// spread through a film the same way wherever it came from, so it produces
    /// a shallow peak among many others of nearly the same size.
    #[must_use]
    pub fn margin(self) -> f32 {
        self.margin
    }

    /// The two parts as one number, for comparing against a threshold.
    ///
    /// Multiplied rather than averaged, so that a poor showing in either part
    /// cannot be made up for by the other. A tall peak with rivals beside it and
    /// a lonely shallow one are both doubtful, and an average would call one of
    /// them respectable.
    #[must_use]
    pub fn score(self) -> f32 {
        self.peak * self.margin
    }
}

/// What a track has to be put through to sit over its film, and how sure that is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment {
    correction: Correction,
    confidence: Confidence,
}

impl Alignment {
    #[must_use]
    pub fn correction(self) -> Correction {
        self.correction
    }

    #[must_use]
    pub fn confidence(self) -> Confidence {
        self.confidence
    }
}

/// The correction that best explains where the speech falls in terms of where
/// the cues claim it does.
///
/// The cues are taken as the file wrote them, so this measures a track against a
/// film rather than against its own last answer. Handing it cues that have
/// already been corrected would return the residual of that correction and
/// converge on nothing.
///
/// There is always an answer, because there is no case where declining is more
/// use to a caller than a measurement with the confidence attached to it. Cues
/// that cannot be correlated at all, being none of them, or a speech signal with
/// no shape, come back as the identity with [`Confidence::NONE`], which no
/// threshold worth having will accept.
#[must_use]
pub fn align(cues: &[Cue], speech: &Signal) -> Alignment {
    let nothing = Alignment {
        correction: Correction::IDENTITY,
        confidence: Confidence::NONE,
    };

    let window = (LAG_WINDOW_MS / BIN_MS) as usize;
    // The buffers are sized for the longest the cues can reach at any candidate,
    // so that the whole search runs through one set of them.
    let longest = RATES
        .iter()
        .map(|rate| signal::span(cues, *rate))
        .max()
        .unwrap_or(0);
    let Some(mut correlator) = Correlator::new(speech, longest, window) else {
        return nothing;
    };

    let mut candidate = Signal::from_cues(cues);
    let mut best: Option<(f64, Peak)> = None;
    for rate in RATES {
        candidate.rebuild(cues, rate);
        let Some(peak) = correlator.correlate(&candidate) else {
            continue;
        };
        // Strictly better, and the identity is tried first, so a track is only
        // given a stretch by a candidate that plainly explains it better than
        // leaving it alone does.
        if best.is_none_or(|(_, found)| peak.height > found.height) {
            best = Some((rate, peak));
        }
    }

    let Some((rate, peak)) = best else {
        return nothing;
    };

    let offset_ms = peak
        .lag
        .saturating_mul(isize::try_from(BIN_MS).unwrap_or(1));
    Alignment {
        correction: Correction::new(i32::try_from(offset_ms).unwrap_or(0), rate),
        confidence: confidence_of(&peak),
    }
}

fn confidence_of(peak: &Peak) -> Confidence {
    if peak.height <= 0.0 {
        return Confidence::NONE;
    }

    // How far the peak stands clear of its nearest rival, as a fraction of
    // itself. Scale free on purpose: what matters is that the answer is not one
    // of several equally good ones, and that question has the same answer
    // whether the film is loud or quiet.
    let margin = ((peak.height - peak.runner_up) / peak.height).clamp(0.0, 1.0);

    #[allow(clippy::cast_possible_truncation)]
    Confidence {
        peak: peak.height.clamp(0.0, 1.0) as f32,
        margin: margin as f32,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::cast_possible_truncation,
        clippy::expect_used,
        clippy::unwrap_used
    )]

    use super::{Confidence, align};
    use crate::rate::RATES;
    use crate::signal::{BIN_MS, Signal};
    use subtext_core::{Correction, Cue, Timestamp};

    /// A level the cases below have to fall either side of.
    ///
    /// Stated here so that the tests which have to be above it and the tests
    /// which have to be below it are talking about the same number, and no more
    /// than that. What these tests demonstrate is the ordering: a track that
    /// belongs to its film scores well clear of one that does not.
    ///
    /// It is not the number an application would compare against. Every signal
    /// here is built from cues, on both sides, and a pair like that scores far
    /// higher than a real subtitle measured against real audio does. Whoever is
    /// deciding to write to somebody's library sets their own value from films
    /// rather than from this.
    const DEFENSIBLE: f32 = 0.25;

    /// A film's worth of dialogue: a line every few seconds, in bursts with
    /// quiet between them, which is the shape a transcript actually has.
    fn dialogue(count: u32) -> Vec<Cue> {
        (0..count)
            .map(|at| {
                // Lines come in exchanges rather than evenly, so the signal has
                // something to line up on other than its own regularity.
                let gap = 1_400 + (at % 7) * 900;
                let start = 30_000 + at * gap;
                let length = 900 + (at % 5) * 400;
                Cue {
                    index: at + 1,
                    start: Timestamp::from_millis(start),
                    end: Timestamp::from_millis(start + length),
                    text: "line".to_owned(),
                    position: None,
                }
            })
            .collect()
    }

    /// What the film sounds like if the cues are right about it.
    fn speech_of(cues: &[Cue], correction: Correction) -> Signal {
        let moved: Vec<Cue> = cues
            .iter()
            .map(|cue| Cue {
                start: correction.apply(cue.start),
                end: correction.apply(cue.end),
                ..cue.clone()
            })
            .collect();
        Signal::from_cues(&moved)
    }

    /// Flips one bin in every `one_in`, which is what a speech detector getting
    /// it wrong here and there looks like.
    fn noisier(signal: &Signal, one_in: usize) -> Signal {
        let mut bins: Vec<bool> = signal.bins().to_vec();
        // A fixed pattern rather than a random one, so a failing test fails the
        // same way twice.
        for (at, bin) in bins.iter_mut().enumerate() {
            if at % one_in == 0 {
                *bin = !*bin;
            }
        }
        Signal::from_bins(bins)
    }

    #[test]
    fn a_known_offset_comes_back_within_one_bin() {
        let cues = dialogue(800);
        let truth = Correction::of_offset(2_500);
        let found = align(&cues, &speech_of(&cues, truth));

        let error = (found.correction().offset_ms() - truth.offset_ms()).unsigned_abs();
        assert!(error <= BIN_MS, "out by {error}ms");
        assert!(found.confidence().score() > DEFENSIBLE);
    }

    #[test]
    fn a_track_that_is_early_comes_back_early() {
        let cues = dialogue(800);
        let truth = Correction::of_offset(-4_000);
        let found = align(&cues, &speech_of(&cues, truth));

        let error = (found.correction().offset_ms() - truth.offset_ms()).unsigned_abs();
        assert!(error <= BIN_MS, "out by {error}ms");
    }

    #[test]
    fn a_track_that_needs_nothing_is_left_alone() {
        let cues = dialogue(800);
        let found = align(&cues, &speech_of(&cues, Correction::IDENTITY));

        assert!(found.correction().is_identity());
        assert!(found.confidence().score() > DEFENSIBLE);
    }

    #[test]
    fn a_known_rate_is_chosen_from_the_candidates() {
        let cues = dialogue(1_500);
        let truth = Correction::new(0, RATES[1]);
        let found = align(&cues, &speech_of(&cues, truth));

        assert!((found.correction().rate() - truth.rate()).abs() < f64::EPSILON);
    }

    #[test]
    fn a_rate_and_an_offset_are_recovered_together() {
        let cues = dialogue(1_500);
        let truth = Correction::new(-3_000, RATES[2]);
        let found = align(&cues, &speech_of(&cues, truth));
        let correction = found.correction();

        assert!((correction.rate() - truth.rate()).abs() < f64::EPSILON);

        // The residual across the whole film rather than the offset alone,
        // since a rate and an offset trade against each other and only the
        // combination has to be right.
        for cue in &cues {
            let error = i64::from(correction.apply(cue.start).millis())
                - i64::from(truth.apply(cue.start).millis());
            assert!(error.abs() < 100, "out by {error}ms at {}", cue.start);
        }
    }

    #[test]
    fn a_signal_that_is_wrong_here_and_there_is_still_read_correctly() {
        let cues = dialogue(800);
        let truth = Correction::of_offset(1_800);
        let speech = noisier(&speech_of(&cues, truth), 20);
        let found = align(&cues, &speech);

        let error = (found.correction().offset_ms() - truth.offset_ms()).unsigned_abs();
        assert!(error <= BIN_MS, "out by {error}ms");
        assert!(found.confidence().score() > DEFENSIBLE);
    }

    #[test]
    fn enough_noise_and_the_engine_stops_being_sure() {
        let cues = dialogue(800);
        let speech = speech_of(&cues, Correction::of_offset(1_800));

        // Confidence has to fall as the signal is spoiled, and end below what
        // anybody would act on. Where exactly it crosses is not the claim.
        let mut previous = f32::MAX;
        for one_in in [40, 10, 5, 2] {
            let score = align(&cues, &noisier(&speech, one_in)).confidence().score();
            assert!(score < previous, "confidence rose at one in {one_in}");
            previous = score;
        }
        assert!(previous < DEFENSIBLE, "still {previous} at one bin in two");
    }

    #[test]
    fn a_subtitle_for_a_different_film_is_not_confident() {
        let ours = dialogue(800);
        // A different film: different exchanges, different lengths, the same
        // general shape. This is the case the whole confidence figure exists
        // for, and the number it must come in under is stated above.
        let theirs: Vec<Cue> = dialogue(800)
            .into_iter()
            .enumerate()
            .map(|(at, cue)| {
                let start = 12_000 + (at as u32) * (2_600 + (at as u32 % 11) * 700);
                Cue {
                    start: Timestamp::from_millis(start),
                    end: Timestamp::from_millis(start + 1_100 + (at as u32 % 3) * 600),
                    ..cue
                }
            })
            .collect();

        // An order of magnitude below the threshold and not merely under it,
        // because a case this common has to fail with room to spare rather than
        // by a whisker that a different film could close.
        let score = align(&ours, &Signal::from_cues(&theirs))
            .confidence()
            .score();
        assert!(
            score < DEFENSIBLE / 10.0,
            "scored {score} against another film"
        );
    }

    #[test]
    fn dialogue_that_repeats_within_the_window_is_not_claimed_to_be_certain() {
        // Every line falls on the same pattern once a minute, so moving the
        // track a minute explains the film as well as leaving it where it is.
        // The peak is genuinely tall and genuinely ambiguous, and the margin is
        // the part that has to notice.
        let cues: Vec<Cue> = (0..600)
            .map(|at| {
                let start = 20_000 + (at / 10) * 60_000 + (at % 10) * 4_000;
                Cue {
                    index: at + 1,
                    start: Timestamp::from_millis(start),
                    end: Timestamp::from_millis(start + 1_500),
                    text: "line".to_owned(),
                    position: None,
                }
            })
            .collect();

        let found = align(&cues, &speech_of(&cues, Correction::of_offset(2_000)));
        assert!(found.confidence().peak() > 0.9);
        assert!(found.confidence().score() < DEFENSIBLE);
    }

    #[test]
    fn nothing_to_measure_comes_back_as_nothing_measured() {
        let cues = dialogue(200);
        let speech = speech_of(&cues, Correction::IDENTITY);

        for found in [
            align(&[], &speech),
            align(&cues, &Signal::from_bins(Vec::new())),
            align(&cues, &Signal::from_bins(vec![true; 200_000])),
            align(&cues, &Signal::from_bins(vec![false; 200_000])),
        ] {
            assert!(found.correction().is_identity());
            assert_eq!(found.confidence(), Confidence::NONE);
            assert!(found.confidence().score() < f32::EPSILON);
        }
    }

    #[test]
    fn a_single_cue_is_answered_like_any_other_track() {
        let cues = dialogue(1);
        let found = align(&cues, &speech_of(&cues, Correction::of_offset(1_000)));

        // One line against one burst of speech lines up perfectly and means
        // almost nothing, and the engine says the former because the latter is
        // not a question about signals. Whether a track carries enough lines to
        // be worth correlating is settled by whoever asks, before any of this
        // runs.
        //
        // The line is put in the right place rather than the offset holding a
        // particular number: with a single cue, a small stretch and a slightly
        // smaller shift explain the film exactly as well as the shift alone, and
        // there is nothing in one line to choose between them. It takes a
        // transcript to tell a rate from an offset, which is why nothing here
        // asserts on the two separately.
        let moved = found.correction().apply(cues[0].start).millis();
        assert!(moved.abs_diff(31_000) <= BIN_MS, "landed at {moved}ms");
    }

    #[test]
    fn cues_running_past_the_end_of_the_film_are_measured_against_what_there_is() {
        let cues = dialogue(800);
        let truth = Correction::of_offset(1_500);
        let whole = speech_of(&cues, truth);

        // A film whose audio stops two thirds of the way through, which is what
        // a truncated download sounds like.
        let mut cut = whole.bins().to_vec();
        cut.truncate(whole.len() * 2 / 3);
        let found = align(&cues, &Signal::from_bins(cut));

        let error = (found.correction().offset_ms() - truth.offset_ms()).unsigned_abs();
        assert!(error <= BIN_MS, "out by {error}ms");
    }

    #[test]
    fn a_shift_wider_than_the_search_is_declined_rather_than_guessed_at() {
        let cues = dialogue(800);
        // Three minutes, past the two the window covers. The engine has no
        // business finding this and every business not being sure about
        // whatever it does find instead.
        let speech = speech_of(&cues, Correction::of_offset(180_000));
        assert!(align(&cues, &speech).confidence().score() < DEFENSIBLE);
    }
}
