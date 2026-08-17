//! Measuring a candidate across the whole film rather than at one peak.
//!
//! A correlation over a whole film returns one number, and one number cannot
//! separate a stretch from a shift. The two trade against each other: a wrong
//! stretch at a compensating lag explains a film almost as well as the right
//! stretch at the right lag, and choosing between them on the height of a peak
//! is choosing on a difference that noise can cover.
//!
//! Cutting the film up separates them by geometry instead. Apply a candidate,
//! then ask each stretch of the film how far the lines in it still have to move.
//! A candidate that is right answers nought everywhere. A candidate whose
//! stretch is wrong answers a number that grows steadily from one end of the
//! film to the other, because that is what a wrong stretch is. A candidate that
//! is wrong in some other way answers nothing in particular. Fitting a line to
//! the answers reads the first two off directly: the slope is the stretch still
//! missing and the intercept is the shift, and how well the line fits is how
//! well the candidate explains the film.
//!
//! The order matters and is forced by arithmetic. Twenty five frames over 23.976
//! is a stretch of four and a quarter per cent, which is five minutes across a
//! feature and thirteen seconds inside a five minute chunk. Nothing can be
//! measured in a narrow window until the candidate has been applied to it, which
//! is why this runs per candidate rather than once.

use crate::correlate::{Correlator, Peak};
use crate::misfit::{Reading, Said, borne_out, off};
use crate::signal::{BIN_MS, Signal};

/// How much film one chunk covers at most, in milliseconds.
///
/// Five minutes, which cuts a feature into about two dozen points. Enough of
/// them that a line through them is a measurement rather than a pair of
/// readings, and each still long enough to hold a few hundred lines of dialogue,
/// which is what it takes to tell one moment of a film from another.
const LONGEST_CHUNK_MS: u32 = 300_000;

/// The least a chunk may cover.
///
/// A minute. Below this there is not enough dialogue in a chunk for a
/// correlation to mean anything, and a film too short to give even one such
/// chunk is measured in a single piece rather than not at all.
const SHORTEST_CHUNK_MS: u32 = 60_000;

/// How many chunks a film is cut into where its length allows.
const CHUNKS: usize = 24;

/// How far either way a chunk is searched, in milliseconds.
///
/// Ten seconds, and narrow on purpose. The candidate has already been applied,
/// so what is left to find is the error the candidate did not explain, and a
/// chunk that needs more than ten seconds on top of its candidate is a chunk the
/// candidate does not describe. Reporting that as a distant peak would be worse
/// than reporting nothing, since it would be entered into the fit as though it
/// were a measurement.
///
/// It is also what lets a neighbouring candidate converge on the right one. A
/// stretch a thousandth away from the truth leaves three and a half seconds of
/// error at either end of a feature, which is inside this, so the fit sees the
/// slope and names the ratio the film actually wants.
const LOCAL_WINDOW_MS: u32 = 10_000;

/// How near the fitted line a chunk has to sit to count as agreeing with it.
///
/// A hundred and fifty milliseconds, which is under four frames and inside what
/// a chunk's own peak can be expected to wander by. What this number decides is
/// the difference between a film explained by one correction and a film that
/// merely correlates, so it is set where a chunk that genuinely fits the line
/// clears it and a chunk that happens to fall near it does not.
pub(crate) const AGREE_MS: f64 = 150.0;

/// How far either way a stretch is searched a second time, in milliseconds.
///
/// Two minutes, the same as the whole film is searched by, and capped at half a
/// chunk so that what is being compared is still mostly overlap rather than
/// padding.
///
/// This is not part of the fit and never enters it. A stretch the answer does not
/// account for has said that much and no more, and there are two reasons for it:
/// the film has time cut into it, so this stretch has been moved further than the
/// answer accounts for, or the lines being looked for are not spoken here at all.
/// Looking again over a window this wide is what tells those apart, and telling
/// them apart is the difference between a file somebody could correct and a file
/// for a different cut.
const WIDE_WINDOW_MS: u32 = 120_000;

