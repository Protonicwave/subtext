//! The stretches a subtitle file is ever actually out by.

/// The conversions between the framerates films are released at.
///
/// The same list the timing panel offers by hand, and deliberately so. A rate
/// found here is one somebody can recognise when they see it, choose themselves,
/// and describe to another person. Fitting a free ratio instead would find a
/// slightly better number that answers to nothing.
///
/// The identity comes first, and a candidate has to beat it outright to be
/// preferred, so a track that needs no stretch is not given one on the strength
/// of a rounding difference.
pub const RATES: [f64; 6] = [
    1.0,
    25.0 / 23.976,
    23.976 / 25.0,
    24.0 / 23.976,
    23.976 / 24.0,
    30.0 / 29.97,
];

/// How far a measured stretch may sit from a named one and still be called it,
/// as a ratio.
///
/// Half the gap between the two closest candidates, so that a value can never be
/// near enough to two of them to have to choose. The closest pair is a film
/// taken from 23.976 frames to 24, which is a thousandth.
const NEAREST: f64 = 0.0005;

/// The same bound again, as drift across the film the stretch was measured on.
///
/// Half a second. The bound above is the one that keeps two candidates apart,
/// and on a feature length film it is far looser than the measurement: a
/// thousandth of a two hour film is seven seconds, which nobody would call the
/// same rate. This is what actually bites, and it is stated in the units
/// somebody watching would use rather than as a ratio, because that is what a
/// stretch means to them.
const DRIFT_MS: f64 = 500.0;

/// The candidates with the arithmetic duplicates left out.
///
/// A film taken from 23.976 frames to 24 and one taken from 29.97 to 30 are
/// stretched by exactly the same thousandth, so two of the six name one ratio.
/// Both are kept in the list, for the reason the test below gives, and measuring
/// both would cost a pass of the local stage to arrive at the answer already in
/// hand.
pub(crate) fn distinct() -> impl Iterator<Item = f64> {
    RATES.iter().copied().enumerate().filter_map(|(at, rate)| {
        RATES[..at]
            .iter()
            .all(|earlier| (earlier - rate).abs() > f64::EPSILON)
            .then_some(rate)
    })
}

/// The candidate a measured stretch is close enough to be called, if any.
///
/// A stretch nobody can name is far more likely to be a splice, a different cut
/// or the wrong file than a real framerate conversion, so the answer where
/// nothing is near enough is that there is no answer. The list is also the one
/// the timing panel offers by hand, which is what keeps a measured value one
/// somebody could have chosen themselves and can recognise when they see it.
///
/// `film_ms` is how long the film the stretch was measured on runs for, since
/// what makes two stretches the same is how far apart they leave the lines by
/// the end of it and not the ratio itself.
pub(crate) fn nearest(measured: f64, film_ms: f64) -> Option<f64> {
    RATES
        .iter()
        .copied()
        .filter(|known| {
            let apart = (known - measured).abs();
            apart <= NEAREST && apart * film_ms <= DRIFT_MS
        })
        .min_by(|one, other| (one - measured).abs().total_cmp(&(other - measured).abs()))
}

#[cfg(test)]
mod tests {
    // A candidate list this cannot read is the failure these tests are looking
    // for, and stopping there says so more plainly than an assertion would.
    #![allow(clippy::expect_used)]

    use super::{DRIFT_MS, RATES, distinct, nearest};

    /// Two hours, which is a film.
    const FILM_MS: f64 = 2.0 * 60.0 * 60.0 * 1_000.0;

    #[test]
    fn the_identity_is_the_first_candidate() {
        assert!((RATES[0] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn every_candidate_is_a_conversion_a_film_could_need() {
        // Wider than any real pair of framerates and narrow enough that a
        // candidate cannot move the end of a three hour film by more than the
        // ten minutes a correction is allowed to hold.
        for rate in RATES {
            assert!(
                rate > 0.95 && rate < 1.05,
                "{rate} is not a framerate ratio"
            );
        }
    }

    #[test]
    fn two_of_the_candidates_are_the_same_ratio_under_two_names() {
        // The list is the panel's, and the panel names conversions rather than
        // numbers. A film taken from 23.976 to 24 and one taken from 29.97 to 30
        // are stretched by exactly the same thousandth, so one of the six is
        // arithmetically spare. It is kept, because a list that has drifted from
        // the one somebody sees is a worse problem than a sixth of a second of
        // correlation, and because the candidate that wins has to be a candidate
        // they can then find by hand.
        assert!((RATES[3] - RATES[5]).abs() < f64::EPSILON);
        assert!((RATES[4] - 29.97 / 30.0).abs() < f64::EPSILON);
    }

    /// The list has six entries naming five ratios, and the work behind each
    /// candidate is paid for once rather than per name.
    #[test]
    fn the_ratio_named_twice_is_only_measured_once() {
        let measured: Vec<f64> = distinct().collect();
        assert_eq!(measured.len(), 5);
        assert!((measured[0] - 1.0).abs() < f64::EPSILON);
        for rate in RATES {
            assert!(
                measured
                    .iter()
                    .any(|kept| (kept - rate).abs() < f64::EPSILON)
            );
        }
    }

    #[test]
    fn a_stretch_all_but_on_a_candidate_is_called_that_candidate() {
        // A hundredth of the drift allowed, which is fifty milliseconds across
        // the film and is what a fit over real chunks actually leaves.
        let measured = RATES[1] + DRIFT_MS / 100.0 / FILM_MS;
        let named = nearest(measured, FILM_MS).expect("a stretch this close has a name");
        assert!((named - RATES[1]).abs() < f64::EPSILON);
    }

    /// The identity has to be nameable like any other, since a file that needs
    /// no stretch is the commonest file there is.
    #[test]
    fn a_stretch_of_nothing_is_named_as_nothing() {
        let named = nearest(1.0, FILM_MS).expect("the identity is a candidate");
        assert!((named - 1.0).abs() < f64::EPSILON);
    }

    /// A stretch nobody could have chosen by hand is far likelier to be a splice
    /// or the wrong file than a framerate conversion, and the honest answer is
    /// that it has no name.
    #[test]
    fn a_stretch_between_the_candidates_has_no_name() {
        assert_eq!(nearest(1.02, FILM_MS), None);
        // And one that would pass as a ratio still fails on what it does to the
        // film: a fifth of a thousandth is a second and a half by the end of
        // this one.
        assert_eq!(nearest(1.0 + 0.0002, FILM_MS), None);
    }

    /// The drift bound is the one that bites on a film long enough for a stretch
    /// to matter, so the same measurement is named on a short film and refused
    /// on a long one.
    #[test]
    fn how_long_the_film_runs_decides_what_counts_as_the_same_stretch() {
        let measured = 1.0 + 0.0002;
        assert!(nearest(measured, 10.0 * 60.0 * 1_000.0).is_some());
        assert_eq!(nearest(measured, FILM_MS), None);
    }
}
