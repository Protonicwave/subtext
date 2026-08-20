//! The answer, and how much of it to believe.

use subtext_core::{Correction, Cue, Timestamp};

use crate::landing::{Landing, landing_of};
use crate::local::{Chunks, Fit};
use crate::misfit::{Misfit, misfit_of};
use crate::rate;
use crate::signal::{self, BIN_MS, Signal};

use crate::correlate::Correlator;

/// How far either way a track may be moved to make it fit, in milliseconds.
///
/// Two minutes. A file out by more than that is not this film's subtitle with a
/// different intro on the front of it, it is the wrong file, and the honest
/// answer is to decline rather than to return the best of a bad set. Narrowing
/// the search this way is also what makes it quick.
const LAG_WINDOW_MS: u32 = 120_000;

/// How many explanations of the film are measured against it in full.
///
/// The coarse pass ranks its candidates by the height of a peak, and the height
/// of a peak is precisely the thing that cannot tell a stretch from a shift.
/// Carrying only the winner forward would confirm the coarse ranking rather than
/// check it, which is the mistake all of this exists to correct, so several go
/// through. Six covers the five distinct ratios with room for a second reading
/// of one of them, and costs a fraction of the time the audio behind it did.
const CARRIED: usize = 6;

/// How far apart two answers have to leave the lines to be different answers, in
/// milliseconds.
///
/// A quarter of a second, somewhere in the film. Two candidates often converge:
/// a stretch a thousandth away from the truth is measured across the film and
/// comes back naming the truth, so the film has two candidates saying the same
/// thing. That is agreement and not competition, and counting it as competition
/// would report a film explained twice over as a film nobody could explain.
const APART_MS: f64 = 250.0;

/// How sure the engine is, in three parts and one number.
///
/// Reported rather than acted on. What counts as sure enough is a judgement
/// about when to touch somebody's file, which belongs with whoever is deciding
/// to touch it and not with the arithmetic.
///
/// The parts are what the film said rather than what the correlation scored.
/// Every one of them comes from measuring the answer against stretches of the
/// film it claims to explain, which is why none of them is the peak that
/// produced the answer in the first place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Confidence {
    /// What the film's stretches said about the answer, which is where every
    /// part of this but the separation is read from.
    fit: Fit,
    separation: f32,
}

impl Confidence {
    /// Nothing was measurable, which is not the same as measuring badly.
    pub const NONE: Self = Self {
        fit: Fit {
            slope: 0.0,
            intercept_ms: 0.0,
            agreed: 0,
            across: 0,
            spread_ms: f32::MAX,
        },
        separation: 0.0,
    };

    /// How many stretches of the film agreed with the answer.
    ///
    /// The film is cut into stretches and each is asked how far its lines still
    /// have to move once the answer is applied. This is how many of them said
    /// what the answer says they should, out of [`Confidence::across`].
    ///
    /// The count rather than the share, because it is the count that can be put
    /// to somebody: a share is a verdict on the answer and a count is what the
    /// film said.
    #[must_use]
    pub fn agreed(self) -> u32 {
        self.fit.agreed
    }

    /// How many stretches the film was measured in.
    #[must_use]
    pub fn across(self) -> u32 {
        self.fit.across
    }

    /// How far the middle stretch sits from the answer, in milliseconds.
    ///
    /// The middle rather than the average, for the same reason the line through
    /// the stretches is a middle: a handful of stretches that found the wrong
    /// thing would otherwise decide a figure that is meant to describe the rest.
    #[must_use]
    pub fn spread_ms(self) -> f32 {
        self.fit.spread_ms
    }

    /// How much of the film agrees with the answer, from none of it to all.
    #[must_use]
    pub fn agreement(self) -> f32 {
        self.fit.agreement()
    }

    /// How closely the stretches that agree sit to the answer, from barely to
    /// exactly.
    #[must_use]
    pub fn tightness(self) -> f32 {
        self.fit.tightness()
    }

    /// How far the answer stands above the next explanation that is not the same
    /// answer said differently, from level with it to alone.
    ///
    /// This is the part that catches a subtitle belonging to a different film. A
    /// wrong pairing still correlates a little everywhere, because dialogue is
    /// spread through a film the same way wherever it came from, so it produces
    /// several explanations that each describe about as little of the film as
    /// the others.
    #[must_use]
    pub fn separation(self) -> f32 {
        self.separation
    }