const LONGEST_CHUNK_BINS: usize = (LONGEST_CHUNK_MS / BIN_MS) as usize;
const SHORTEST_CHUNK_BINS: usize = (SHORTEST_CHUNK_MS / BIN_MS) as usize;
const LOCAL_WINDOW_BINS: usize = (LOCAL_WINDOW_MS / BIN_MS) as usize;
const WIDE_WINDOW_BINS: usize = (WIDE_WINDOW_MS / BIN_MS) as usize;

/// What one stretch of the film had to say about a candidate.
#[derive(Clone, Copy, Debug)]
struct Observation {
    /// Where in the film the chunk sits, in milliseconds at its middle.
    at_ms: f64,
    /// How far the lines in it still have to move, in milliseconds.
    residual_ms: f64,
}

/// A line through what the chunks said, and how well it describes them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fit {
    /// The stretch the candidate still has to be multiplied by.
    pub(crate) slope: f64,
    /// The shift the candidate still has to be moved by, in milliseconds.
    pub(crate) intercept_ms: f64,
    /// How much of the film agrees with the line, from none of it to all.
    pub(crate) agreement: f32,
    /// How closely the chunks that agree sit to it, from level with the bound to
    /// exactly on the line.
    pub(crate) tightness: f32,
}

impl Fit {
    /// The two parts as one number, for ranking candidates against each other.
    ///
    /// Multiplied rather than averaged, so that a poor showing in either part
    /// cannot be made up for by the other. A line most of the film ignores and a
    /// line three chunks sit exactly on are both poor explanations, and an
    /// average would call one of them respectable.
    pub(crate) fn score(self) -> f32 {
        self.agreement * self.tightness
    }
}

/// The film cut into stretches, with everything a pass over them needs.
///
/// Allocated once for an alignment and reused for every chunk of every
/// candidate. The correlator inside it is sized for a chunk rather than for a
/// film: putting thirty thousand bins through the coarse pass's million bin plan
/// would pay for the whole film to answer a question about five minutes of it,
/// several hundred times over.
pub(crate) struct Chunks {
    correlator: Correlator,
    /// The same stretches searched a great deal further, for the one pass that
    /// asks why a stretch could not be placed rather than where it sits.
    wide: Correlator,
    /// The cues over one chunk of film, gathered where the candidate puts them.
    cues: Vec<bool>,
    /// What the chunks said, cleared and refilled for each candidate.
    seen: Vec<Observation>,
    /// Pairwise slopes, held for the same reason.
    slopes: Vec<f64>,
    /// How far each chunk sits from the fitted line, likewise.
    residuals: Vec<f64>,
    /// What every chunk said about the answer, including the chunks the fit
    /// could not use. Filled once, for the candidate that won.
    readings: Vec<Reading>,
    /// The chunks the narrow search could not place, waiting for the wide one.
    pending: Vec<usize>,
    chunk_bins: usize,
    count: usize,
}

impl Chunks {
    /// Cuts a film of `film_bins` up, and allocates everything measuring it will
    /// need.
    ///
    /// Returns `None` for a film with nothing in it. A film too short to hold a
    /// chunk of the usual length is measured in one piece, which says what it
    /// can about a shift and nothing about a stretch, and that is the honest
    /// answer for a film there is not enough of.
    pub(crate) fn over(film_bins: usize) -> Option<Self> {
        if film_bins == 0 {
            return None;
        }

        let chunk_bins = (film_bins / CHUNKS)
            .clamp(SHORTEST_CHUNK_BINS, LONGEST_CHUNK_BINS)
            .min(film_bins);
        let count = (film_bins / chunk_bins).max(1);

        Some(Self {
            correlator: Correlator::sized(chunk_bins, LOCAL_WINDOW_BINS),
            wide: Correlator::sized(chunk_bins, WIDE_WINDOW_BINS.min(chunk_bins / 2)),
            cues: vec![false; chunk_bins],
            seen: Vec::with_capacity(count),
            slopes: Vec::with_capacity(count * count),
            residuals: Vec::with_capacity(count),
            readings: Vec::with_capacity(count),
            pending: Vec::with_capacity(count),
            chunk_bins,
            count,
        })
    }

