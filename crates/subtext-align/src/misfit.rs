//! The two ways a subtitle can fail to fit a film, told apart.
//!
//! A correlation over a whole film cannot represent this distinction at all. It
//! returns the one correction that explains the film best, and a file that no
//! single correction can explain still has a best one. What the local readings
//! can say, and a peak cannot, is whether every stretch of the film was
//! explained by that answer, and where it stopped being.
//!
//! Two shapes come out of that, and they want opposite things said about them.
//!
//! A rip with advertisement breaks in it has time inserted into content that is
//! otherwise contiguous. Every stretch of the film still finds where its lines
//! belong; the readings simply sit in tight segments either side of a clean step,
//! because a break moves everything after it and nothing before it. The film is
//! all there and the lines are all there, and what is wrong is that one number
//! cannot describe both halves.
//!
//! A subtitle for a different cut has content missing. Somewhere in the film
//! nobody is saying any of the lines being looked for, because the scene they
//! belong to is not in this cut or the one that is playing was not in theirs.
//! That stretch finds nothing at any shift, and no correction is the answer to
//! it: bending a theatrical subtitle onto an extended cut produces something that
//! looks aligned and is wrong in every added scene. Saying where it stopped
//! matching is worth more than any correction that could be applied there.
//!
//! Neither is written. This module names them, and naming is the whole of what it
//! does.

use crate::local::{AGREE_MS, middle_of};

/// How far apart two stretches of a film have to be left before the difference
/// is a step rather than a wobble, in milliseconds.
///
/// A second. What separates the two numbers here is a break, and a break that
/// somebody recorded a film through is seconds of advertisement at the least. The
/// bound below which readings count as agreeing is a seventh of this, so a film
/// whose chunks merely scatter cannot reach it: to be read as a step, each side
/// has to be tight to within that bound and the two sides a second apart, and a
/// steady drift cannot be both at once.
const STEP_MS: f64 = 1_000.0;

/// How many readings either side of a boundary are weighed against each other.
///
/// Four, which is twenty minutes of a feature at the usual chunk length. Enough
/// that one chunk finding the wrong thing cannot invent a step, and few enough
/// that a second break twenty minutes further on is a separate boundary rather
/// than part of this one. Weighing the whole film either side instead would miss
/// the second of two breaks entirely, since the segment between them would be
/// averaged into both.
const EITHER_SIDE: usize = 4;

/// The fewest readings a side may have and still be one.
///
/// Two. A single reading has no spread, so it agrees with itself perfectly and
/// would make a step out of any one chunk that found something distant.
const LEAST_ASIDE: usize = 2;

/// How many stretches of film in a row have to find nothing before the subtitle
/// is said not to describe them.
///
/// Three, which is fifteen minutes of a feature at the usual chunk length and
/// three minutes of the shortest film this measures at all. A scene or two that
/// nobody could match is a bad reading of a mixed soundtrack. A quarter of an
/// hour of it is a different cut of the film.
const LEAST_UNEXPLAINED: usize = 3;

/// What share of the film has to go unexplained alongside that.
///
/// A fifth, expressed as the divisor. The count above is what stops a short
/// stretch being called a different cut; this is what stops a long film being
/// called one on the strength of a fifteen minute patch that is a small part of
/// it. Both have to hold.
const UNEXPLAINED_SHARE: usize = 5;

/// A film the answer does not describe, and which of the two ways it does not.
///
/// Reported rather than acted on, in the same way and for the same reason as the
/// confidence: what to do about a file is a judgement about somebody's library.
/// Neither of these is a correction and neither is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Misfit {
    /// Time has been inserted into the film, which is what an advertisement
    /// break in a recording does. The lines are all there and one number cannot
    /// place them all.
    Breaks {
        /// Roughly where the film jumps, in milliseconds. Roughly because it is
        /// read off stretches of film several minutes long, and naming the frame
        /// would be claiming a precision this does not have.
        at_ms: u32,
    },
    /// The film says things this subtitle has no lines for, and goes on saying
    /// them. A subtitle for a different cut, and not a timing problem at all.
    DifferentCut {
        /// Roughly where it stopped matching, in milliseconds.
        from_ms: u32,
    },
}