    /// The three parts as one number, for comparing against a threshold.
    ///
    /// Multiplied rather than averaged, so that a poor showing in any part
    /// cannot be made up for by the others. An answer most of the film ignores,
    /// an answer the film agrees with loosely, and an answer with an equally
    /// good rival beside it are all doubtful, and an average would call some of
    /// them respectable.
    #[must_use]
    pub fn score(self) -> f32 {
        self.fit.score() * self.separation
    }
}

/// What a track has to be put through to sit over its film, and how sure that is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment {
    correction: Correction,
    confidence: Confidence,
    landing: Landing,
    as_written: Landing,
    misfit: Option<Misfit>,
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

    /// How the lines would sit against the talking with this correction
    /// applied.
    ///
    /// The confidence above says how much of the film agrees about where its
    /// lines belong. This says whether they then arrive when somebody speaks,
    /// which is a different question and the one somebody watching would ask. A
    /// film can agree with itself about a wrong answer, and does whenever a
    /// subtitle is for a different cut of the same picture.
    #[must_use]
    pub fn landing(self) -> Landing {
        self.landing
    }

    /// How the lines sit against the talking with the file left as it is.
    ///
    /// The figure above means very little on its own, because no film scores
    /// well on it and none was ever going to. It means a great deal beside this
    /// one, which is why the two always travel together.
    #[must_use]
    pub fn as_written(self) -> Landing {
        self.as_written
    }

    /// A way this film cannot be described by one correction, where the film
    /// showed one.
    ///
    /// There is always a correction, and for these two shapes of file there is
    /// always a bad one. A film with time cut into it has a best answer for one
    /// half and the wrong answer for the other. A subtitle for a different cut
    /// has a best answer for the scenes the two versions share and nothing to
    /// say about the rest. Both can score well, because most of the film really
    /// does line up, and neither is a correction anybody should be handed.
    ///
    /// Reported rather than acted on, like everything else here.
    #[must_use]
    pub fn misfit(self) -> Option<Misfit> {
        self.misfit
    }
}

/// One explanation of the film, as the coarse pass left it.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    rate: f64,
    /// The shift, in bins, that the whole-film correlation found for that rate.
    lag: isize,
    /// What that correlation scored, which decides only which candidates go
    /// through to be measured properly.
    height: f64,
}

/// One explanation of the film, once the film has had its say about it.
#[derive(Clone, Copy, Debug)]
struct Measured {
    correction: Correction,
    score: f32,
    /// The candidate this came from, and the line the film's stretches came to
    /// about it. Both are kept so that the winner can be put back to the film
    /// once, to be read for the ways the film does not fit it. The settled
    /// correction above cannot stand in for them: the stretches were measured
    /// against the candidate, and what each of them is left needing is measured
    /// against this line.
    from: Candidate,
    fit: Fit,
}