    /// What the film makes of a candidate that has already been applied to the
    /// cues.
    ///
    /// `cues` is the track rebuilt at the candidate's rate and `lag_bins` the
    /// shift the coarse pass found for it, so that a bin of film at `at`
    /// corresponds to the bin of `cues` at `at - lag_bins`. What comes back
    /// describes what the candidate did not explain.
    ///
    /// Returns `None` where too little of the film had anything to say for a
    /// line through it to mean anything.
    pub(crate) fn fit(&mut self, speech: &Signal, cues: &Signal, lag_bins: isize) -> Option<Fit> {
        let Self {
            correlator,
            cues: gathered,
            seen,
            chunk_bins,
            count,
            ..
        } = self;

        seen.clear();
        for chunk in 0..*count {
            let from = chunk * *chunk_bins;
            let Some(film) = speech.bins().get(from..from + *chunk_bins) else {
                continue;
            };
            if !correlator.measuring(film) {
                continue;
            }

            gather(
                gathered,
                cues.bins(),
                isize::try_from(from).unwrap_or(isize::MAX) - lag_bins,
            );
            let Some(peak) = correlator.against(gathered) else {
                continue;
            };
            // A chunk that answers from the very edge of the window has not been
            // explained by the candidate; it has run out of room to say so. The
            // fit is better off without it than with a bound reported as a
            // measurement.
            if peak.lag.abs() >= correlator.window() {
                continue;
            }

            #[allow(clippy::cast_precision_loss)]
            let middle = (from + *chunk_bins / 2) as f64 * f64::from(BIN_MS);
            #[allow(clippy::cast_precision_loss)]
            let residual = peak.lag as f64 * f64::from(BIN_MS);
            seen.push(Observation {
                at_ms: middle,
                residual_ms: residual,
            });
        }

        self.line()
    }