/// What one stretch of film had to say about the answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Said {
    /// Where the lines in it still have to move to, in milliseconds. Nought for
    /// a stretch the answer already describes.
    Where(f64),
    /// Somebody is talking here and nothing in the track explains it, at any
    /// shift worth looking at.
    Nothing,
    /// Nothing to measure against. The film is silent here, or there is no film
    /// here at all, and a stretch like that is no evidence about a subtitle
    /// either way.
    Quiet,
}

/// One stretch of film, and what it said.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reading {
    /// The middle of the stretch, in milliseconds into the film.
    pub(crate) at_ms: f64,
    pub(crate) said: Said,
}

/// Which of the two shapes the readings have, where they have either.
///
/// Content missing is looked for first. A film carrying both, which is an
/// extended cut of a recorded broadcast, is the more serious of the two
/// diagnoses: breaks are a candidate for correction and a different cut is not,
/// so naming the breaks and quietly bending the subtitle over the added scenes
/// would be the worse mistake.
pub(crate) fn misfit_of(readings: &[Reading]) -> Option<Misfit> {
    unexplained(readings).or_else(|| stepped(readings))
}

/// Whether what a stretch said is borne out by the stretches around it.
///
/// This is what "found nothing" comes to in practice, and it is a question about
/// neighbours rather than about the strength of a peak. Dialogue falls every few
/// seconds, so a correlation over a stretch of film peaks somewhere whether the
/// lines are spoken in it or not, and the height it reaches depends on the mix,
/// the language and how much of the film is talking. What a stretch that really
/// found its lines has, and a stretch that peaked by chance has not, is company:
/// the stretches either side of it say the same thing, because a film is out by
/// the same amount for minutes at a time.
///
/// The two nearest stretches on one side are taken, a line is drawn through them,
/// and the stretch in question has to sit on it. Either side will do, which is
/// what lets the stretches at a break bear each other out: the one after the break
/// is borne out by the two after it, and the one before by the two before it.
///
/// A line rather than a level, because two stretches on a level and two on a
/// gentle drift should both count. A drift is what is left when the answer's
/// stretch is a little off, which is an answer that is nearly right rather than a
/// film that is not being described.
pub(crate) fn borne_out(readings: &[Reading], at: usize) -> bool {
    let Some(reading) = readings.get(at) else {
        return false;
    };
    let Said::Where(residual_ms) = reading.said else {
        return false;
    };

    let before = readings.get(..at).unwrap_or_default().iter().rev();
    let after = readings.get(at + 1..).unwrap_or_default().iter();
    predicts(before, reading.at_ms, residual_ms) || predicts(after, reading.at_ms, residual_ms)
}

/// Whether the first two stretches of `neighbours` say where the one at `at_ms`
/// should sit, and say `residual_ms`.
fn predicts<'a>(
    neighbours: impl Iterator<Item = &'a Reading>,
    at_ms: f64,
    residual_ms: f64,
) -> bool {
    let mut found = [(0.0, 0.0); 2];
    let mut count = 0;
    for neighbour in neighbours {
        if let Said::Where(neighbour_ms) = neighbour.said {
            found[count] = (neighbour.at_ms, neighbour_ms);
            count += 1;
            if count == 2 {
                break;
            }
        }
    }
    if count < 2 {
        return false;
    }

    let ((near_at, near), (far_at, far)) = (found[0], found[1]);
    let across = near_at - far_at;
    if across.abs() <= f64::EPSILON {
        return false;
    }

    let slope = (near - far) / across;
    let predicted = slope.mul_add(at_ms - near_at, near);
    (predicted - residual_ms).abs() <= AGREE_MS
}