/// The correction that best explains where the speech falls in terms of where
/// the cues claim it does.
///
/// The cues are taken as the file wrote them, so this measures a track against a
/// film rather than against its own last answer. Handing it cues that have
/// already been corrected would return the residual of that correction and
/// converge on nothing.
///
/// Two stages. A correlation over the whole film at each candidate stretch,
/// which is quick and which narrows the field; then, for each of the few that
/// survive, the film in stretches, each asked what the candidate still leaves it
/// needing, and a line fitted to the answers. The second stage is what picks the
/// winner, because the first cannot: a stretch and a shift trade against each
/// other and the tallest peak is as often the wrong pair as the right one.
///
/// There is always an answer, because there is no case where declining is more
/// use to a caller than a measurement with the confidence attached to it. Cues
/// that cannot be correlated at all, being none of them, or a speech signal with
/// no shape, come back as the identity with [`Confidence::NONE`], which no
/// threshold worth having will accept. So does a film that agrees with no
/// explanation of itself.
#[must_use]
pub fn align(cues: &[Cue], speech: &Signal) -> Alignment {
    // Nothing measured, in every part of the answer. Where there is no shape to
    // correlate there is no shape to land on either, so reporting how the lines
    // sit would be reporting the same absence twice in a form that looks like a
    // figure.
    let nothing = Alignment {
        correction: Correction::IDENTITY,
        confidence: Confidence::NONE,
        landing: Landing::NONE,
        as_written: Landing::NONE,
        misfit: None,
    };

    // Nothing settled, which is not nothing measured. The film was read and no
    // explanation of it held up, so how the lines sit as written is still worth
    // saying and the answer is to leave them alone.
    let unsettled = || Alignment {
        correction: Correction::IDENTITY,
        confidence: Confidence::NONE,
        landing: landing_of(cues, speech, Correction::IDENTITY),
        as_written: landing_of(cues, speech, Correction::IDENTITY),
        // Nothing explained the film, so there is no answer for the film to be
        // read as not fitting. Naming a shape here would be describing how badly
        // an answer nobody is being offered fails.
        misfit: None,
    };

    let window = (LAG_WINDOW_MS / BIN_MS) as usize;
    // The buffers are sized for the longest the cues can reach at any candidate,
    // so that the whole coarse pass runs through one set of them.
    let longest = rate::distinct()
        .map(|rate| signal::span(cues, rate))
        .max()
        .unwrap_or(0);
    let Some(mut correlator) = Correlator::new(speech, longest, window) else {
        return nothing;
    };

    let mut track = Signal::from_cues(cues);
    let carried = coarse(&mut correlator, &mut track, cues);
    if carried.is_empty() {
        return nothing;
    }

    let Some(mut chunks) = Chunks::over(speech.len()) else {
        return nothing;
    };

    #[allow(clippy::cast_precision_loss)]
    let film_ms = (speech.len() * BIN_MS as usize) as f64;
    let mut measured: Vec<Measured> = Vec::with_capacity(carried.len());
    for candidate in carried {
        track.rebuild(cues, candidate.rate);
        let Some(fit) = chunks.fit(speech, &track, candidate.lag) else {
            continue;
        };
        let Some(correction) = settle(candidate, fit, film_ms) else {
            continue;
        };
        measured.push(Measured {
            correction,
            score: fit.score(),
            from: candidate,
            fit,
        });
    }

    measured.sort_by(|one, other| other.score.total_cmp(&one.score));
    let Some(best) = measured.first().copied() else {
        return unsettled();
    };
    if best.score <= 0.0 {
        return unsettled();
    }

    // The winner put back to the film once, and every stretch of it asked not
    // where its lines belong but whether the answer explains it at all. This is
    // the only pass that asks, so it runs after the field has been settled rather
    // than for each candidate: on a film that fits, nothing comes of it, and on a
    // film that does not, what comes of it is the difference between a file
    // somebody could correct and a file for a different picture.
    track.rebuild(cues, best.from.rate);
    let misfit = misfit_of(chunks.readings(speech, &track, best.from.lag, best.fit));

    Alignment {
        correction: best.correction,
        confidence: Confidence {
            fit: best.fit,
            separation: separation_of(&measured, film_ms),
        },
        landing: landing_of(cues, speech, best.correction),
        as_written: landing_of(cues, speech, Correction::IDENTITY),
        misfit,
    }
}

/// The explanations worth measuring properly.
///
/// One correlation over the whole film for each distinct stretch, and both of
/// the two lags it can offer: the tallest, and the best anywhere clear of it.
/// The second is there because a film whose dialogue falls into a repeating
/// pattern has two answers the arithmetic cannot choose between, and the film
/// itself can.
fn coarse(correlator: &mut Correlator, track: &mut Signal, cues: &[Cue]) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = Vec::new();
    for rate in rate::distinct() {
        track.rebuild(cues, rate);
        let Some(peak) = correlator.against(track.bins()) else {
            continue;
        };
        found.push(Candidate {
            rate,
            lag: peak.lag,
            height: peak.height,
        });
        if let Some((lag, height)) = peak.rival {
            found.push(Candidate { rate, lag, height });
        }
    }

    found.sort_by(|one, other| other.height.total_cmp(&one.height));

    // Leaving the track alone goes through whatever it scored. Most files need
    // no stretch at all, and a coarse ranking that put the identity seventh
    // would be the one thing this stage cannot recover from: a stretch it never
    // compared against no stretch.
    if let Some(at) = found
        .iter()
        .position(|candidate| (candidate.rate - 1.0).abs() < f64::EPSILON)
        .filter(|at| *at >= CARRIED)
    {
        found.swap(CARRIED - 1, at);
    }

    found.truncate(CARRIED);
    found
}