    /// What every stretch of the film said about a candidate, including the
    /// stretches the fit had to leave out.
    ///
    /// The fit wants only the stretches that were placed, because a bound
    /// reported as a measurement would pull the line. Reading a film for the ways
    /// it does not fit wants the opposite: the stretches that could not be placed
    /// are the evidence, and what has to be said about each of them is why.
    ///
    /// So this runs once, for the candidate that won, and asks a second question
    /// of every stretch the answer does not account for. Searched over two minutes
    /// rather than ten seconds, does the stretch say where its lines are after all?
    /// Then the film has been moved further here than the answer accounts for,
    /// which is what time cut into a film does. Does it still say nothing the film
    /// around it agrees with? Then these lines are not spoken in this stretch of
    /// this film, which is what a subtitle for a different cut looks like and what
    /// no correction can put right.
    ///
    /// What every reading holds is its distance from the answer rather than from
    /// the candidate the answer was composed out of. That is what makes a stretch
    /// the answer describes read nought, whatever stretch and shift the answer is
    /// made of, so the two shapes below are the only things left in the numbers.
    ///
    /// Nothing is allocated. The wide search runs over the buffers it was given
    /// when the film was cut up, and on a film that fits it runs not at all.
    pub(crate) fn readings(
        &mut self,
        speech: &Signal,
        cues: &Signal,
        lag_bins: isize,
        fit: Fit,
    ) -> &[Reading] {
        let Self {
            correlator,
            wide,
            cues: gathered,
            readings,
            pending,
            chunk_bins,
            count,
            ..
        } = self;

        readings.clear();
        pending.clear();

        for chunk in 0..*count {
            let from = chunk * *chunk_bins;
            #[allow(clippy::cast_precision_loss)]
            let at_ms = (from + *chunk_bins / 2) as f64 * f64::from(BIN_MS);
            let answer_ms = fit.slope.mul_add(at_ms, fit.intercept_ms);

            let said = match speech.bins().get(from..from + *chunk_bins) {
                Some(film) if correlator.measuring(film) => {
                    gather(
                        gathered,
                        cues.bins(),
                        isize::try_from(from).unwrap_or(isize::MAX) - lag_bins,
                    );
                    match correlator.against(gathered) {
                        Some(peak) if answered(&peak, correlator.window()) => {
                            away_from(answer_ms, peak.lag)
                        }
                        // Nothing inside a window this narrow, which a stretch
                        // moved further than the answer accounts for cannot have.
                        _ => Said::Nothing,
                    }
                }
                // Either the film says nothing here or there is no film here.
                // Silence is no evidence about a subtitle, so it is not counted
                // as a stretch that found nothing.
                _ => Said::Quiet,
            };

            readings.push(Reading { at_ms, said });
        }

        // Which stretches to look at again: the ones the answer does not account
        // for, and the ones nothing around them bears out. Both, because a stretch
        // moved half a minute by a break can look like either. It peaks somewhere
        // inside ten seconds whatever the truth is, and that chance peak lands near
        // the answer about as readily as it lands far from it, so the reading that
        // matters is whether the film around it agrees.
        //
        // The wider search subsumes this one, since it looks over everything this
        // looked over and further, so a stretch that was right the first time comes
        // back with the same answer. What it costs is one more transform for each
        // stretch in doubt, on the buffers already to hand.
        pending.extend(
            (0..readings.len())
                .filter(|at| off(&readings[*at]) || !borne_out(readings, *at))
                .filter(|at| readings[*at].said != Said::Quiet),
        );

        for chunk in pending.iter().copied() {
            let from = chunk * *chunk_bins;
            let Some(film) = speech.bins().get(from..from + *chunk_bins) else {
                continue;
            };
            if !wide.measuring(film) {
                continue;
            }
            gather(
                gathered,
                cues.bins(),
                isize::try_from(from).unwrap_or(isize::MAX) - lag_bins,
            );
            let Some(peak) = wide.against(gathered) else {
                continue;
            };
            if !answered(&peak, wide.window()) {
                continue;
            }
            if let Some(reading) = readings.get_mut(chunk) {
                let answer_ms = fit.slope.mul_add(reading.at_ms, fit.intercept_ms);
                reading.said = away_from(answer_ms, peak.lag);
            }
        }

        readings
    }

    /// The line through what the chunks said.
    ///
    /// Fitted by taking the middle of the slopes between every pair of chunks,
    /// which is what makes it survive a minority of them disagreeing. A least
    /// squares line is pulled by every point it is given in proportion to how
    /// far away it is, so a handful of chunks that found the wrong thing would
    /// decide it; the middle of the pairwise slopes is unmoved until they are
    /// most of the film. That matters here more than the last decimal of
    /// precision, because the chunks that disagree are exactly the shape a
    /// spliced film has.
    fn line(&mut self) -> Option<Fit> {
        // One chunk says where the film sits and nothing about how it drifts, so
        // the line through it is flat. This is a short film rather than a
        // failure.
        let first = self.seen.first()?.residual_ms;
        if self.seen.len() == 1 {
            return Some(self.about(0.0, first));
        }

        let Self {
            seen,
            slopes,
            residuals,
            ..
        } = self;

        slopes.clear();
        for (at, one) in seen.iter().enumerate() {
            for other in &seen[at + 1..] {
                let across = other.at_ms - one.at_ms;
                if across.abs() > f64::EPSILON {
                    slopes.push((other.residual_ms - one.residual_ms) / across);
                }
            }
        }
        let slope = middle_of(slopes)?;

        residuals.clear();
        residuals.extend(
            seen.iter()
                .map(|seen| seen.residual_ms - slope * seen.at_ms),
        );
        let intercept = middle_of(residuals)?;

        Some(self.about(slope, intercept))
    }

