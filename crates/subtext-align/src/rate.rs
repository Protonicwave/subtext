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

#[cfg(test)]
mod tests {
    use super::RATES;

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
}