/// The correction a candidate comes to once the film has corrected it.
///
/// A candidate maps an authored moment to a film moment. The line fitted across
/// the chunks says how far that mapping is still out and how fast that error
/// grows, so composing the two gives the mapping the film actually wants, and
/// composing them is all this does.
///
/// The stretch that falls out is then named, and a stretch with no name is no
/// answer. Nothing where it cannot be named: a ratio that is not one of the
/// framerate conversions is far likelier to be a splice, a different cut or the
/// wrong film than a real stretch, and a value nobody could have chosen by hand
/// is one nobody can check either.
fn settle(candidate: Candidate, fit: Fit, film_ms: f64) -> Option<Correction> {
    #[allow(clippy::cast_precision_loss)]
    let offset_ms = (candidate.lag * isize::try_from(BIN_MS).unwrap_or(1)) as f64;

    let stretched = candidate.rate * (1.0 + fit.slope);
    let moved = offset_ms.mul_add(1.0 + fit.slope, fit.intercept_ms);
    let named = rate::nearest(stretched, film_ms)?;

    // Naming the stretch moves it a little, so the shift takes up the
    // difference, measured at the middle of the film. Keeping the middle where
    // the fit put it spreads the change the naming makes across both halves
    // rather than leaving it all at one end.
    let middle = (film_ms / 2.0 - moved) / stretched;
    let settled = (stretched - named).mul_add(middle, moved);

    #[allow(clippy::cast_possible_truncation)]
    Some(Correction::new(
        settled.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
        named,
    ))
}

/// How far the winner stands above the best explanation that is not itself.
///
/// Candidates converge. A stretch a thousandth away from the truth is measured
/// across the film and comes back naming the truth, so two of them arrive at the
/// same correction by different routes. That is the film agreeing with itself,
/// and reporting it as a rival would say a film explained twice over could not
/// be explained at all. So the runner up is the best candidate that actually
/// puts the lines somewhere else.
///
/// A winner with no rival at all stands alone, and says so. Nothing else
/// explained the film, which is the strongest thing the parts can say and the
/// rarest.
fn separation_of(measured: &[Measured], film_ms: f64) -> f32 {
    let Some(best) = measured.first() else {
        return 0.0;
    };

    let rival = measured
        .iter()
        .skip(1)
        .find(|other| apart(best.correction, other.correction, film_ms));
    let Some(rival) = rival else {
        return 1.0;
    };

    if best.score <= 0.0 {
        return 0.0;
    }
    ((best.score - rival.score) / best.score).clamp(0.0, 1.0)
}