/// The longest run of stretches nothing bears out, where it is long enough to
/// mean the subtitle is for something else.
///
/// A stretch with nothing in the film to measure against ends a run rather than
/// continuing it. Silence is not evidence either way, and a subtitle should not be
/// called wrong for a quiet reel between two it describes.
fn unexplained(readings: &[Reading]) -> Option<Misfit> {
    let mut longest = 0;
    let mut from = 0;
    let mut run = 0;
    for at in 0..readings.len() {
        if readings[at].said == Said::Quiet || borne_out(readings, at) {
            run = 0;
            continue;
        }
        run += 1;
        if run > longest {
            longest = run;
            from = at + 1 - run;
        }
    }

    if longest < LEAST_UNEXPLAINED || longest * UNEXPLAINED_SHARE < readings.len() {
        return None;
    }

    // Between the last stretch that matched and the first that did not, since
    // the change happened somewhere in there and the readings are not fine
    // enough to say where. A run beginning at the opening of the film has
    // nothing before it, and the answer there is the opening of the film.
    let started = readings.get(from)?.at_ms;
    let from_ms = match from.checked_sub(1).and_then(|before| readings.get(before)) {
        Some(before) => before.at_ms.midpoint(started),
        None => 0.0,
    };
    Some(Misfit::DifferentCut {
        from_ms: moment(from_ms),
    })
}

/// The first clean step in the readings.
///
/// Only reached once a run of stretches nothing bears out has been ruled out, and
/// that ordering is what separates the two shapes. A film with a quarter of an
/// hour it cannot explain is not a film with breaks in it, whatever the stretches
/// either side of that happen to look like.
///
/// A stretch here and there that nothing bears out is passed over rather than
/// counted. One is a bad reading of a mixed soundtrack, or the stretch a break
/// falls in the middle of, which belongs to both sides and therefore to neither.
fn stepped(readings: &[Reading]) -> Option<Misfit> {
    for boundary in 1..readings.len() {
        let Some(before) = side(readings, (0..boundary).rev()) else {
            continue;
        };
        let Some(after) = side(readings, boundary..readings.len()) else {
            continue;
        };
        if (after.level_ms - before.level_ms).abs() < STEP_MS {
            continue;
        }

        // Between the last stretch on one level and the first on the other,
        // which is the most a reading several minutes wide can say about where a
        // film jumps.
        let at_ms = before.nearest_at_ms.midpoint(after.nearest_at_ms);
        return Some(Misfit::Breaks {
            at_ms: moment(at_ms),
        });
    }

    None
}

/// Where the film sits on one side of a boundary.
#[derive(Clone, Copy, Debug)]
struct Side {
    /// The level the stretches on this side agree on, in milliseconds.
    level_ms: f64,
    /// Where the stretch nearest the boundary sits, in milliseconds into the
    /// film.
    nearest_at_ms: f64,
}

/// Where a side of a boundary sits, if it sits anywhere in particular.
///
/// Nothing where too few stretches spoke, and nothing where the ones that did
/// disagree among themselves: a side that is not tight has no level to compare,
/// and comparing it anyway is how a steady drift would be read as a step.
///
/// Nothing either where the stretch nearest the boundary is not itself on that
/// level. Without this the boundary would slide: a window of four readings turns
/// over once most of it is past the step, so the step would be reported a stretch
/// or two before the film actually jumps. Requiring the nearest stretch to agree
/// with its own side puts the boundary where the change is.
fn side(readings: &[Reading], order: impl Iterator<Item = usize>) -> Option<Side> {
    let mut found = [0.0; EITHER_SIDE];
    let mut nearest_at_ms = 0.0;
    let mut count = 0;
    for at in order {
        if !borne_out(readings, at) {
            continue;
        }
        if let Said::Where(residual_ms) = readings[at].said {
            if count == 0 {
                nearest_at_ms = readings[at].at_ms;
            }
            found[count] = residual_ms;
            count += 1;
            if count == EITHER_SIDE {
                break;
            }
        }
    }
    if count < LEAST_ASIDE {
        return None;
    }

    let nearest_ms = found[0];
    let level_ms = middle_of(&mut found[..count])?;
    for slot in &mut found[..count] {
        *slot = (*slot - level_ms).abs();
    }
    let tight = middle_of(&mut found[..count])? <= AGREE_MS;

    (tight && (nearest_ms - level_ms).abs() <= AGREE_MS).then_some(Side {
        level_ms,
        nearest_at_ms,
    })
}