    /// How well a line describes the chunks, given the line.
    fn about(&mut self, slope: f64, intercept_ms: f64) -> Fit {
        let Self {
            seen,
            residuals,
            count,
            ..
        } = self;
        let count = *count;

        residuals.clear();
        residuals.extend(
            seen.iter()
                .map(|seen| (seen.residual_ms - (intercept_ms + slope * seen.at_ms)).abs()),
        );

        let agreed = residuals.iter().filter(|away| **away <= AGREE_MS).count();
        // Out of every chunk the film was cut into rather than out of the ones
        // that answered. A chunk with nothing to say about a candidate is
        // evidence against it, and counting only the chunks that spoke would let
        // a candidate two thirds of a film ignores score as well as one that
        // explains all of it.
        #[allow(clippy::cast_precision_loss)]
        let agreement = agreed as f32 / count as f32;

        let spread = middle_of(residuals).unwrap_or(AGREE_MS);
        #[allow(clippy::cast_possible_truncation)]
        let tightness = (1.0 - spread / AGREE_MS).clamp(0.0, 1.0) as f32;

        Fit {
            slope,
            intercept_ms,
            agreement,
            tightness,
        }
    }
}

/// Whether a stretch of film came back with an answer at all.
///
/// Inside the window rather than at the edge of it, since the edge is where a
/// stretch says it has run out of room rather than where it says its lines are.
/// And above nothing, since a correlation whose best offering is worse than
/// nothing has found nothing.
///
/// Whether the answer is any good is not asked here and cannot be: the height a
/// correlation reaches depends on the mix, the language and how much of the film
/// is talking, so there is no level to compare it against. That question is put to
/// the stretches either side instead, in [`crate::misfit::borne_out`].
fn answered(peak: &Peak, window: isize) -> bool {
    peak.lag.abs() < window && peak.height > 0.0
}

/// How far a stretch is left from where the answer puts it.
///
/// The stretch says where its lines belong; `answer_ms` is where the fitted line
/// says they belong. The difference is what the answer failed to explain about
/// this stretch of film, and nought is a stretch it explained.
#[allow(clippy::cast_precision_loss)]
fn away_from(answer_ms: f64, lag: isize) -> Said {
    Said::Where(lag as f64 * f64::from(BIN_MS) - answer_ms)
}

/// Copies the bins of `from` that fall over a chunk into `dest`, padding with
/// quiet where the chunk reaches past either end.
///
/// `at` is the index in `from` that `dest` starts at, and it is signed because a
/// candidate that moves the cues later puts the opening of the film before the
/// beginning of the track. Both ends are quiet rather than missing, which is the
/// same thing the signal itself says about a bin past its end.
fn gather(dest: &mut [bool], from: &[bool], at: isize) {
    dest.fill(false);

    // Where the chunk begins before the track does, the copy begins that far
    // into the chunk instead of at the front of it.
    let (into, start) = if at < 0 {
        (at.unsigned_abs(), 0)
    } else {
        (0, at.unsigned_abs())
    };
    if into >= dest.len() || start >= from.len() {
        return;
    }

    let taken = (dest.len() - into).min(from.len() - start);
    dest[into..into + taken].copy_from_slice(&from[start..start + taken]);
}