/// Whether two corrections put the lines in different places anywhere in the
/// film.
///
/// Both are straight lines, so the furthest they ever get from each other is at
/// one end or the other and there is nowhere else to look.
fn apart(one: Correction, other: Correction, film_ms: f64) -> bool {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let end = film_ms.clamp(0.0, f64::from(u32::MAX)) as u32;
    [0, end].iter().any(|at| {
        let at = Timestamp::from_millis(*at);
        let away = i64::from(one.apply(at).millis()) - i64::from(other.apply(at).millis());
        #[allow(clippy::cast_precision_loss)]
        let away = away.abs() as f64;
        away > APART_MS
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::cast_possible_truncation,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used
    )]

    use super::{Confidence, align};
    use crate::misfit::Misfit;
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
    ///
    /// Scattered rather than at a fixed spacing. Flipping every second bin is a
    /// transformation and not noise: it leaves the film's shape exactly where it
    /// was, and a correlation would read straight through it. The pattern is
    /// fixed all the same, so that a failing test fails the same way twice.
    fn noisier(signal: &Signal, one_in: u64) -> Signal {
        let mut bins: Vec<bool> = signal.bins().to_vec();
        for (at, bin) in bins.iter_mut().enumerate() {
            let scattered = (at as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 33;
            if scattered.is_multiple_of(one_in) {
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

    /// The same film captioned twice, once for somebody who can hear it and
    /// once for somebody who cannot. The second carries the first's lines and a
    /// description of every sound between them, and the two have to be measured
    /// to the same answer, because they describe the same film.
    ///
    /// The descriptions sit in the quiet, which is where the sounds they caption
    /// happen and where a detector looking for voices finds nothing. Fed in,
    /// they pull the answer towards a film that talks in its own gaps.
    #[test]
    fn a_track_written_for_the_hearing_impaired_is_measured_like_the_plain_one() {
        let plain = dialogue(800);
        let speech = speech_of(&plain, Correction::of_offset(2_500));

        let mut captioned = plain.clone();
        for pair in plain.windows(2) {
            let (before, after) = (&pair[0], &pair[1]);
            let quiet = after.start.millis().saturating_sub(before.end.millis());
            if quiet < 1_200 {
                continue;
            }
            let at = before.end.millis() + quiet / 3;
            captioned.push(Cue {
                index: 0,
                start: Timestamp::from_millis(at),
                end: Timestamp::from_millis(at + 900),
                text: "[DOOR SLAMS]".to_owned(),
                position: None,
            });
        }
        captioned.sort_by_key(|cue| cue.start);

        let one = align(&plain, &speech).correction();
        let other = align(&captioned, &speech).correction();
        let apart = (one.offset_ms() - other.offset_ms()).abs();
        assert!(apart <= 50, "the two tracks were {apart}ms apart");
        assert!((one.rate() - other.rate()).abs() < f64::EPSILON);
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
        // anybody would act on. Where exactly it crosses is not the claim, and
        // it does not have to fall at every step: agreement is a share of two
        // dozen chunks, so it comes in steps rather than smoothly.
        let mut previous = f32::MAX;
        for one_in in [40, 10, 5, 2] {
            let score = align(&cues, &noisier(&speech, one_in)).confidence().score();
            assert!(score <= previous, "confidence rose at one in {one_in}");
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
        // Both answers describe every stretch of the film perfectly, so the
        // agreement and the tightness are no use here and the separation is the
        // part that has to notice.
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
        assert!(found.confidence().agreement() > 0.9);
        assert!(found.confidence().tightness() > 0.9);
        assert!(found.confidence().separation() < 0.1);
        assert!(found.confidence().score() < DEFENSIBLE);
    }

    /// What the confidence is made of, rather than what it comes to.
    ///
    /// The share it reports is the count of stretches that agreed over the count
    /// it was measured in, and both of those go to whoever has to say why an
    /// answer should be believed. A figure that did not match the share it is
    /// read from would be two claims about the same film.
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn the_confidence_says_how_many_stretches_agreed_and_how_many_there_were() {
        let cues = dialogue(1_500);
        let found = align(&cues, &speech_of(&cues, Correction::of_offset(2_500)));
        let confidence = found.confidence();

        assert!(
            confidence.across() > 1,
            "measured in {}",
            confidence.across()
        );
        assert!(confidence.agreed() <= confidence.across());
        assert!(
            (confidence.agreement() - confidence.agreed() as f32 / confidence.across() as f32)
                .abs()
                < f32::EPSILON
        );
        // A film the answer describes leaves its stretches sitting on the line
        // rather than merely near it.
        assert!(
            confidence.spread_ms() < 150.0,
            "{}ms",
            confidence.spread_ms()
        );
    }

    /// Nothing measured says nothing in every part, and a film nobody could
    /// measure must not read as a film measured in no stretches and agreed by
    /// none of them, which is a figure rather than an absence.
    #[test]
    fn a_film_that_could_not_be_measured_reports_no_stretches_at_all() {
        let confidence = align(&[], &Signal::from_bins(vec![false; 100])).confidence();

        assert_eq!(confidence.agreed(), 0);
        assert_eq!(confidence.across(), 0);
        assert!(confidence.agreement().abs() < f32::EPSILON);
        assert!(confidence.tightness().abs() < f32::EPSILON);
    }

    /// The failure this whole stage exists to stop. A stretch and a shift trade
    /// against each other, so the wrong stretch at a compensating lag scores
    /// almost as well on one peak as the right one does, and a film given a
    /// stretch it did not need is barely wrong for the first minutes and minutes
    /// wrong by the end.
    ///
    /// A line across the film cannot be fooled that way, because a wrong stretch
    /// leaves an error that grows and a shift leaves one that does not, and no
    /// compensating lag makes a growing error look flat.
    #[test]
    fn a_film_that_needs_a_shift_is_not_given_a_stretch() {
        let cues = dialogue(1_500);
        let truth = Correction::of_offset(4_000);
        let found = align(&cues, &speech_of(&cues, truth));

        assert!(
            (found.correction().rate() - 1.0).abs() < f64::EPSILON,
            "stretched by {}",
            found.correction().rate()
        );
        let error = (found.correction().offset_ms() - truth.offset_ms()).unsigned_abs();
        assert!(error <= BIN_MS, "out by {error}ms");
    }

    /// The other way round, and the reason more than one candidate is carried.
    /// Refining around whatever the coarse pass liked best would confirm its
    /// ranking rather than check it, so every stretch the film could want is
    /// measured across the whole of it.
    #[test]
    fn a_film_that_needs_a_stretch_is_not_answered_with_a_shift() {
        let cues = dialogue(1_500);
        let truth = Correction::new(0, RATES[1]);
        let found = align(&cues, &speech_of(&cues, truth));
        let correction = found.correction();

        assert!((correction.rate() - truth.rate()).abs() < f64::EPSILON);
        for cue in &cues {
            let error = i64::from(correction.apply(cue.start).millis())
                - i64::from(truth.apply(cue.start).millis());
            assert!(error.abs() < 100, "out by {error}ms at {}", cue.start);
        }
    }

    /// A stretch nobody could have chosen by hand is refused rather than
    /// written. A film with time cut into the middle of it is out by nothing for
    /// the first part and by a minute and a half for the rest, and the nearest
    /// straight line through that is a stretch no framerate produces.
    ///
    /// Telling a file like this apart from one that merely needs shifting is not
    /// asked of the engine here, and the correction it settles on does put most
    /// of the film right. What is asked is that the answer is one somebody could
    /// have arrived at by hand, and that the part of the film the answer leaves
    /// behind shows up in the figure that would have to be cleared before
    /// anything were written.
    #[test]
    fn a_film_with_time_cut_into_it_is_not_answered_with_a_stretch_nobody_can_name() {
        let cues = dialogue(1_500);
        let cut = cues[cues.len() * 2 / 5].start.millis();
        let spliced: Vec<Cue> = cues
            .iter()
            .map(|cue| {
                let moved = |at: u32| if at < cut { at } else { at + 90_000 };
                Cue {
                    start: Timestamp::from_millis(moved(cue.start.millis())),
                    end: Timestamp::from_millis(moved(cue.end.millis())),
                    ..cue.clone()
                }
            })
            .collect();

        let found = align(&cues, &Signal::from_cues(&spliced));
        assert!(
            RATES
                .iter()
                .any(|known| (known - found.correction().rate()).abs() < f64::EPSILON),
            "answered with a rate of {}",
            found.correction().rate()
        );
        // And the file is named for what it is, so that nothing has to work out
        // from a rate and a landing figure why the answer was refused.
        assert!(matches!(found.misfit(), Some(Misfit::Breaks { .. })));
        // And the part of the film it cannot answer is missing from the figure,
        // which is what stands between this and somebody's library.
        assert!(
            found.landing().fraction() < 0.75,
            "{} of the film landed",
            found.landing().fraction()
        );
    }

    /// A film recorded off a broadcast, which is the first of the two shapes no
    /// single correction fits. Twenty seconds of advertisement is cut into the
    /// middle of it, so the lines before the break belong where they are and the
    /// lines after belong twenty seconds later. Every stretch of the film knows
    /// where its own lines are; no one number places both halves.
    ///
    /// Twenty seconds rather than one, because a break has to be further than a
    /// stretch is searched by before it counts as a break rather than as the
    /// wobble of a peak. It is well inside what the second, wider search covers,
    /// which is what makes the second half report where it is instead of
    /// reporting nothing.
    #[test]
    fn a_film_with_breaks_cut_into_it_is_named_as_one() {
        let cues = dialogue(1_500);
        let cut = cues[cues.len() / 2].start.millis();
        let broadcast: Vec<Cue> = cues
            .iter()
            .map(|cue| {
                let moved = |at: u32| if at < cut { at } else { at + 20_000 };
                Cue {
                    start: Timestamp::from_millis(moved(cue.start.millis())),
                    end: Timestamp::from_millis(moved(cue.end.millis())),
                    ..cue.clone()
                }
            })
            .collect();

        let found = align(&cues, &Signal::from_cues(&broadcast));
        let Some(Misfit::Breaks { at_ms }) = found.misfit() else {
            panic!(
                "a film with time cut into it has breaks: {:?}",
                found.misfit()
            );
        };

        // Within a stretch of film of where the break is. The readings are five
        // minutes wide, so this is as near as anything here can honestly say, and
        // it is near enough to tell somebody which part of the film to look at.
        assert!(
            at_ms.abs_diff(cut) < 400_000,
            "the break was at {cut}ms and was found at {at_ms}ms"
        );
    }

    /// A subtitle for a different cut of the same picture, which is the second
    /// shape and the one no correction can answer. A scene of a quarter of an hour
    /// is missing from the subtitle's cut, so the film talks through a stretch the
    /// subtitle has no lines for at all, and everything after it is out by the
    /// length of that scene.
    ///
    /// The distinction from the case above is what this is for. There the lines
    /// were all present and merely moved; here some of the film is not described
    /// at any shift, and bending the subtitle over it would produce something that
    /// looks aligned and is wrong in every added scene.
    #[test]
    fn a_subtitle_for_a_different_cut_is_named_as_one() {
        let spoken = dialogue(1_500);
        let scene = 900_000;
        let from = spoken[spoken.len() * 3 / 5].start.millis();

        // The subtitle's own cut: the scene is not in it, so its lines are not
        // there and everything after it happens a quarter of an hour earlier.
        let theirs: Vec<Cue> = spoken
            .iter()
            .filter(|cue| cue.start.millis() < from || cue.start.millis() >= from + scene)
            .map(|cue| {
                let moved = |at: u32| if at < from { at } else { at - scene };
                Cue {
                    start: Timestamp::from_millis(moved(cue.start.millis())),
                    end: Timestamp::from_millis(moved(cue.end.millis())),
                    ..cue.clone()
                }
            })
            .collect();

        let found = align(&theirs, &Signal::from_cues(&spoken));
        assert!(
            matches!(found.misfit(), Some(Misfit::DifferentCut { .. })),
            "a subtitle for another cut is not a mistimed one: {:?}",
            found.misfit()
        );
    }

    /// And the ordinary cases are not named as anything. A film that needs
    /// shifting, a film that needs stretching, a film that needs nothing and a
    /// film read imperfectly are all films one correction describes.
    #[test]
    fn a_film_one_correction_fits_is_not_named_as_a_misfit() {
        let cues = dialogue(1_500);
        for truth in [
            Correction::IDENTITY,
            Correction::of_offset(2_500),
            Correction::of_offset(-8_000),
            Correction::new(0, RATES[1]),
            Correction::new(-3_000, RATES[2]),
        ] {
            let found = align(&cues, &speech_of(&cues, truth));
            assert_eq!(found.misfit(), None, "at {truth:?}");
        }

        let noisy = noisier(&speech_of(&cues, Correction::of_offset(1_800)), 20);
        assert_eq!(align(&cues, &noisy).misfit(), None);
    }

    /// The figure the caller decides on, alongside the one it has to be
    /// compared against. Neither is any use without the other: no film scores
    /// well against the second, and the first is only ever read as a difference.
    ///
    /// Three quarters rather than all of it, because the transcript above
    /// overlaps its own lines and a line beginning while the last one is still
    /// running has nobody starting to speak under it. That is a property of
    /// every real transcript as well, and it is why the figure is compared with
    /// itself rather than against one.
    ///
    /// The figure for the file as written is well short of that and not nought,
    /// which is the other half of the same point. A window near a second wide
    /// catches a line here and there by chance, on a transcript whose gaps are a
    /// second or two, and a track that lands on a fifth of the film by accident
    /// is exactly why nobody should read this figure on its own.
    #[test]
    fn the_answer_says_how_the_lines_would_land_and_how_they_land_already() {
        let cues = dialogue(800);
        let found = align(&cues, &speech_of(&cues, Correction::of_offset(2_500)));

        assert!(found.landing().fraction() > 0.7);
        assert!(
            found.as_written().fraction() < found.landing().fraction() / 3.0,
            "{} landed already",
            found.as_written().fraction()
        );
        assert_eq!(found.landing().examined(), 800);
        assert_eq!(found.landing().median_ms(), 0);
    }

    /// A track that needs nothing already lands, so the two figures agree and
    /// there is no improvement to be had. This is what stops a correction being
    /// written to a file that was right to begin with.
    #[test]
    fn a_track_that_needs_nothing_lands_the_same_either_way() {
        let cues = dialogue(800);
        let found = align(&cues, &speech_of(&cues, Correction::IDENTITY));

        assert_eq!(found.landing(), found.as_written());
        assert!(found.landing().fraction() > 0.7);
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
            assert!(!found.landing().is_measured());
            assert!(!found.as_written().is_measured());
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