/// A moment in a film as a count of milliseconds, whatever the arithmetic
/// produced.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn moment(at_ms: f64) -> u32 {
    at_ms.clamp(0.0, f64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    // A reading that comes back as the wrong shape of failure is what these
    // tests are looking for, and stopping there names it more plainly than an
    // assertion on a variant would.
    #![allow(clippy::panic)]

    use super::{Misfit, Reading, Said, misfit_of};

    /// How much film one reading stands for, which is the usual chunk length.
    const APART_MS: f64 = 300_000.0;

    /// Readings along a line, which is what a film the answer describes gives.
    fn along(slope: f64, intercept_ms: f64, count: usize) -> Vec<Reading> {
        (0..count)
            .map(|at| {
                #[allow(clippy::cast_precision_loss)]
                let at_ms = at as f64 * APART_MS;
                Reading {
                    at_ms,
                    said: Said::Where(intercept_ms + slope * at_ms),
                }
            })
            .collect()
    }

    /// Which reading a moment falls in, for asserting on where something was
    /// found without repeating the arithmetic that found it.
    fn near(at_ms: u32) -> usize {
        #[allow(clippy::cast_precision_loss)]
        let at = f64::from(at_ms) / APART_MS;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let at = at.round() as usize;
        at
    }

    /// A film with time cut into the middle of it. Every stretch of it knows
    /// where its lines belong, and no one number places both halves.
    #[test]
    fn a_clean_step_is_a_film_with_breaks_in_it() {
        let mut readings = along(0.0, 0.0, 24);
        for reading in readings.iter_mut().skip(12) {
            reading.said = Said::Where(90_000.0);
        }

        let found = misfit_of(&readings);
        let Some(Misfit::Breaks { at_ms }) = found else {
            panic!("a step is a break: {found:?}");
        };
        assert_eq!(near(at_ms), 12, "found at {at_ms}ms");
    }

    /// Two breaks, which is what a film recorded off a broadcast actually has.
    /// The first is what is reported: it is where the film stops being described
    /// by one number, and the second is past it.
    #[test]
    fn the_first_of_two_steps_is_the_one_named() {
        let mut readings = along(0.0, 0.0, 24);
        for reading in readings.iter_mut().skip(8) {
            reading.said = Said::Where(60_000.0);
        }
        for reading in readings.iter_mut().skip(16) {
            reading.said = Said::Where(120_000.0);
        }

        let found = misfit_of(&readings);
        let Some(Misfit::Breaks { at_ms }) = found else {
            panic!("two steps are still breaks: {found:?}");
        };
        assert_eq!(near(at_ms), 8, "found at {at_ms}ms");
    }

    /// The case this has to be careful about. A film out by a growing amount is
    /// a stretch the answer did not take out, and every boundary in it has one
    /// level before and a different level after. What tells it from a step is
    /// that neither side is tight: a drift steep enough to open a second between
    /// the two sides has left the four readings on each side spread further than
    /// that.
    #[test]
    fn a_drift_is_not_a_step_however_far_it_travels() {
        for slope in [0.0005, 0.002, 0.01, 0.05] {
            let found = misfit_of(&along(slope, -2_000.0, 24));
            assert_eq!(found, None, "a slope of {slope} was read as {found:?}");
        }
    }

    #[test]
    fn a_film_the_answer_describes_is_not_a_misfit() {
        assert_eq!(misfit_of(&along(0.0, 0.0, 24)), None);
        assert_eq!(misfit_of(&along(0.0, -140.0, 24)), None);

        // Nor is one where a stretch here and there found something distant, so
        // long as the film either side of it agrees with the answer. One chunk
        // is a bad reading; a segment is a break.
        let mut wobbling = along(0.0, 0.0, 24);
        wobbling[7].said = Said::Where(40_000.0);
        wobbling[18].said = Said::Where(-25_000.0);
        assert_eq!(misfit_of(&wobbling), None);
    }

    /// A stretch of film with no good answer anywhere in it, going on for a
    /// quarter of an hour. The lines being looked for are not spoken there,
    /// which is what a subtitle for a different cut of the same picture looks
    /// like.
    #[test]
    fn a_stretch_that_finds_nothing_is_a_subtitle_for_a_different_cut() {
        let mut readings = along(0.0, 0.0, 24);
        for reading in readings.iter_mut().skip(14) {
            reading.said = Said::Nothing;
        }

        let found = misfit_of(&readings);
        let Some(Misfit::DifferentCut { from_ms }) = found else {
            panic!("a film that goes unexplained is a different cut: {found:?}");
        };
        assert_eq!(near(from_ms), 14, "found at {from_ms}ms");
    }

    /// A different cut is looked for first, because a film that has both is one
    /// no correction should be offered for. Here the readings step and then stop
    /// meaning anything, which is an extended cut of a recording.
    #[test]
    fn a_film_that_both_steps_and_stops_matching_is_a_different_cut() {
        let mut readings = along(0.0, 0.0, 24);
        for reading in readings.iter_mut().skip(6).take(6) {
            reading.said = Said::Where(90_000.0);
        }
        for reading in readings.iter_mut().skip(12) {
            reading.said = Said::Nothing;
        }

        assert!(matches!(
            misfit_of(&readings),
            Some(Misfit::DifferentCut { .. })
        ));
    }

    /// A scene or two nobody could match is a bad reading of a mixed
    /// soundtrack, and a film is not called a different cut for it.
    #[test]
    fn a_short_patch_that_finds_nothing_is_not_a_different_cut() {
        let mut readings = along(0.0, 0.0, 24);
        readings[9].said = Said::Nothing;
        readings[10].said = Said::Nothing;
        assert_eq!(misfit_of(&readings), None);

        // Three in a row is enough of a run and still too small a share of a
        // film this long. Both conditions have to hold.
        readings[11].said = Said::Nothing;
        assert_eq!(misfit_of(&readings), None);
    }

    /// A film with nothing to hear in it says nothing about the subtitle. Silence
    /// is not evidence, and counting it as a stretch that found nothing would
    /// report a quiet film as the wrong film.
    #[test]
    fn a_silent_film_is_not_read_as_the_wrong_subtitle() {
        let mut readings = along(0.0, 0.0, 24);
        for reading in readings.iter_mut().skip(10) {
            reading.said = Said::Quiet;
        }

        assert_eq!(misfit_of(&readings), None);
    }

    /// And silence between two segments does not hide a step either. What is
    /// being asked about a boundary is where the film sits either side of it, and
    /// a stretch with nothing in it is passed over rather than counted.
    #[test]
    fn a_step_across_a_quiet_stretch_is_still_a_step() {
        let mut readings = along(0.0, 0.0, 24);
        readings[11].said = Said::Quiet;
        readings[12].said = Said::Quiet;
        for reading in readings.iter_mut().skip(13) {
            reading.said = Said::Where(45_000.0);
        }

        assert!(matches!(misfit_of(&readings), Some(Misfit::Breaks { .. })));
    }

    /// Too little to say anything about. A film measured in one or two stretches
    /// has no shape to read, and inventing one from two readings is exactly the
    /// mistake this part of the plan exists to stop.
    #[test]
    fn too_few_readings_to_read_are_read_as_nothing() {
        assert_eq!(misfit_of(&[]), None);
        assert_eq!(misfit_of(&along(0.0, 0.0, 1)), None);

        let mut two = along(0.0, 0.0, 2);
        two[1].said = Said::Where(90_000.0);
        assert_eq!(misfit_of(&two), None);
    }
}