/// The middle value, which is the one an outlier cannot move.
///
/// The lower of the two middles where the count is even, which is a difference
/// of half a bin in a figure that is already an estimate.
pub(crate) fn middle_of(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable_by(f64::total_cmp);
    values.get((values.len() - 1) / 2).copied()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::expect_used,
        clippy::panic
    )]

    use super::{AGREE_MS, Chunks, Fit, Observation, gather, middle_of};
    use crate::misfit::Said;
    use crate::signal::{BIN_MS, Signal};

    /// Two hours, which is a film.
    const FILM_MS: u32 = 2 * 60 * 60 * 1_000;

    fn chunks() -> Chunks {
        Chunks::over((FILM_MS / BIN_MS) as usize).expect("a film has something in it")
    }

    /// The line a set of readings comes to, without going near a film.
    fn through(readings: &[(f64, f64)]) -> Fit {
        let mut chunks = chunks();
        chunks.seen = readings
            .iter()
            .map(|(at_ms, residual_ms)| Observation {
                at_ms: *at_ms,
                residual_ms: *residual_ms,
            })
            .collect();
        chunks.count = readings.len();
        chunks.line().expect("readings make a line")
    }

    /// Readings on a line, which is what a candidate that describes the film
    /// produces.
    fn along(slope: f64, intercept: f64, count: usize) -> Vec<(f64, f64)> {
        (0..count)
            .map(|at| {
                let at_ms = at as f64 * 300_000.0;
                (at_ms, intercept + slope * at_ms)
            })
            .collect()
    }

    #[test]
    fn a_clean_line_is_read_off_exactly() {
        let found = through(&along(0.001, -2_000.0, 24));

        assert!((found.slope - 0.001).abs() < 1e-9, "slope {}", found.slope);
        assert!((found.intercept_ms + 2_000.0).abs() < 1.0);
        assert!((found.agreement - 1.0).abs() < f32::EPSILON);
        assert!((found.tightness - 1.0).abs() < f32::EPSILON);
    }

    /// A candidate that needs nothing further is the commonest answer there is,
    /// and it has to come back as nothing rather than as a small something.
    #[test]
    fn a_film_that_needs_nothing_more_reads_as_nothing_more() {
        let found = through(&along(0.0, 0.0, 24));

        assert!(found.slope.abs() < f64::EPSILON);
        assert!(found.intercept_ms.abs() < f64::EPSILON);
        assert!(found.score() > 0.99);
    }

    /// A minority of chunks finding the wrong thing is what a real film does,
    /// and the line has to be the one the rest of them describe.
    #[test]
    fn a_handful_of_chunks_disagreeing_does_not_move_the_line() {
        let mut readings = along(0.001, -2_000.0, 24);
        for (at, reading) in readings.iter_mut().enumerate() {
            if at % 7 == 3 {
                reading.1 += 6_000.0;
            }
        }
        let found = through(&readings);

        assert!((found.slope - 0.001).abs() < 1e-9, "slope {}", found.slope);
        assert!((found.intercept_ms + 2_000.0).abs() < 1.0);
        // The chunks that agree are exactly on the line, and the ones that do
        // not are counted against it.
        assert!(found.agreement > 0.8 && found.agreement < 1.0);
        assert!((found.tightness - 1.0).abs() < f32::EPSILON);
    }

    /// A film with time cut into the middle of it, which no line can describe.
    /// Half the chunks sit on one line and half on another, so whichever is
    /// fitted leaves the other half disagreeing, and the score says so.
    #[test]
    fn a_step_leaves_half_the_film_disagreeing() {
        let mut readings = along(0.0, 0.0, 24);
        for reading in readings.iter_mut().skip(12) {
            reading.1 += 90_000.0;
        }
        let found = through(&readings);

        assert!(found.agreement <= 0.55, "agreement {}", found.agreement);
        assert!(found.score() < 0.6, "score {}", found.score());
    }

    /// Chunks that found nothing in particular, which is what a candidate the
    /// film does not support looks like. There is no line in it and the fit has
    /// to say so rather than finding one.
    #[test]
    fn pure_scatter_agrees_with_nothing() {
        let readings: Vec<(f64, f64)> = (0..24_i32)
            .map(|at| {
                // A fixed pattern rather than a random one, so a failing test
                // fails the same way twice.
                let wander = f64::from((at * 2_749) % 9_001) - 4_500.0;
                (f64::from(at) * 300_000.0, wander)
            })
            .collect();
        let found = through(&readings);

        assert!(found.agreement < 0.3, "agreement {}", found.agreement);
        assert!(found.score() < 0.1, "score {}", found.score());
    }

    /// A candidate most of the film had nothing to say about is a poor
    /// explanation of it, however neatly the few chunks that spoke line up.
    #[test]
    fn chunks_that_said_nothing_count_against_a_candidate() {
        let mut chunks = chunks();
        chunks.seen = along(0.0, 0.0, 4)
            .iter()
            .map(|(at_ms, residual_ms)| Observation {
                at_ms: *at_ms,
                residual_ms: *residual_ms,
            })
            .collect();
        chunks.count = 24;

        let found = chunks.line().expect("four readings make a line");
        assert!((found.tightness - 1.0).abs() < f32::EPSILON);
        assert!(found.agreement < 0.2, "agreement {}", found.agreement);
    }

    #[test]
    fn a_film_with_one_chunk_in_it_says_where_it_sits_and_nothing_about_drift() {
        let found = through(&[(150_000.0, 1_200.0)]);

        assert!(found.slope.abs() < f64::EPSILON);
        assert!((found.intercept_ms - 1_200.0).abs() < f64::EPSILON);
    }

    /// A film shorter than a chunk is measured in one piece rather than not at
    /// all, and a film with nothing in it is not measured.
    #[test]
    fn a_short_film_is_cut_into_what_there_is() {
        let short = Chunks::over(2_000).expect("thirty seconds is a film of a sort");
        assert_eq!(short.count, 1);
        assert_eq!(short.chunk_bins, 2_000);

        assert_eq!(chunks().count, 24);
        assert!(Chunks::over(0).is_none());
    }

    #[test]
    fn a_chunk_reaching_past_either_end_of_a_track_is_quiet_there() {
        let from = [true, true, true, true];
        let mut dest = [false; 6];

        gather(&mut dest, &from, -2);
        assert_eq!(dest, [false, false, true, true, true, true]);

        gather(&mut dest, &from, 2);
        assert_eq!(dest, [true, true, false, false, false, false]);

        gather(&mut dest, &from, 40);
        assert_eq!(dest, [false; 6]);

        gather(&mut dest, &from, -40);
        assert_eq!(dest, [false; 6]);
    }

    #[test]
    fn the_middle_of_nothing_is_nothing() {
        assert_eq!(middle_of(&mut []), None);
        assert_eq!(middle_of(&mut [3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(middle_of(&mut [4.0, 1.0, 3.0, 2.0]), Some(2.0));
    }

    /// The whole film, measured through the correlator rather than through
    /// readings written down by hand. A candidate that is a second out
    /// everywhere reads as a second out everywhere.
    #[test]
    fn a_candidate_out_by_a_constant_reads_as_that_constant() {
        let mut bins = vec![false; (FILM_MS / BIN_MS) as usize];
        let mut at = 3_000;
        while at + 200 < bins.len() {
            bins[at..at + 90 + (at % 60)].fill(true);
            at += 340 + (at % 211);
        }
        let speech = Signal::from_bins(bins.clone());

        // The same film, said a second earlier than it happens.
        let early = Signal::from_bins(bins[100..].to_vec());

        let mut chunks = chunks();
        let found = chunks
            .fit(&speech, &early, 0)
            .expect("a film measures against itself");

        assert!(
            (found.intercept_ms - 1_000.0).abs() <= f64::from(BIN_MS),
            "read as {}ms",
            found.intercept_ms
        );
        assert!(found.slope.abs() < 1e-5, "slope {}", found.slope);
        assert!(found.score() > 0.9, "score {}", found.score());
        // And the coarse lag is taken off it, so a candidate the coarse pass
        // already shifted has nothing further to say.
        let settled = chunks.fit(&speech, &early, 100).expect("measures again");
        assert!(settled.intercept_ms.abs() <= f64::from(BIN_MS));
        assert!(settled.score() > 0.9);
    }

    /// When somebody starts talking, at bins across a film, unevenly enough that
    /// one moment of it tells itself apart from another.
    ///
    /// The unevenness has to be over the whole film rather than in a pattern that
    /// comes round again. A spacing taken from the position modulo a small number
    /// reads as uneven and repeats every few lines, which puts a moment of this
    /// film seven seconds from a moment that looks exactly like it, and a stretch
    /// of a film like that has two answers rather than one. Real dialogue does not
    /// do that. The spread here is scattered by a multiply instead, and fixed, so
    /// that a failing test fails the same way twice.
    fn exchanges(length: usize) -> Vec<usize> {
        let mut at = 3_000;
        let mut starts = Vec::new();
        while at + 200 < length {
            starts.push(at);
            let scattered = (at as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40;
            at += 300 + (scattered as usize % 611);
        }
        starts
    }

    /// An answer that leaves the film needing nothing further, which is what the
    /// readings below are measured against.
    fn nothing_more() -> Fit {
        Fit {
            slope: 0.0,
            intercept_ms: 0.0,
            agreement: 1.0,
            tightness: 1.0,
        }
    }

    /// A film that talks at those moments.
    fn talking(starts: &[usize], length: usize) -> Signal {
        let mut bins = vec![false; length];
        for start in starts {
            let end = (start + 90).min(length);
            if *start < end {
                bins[*start..end].fill(true);
            }
        }
        Signal::from_bins(bins)
    }

    /// The reading the two misfit shapes are told apart by, taken through the
    /// correlator rather than written down by hand.
    ///
    /// A film with half a minute cut into the middle of it. The first half of the
    /// track sits where the film says, and the second half is out by thirty
    /// seconds, which is three times further than a stretch is searched by at
    /// first. What the reading has to do is place that second half anyway, because
    /// a stretch that says nothing and a stretch that says thirty seconds are the
    /// difference between a subtitle for another cut and a film with breaks in it.
    #[test]
    fn a_stretch_moved_further_than_the_narrow_window_is_still_placed() {
        let length = (FILM_MS / BIN_MS) as usize;
        let starts = exchanges(length);
        let break_at = length / 2;
        let moved: Vec<usize> = starts
            .iter()
            .map(|start| {
                if *start < break_at {
                    *start
                } else {
                    start - 3_000
                }
            })
            .collect();

        let mut chunks = chunks();
        let readings = chunks.readings(
            &talking(&starts, length),
            &talking(&moved, length),
            0,
            nothing_more(),
        );

        assert_eq!(readings.len(), 24);
        for reading in readings {
            let Said::Where(residual_ms) = reading.said else {
                panic!("every stretch of this film knows where its lines are: {reading:?}");
            };
            let wanted = if reading.at_ms < f64::from(break_at as u32 * BIN_MS) {
                0.0
            } else {
                30_000.0
            };
            assert!(
                (residual_ms - wanted).abs() <= f64::from(BIN_MS),
                "the stretch at {}ms read as {residual_ms}ms",
                reading.at_ms
            );
        }
    }

    /// A film with nothing to hear in it says nothing about a subtitle either
    /// way, so a stretch of silence is neither placed nor held against the track.
    #[test]
    fn a_silent_stretch_of_film_is_read_as_silence() {
        let length = (FILM_MS / BIN_MS) as usize;
        let starts = exchanges(length);
        let track = talking(&starts, length);

        // The same film with the last third of its soundtrack missing, which is
        // what a recording that stopped early sounds like.
        let mut quietened = track.bins().to_vec();
        quietened[length * 2 / 3..].fill(false);

        let mut chunks = chunks();
        let readings = chunks.readings(&Signal::from_bins(quietened), &track, 0, nothing_more());

        let quiet = readings
            .iter()
            .filter(|reading| reading.said == Said::Quiet)
            .count();
        assert!(quiet >= 7, "only {quiet} stretches read as silent");
        assert!(
            readings.iter().all(|reading| reading.said != Said::Nothing),
            "silence was counted against the subtitle"
        );
    }

    #[test]
    fn the_bound_a_chunk_has_to_sit_inside_is_the_one_it_is_judged_by() {
        let found = through(&[
            (0.0, 0.0),
            (300_000.0, AGREE_MS - 1.0),
            (600_000.0, 0.0),
            (900_000.0, 0.0),
        ]);
        assert!((found.agreement - 1.0).abs() < f32::EPSILON);
    }
}
