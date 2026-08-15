//! How a track's timings relate to the film it is paired with.

use crate::Timestamp;

/// The furthest a track may be moved, either way, in milliseconds.
///
/// Ten minutes. A file that is further out than that is not the same release
/// with a different intro, it is the wrong file, and the answer to it is a
/// different subtitle rather than a larger number.
const FURTHEST_MS: i32 = 600_000;

/// The slowest and fastest a track may be stretched.
///
/// The conversions that actually occur sit between 0.958 and 1.043, being the
/// ratios among the framerates films are released at. These bounds are far
/// wider than that and still narrow enough that a value which arrived by
/// accident cannot put the last line of a three hour film in the second hour.
const SLOWEST: f64 = 0.5;
const FASTEST: f64 = 2.0;

/// What a subtitle track's timings have to be put through to match its film.
///
/// A sidecar file is paired to a film by the likeness of the two names, which
/// says nothing about whether the timings inside were written against these
/// frames. A file timed against a different release is out by a constant amount
/// where the intro lengths differ, and out by a growing amount where the
/// framerates do. The offset answers the first and the rate answers the second.
///
/// This is a property of the data rather than of how it is shown, which is why
/// it is stored beside the track and applied as cues are read. Everything
/// downstream of that read sees playback timings and never learns that a
/// correction happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Correction {
    offset_ms: i32,
    rate: f64,
}

impl Correction {
    /// The track exactly as it was written, which is what almost every track is.
    pub const IDENTITY: Self = Self {
        offset_ms: 0,
        rate: 1.0,
    };

    /// A correction, with anything out of range brought back into it.
    ///
    /// The bounds are enforced here rather than at the call sites because this
    /// is read from a database, and a column can hold whatever a hand or an
    /// older version put in it.
    #[must_use]
    pub fn new(offset_ms: i32, rate: f64) -> Self {
        Self {
            offset_ms: offset_ms.clamp(-FURTHEST_MS, FURTHEST_MS),
            rate: if rate.is_finite() {
                rate.clamp(SLOWEST, FASTEST)
            } else {
                Self::IDENTITY.rate
            },
        }
    }

    /// A shift with no stretch, which is the correction people actually make.
    #[must_use]
    pub fn of_offset(offset_ms: i32) -> Self {
        Self::new(offset_ms, Self::IDENTITY.rate)
    }

    #[must_use]
    pub const fn offset_ms(self) -> i32 {
        self.offset_ms
    }

    #[must_use]
    pub const fn rate(self) -> f64 {
        self.rate
    }

    /// Whether this correction leaves every timestamp where it found it.
    #[must_use]
    pub fn is_identity(self) -> bool {
        // Compared exactly on purpose. The question is whether anybody has set
        // a rate at all, not whether the one they set is near enough to one,
        // and a rate that is near enough to one is still a rate they chose.
        #[allow(clippy::float_cmp)]
        {
            self.offset_ms == Self::IDENTITY.offset_ms && self.rate == Self::IDENTITY.rate
        }
    }

    /// Where in the film a timestamp written in the file actually falls.
    ///
    /// The one place this arithmetic exists. Every path that reads a cue calls
    /// it, so a line is at the same moment wherever it is asked about.
    #[must_use]
    pub fn apply(self, at: Timestamp) -> Timestamp {
        // An untouched track comes back untouched, without going near a float.
        // Multiplying by one and adding zero does in fact return every value a
        // timestamp can hold unchanged, but a library nobody has corrected
        // should not have to depend on that being true.
        if self.is_identity() {
            return at;
        }

        let moved = f64::from(at.millis()) * self.rate + f64::from(self.offset_ms);
        // A line pushed in front of the film starts with it. Losing the order
        // of the opening lines is better than losing the lines.
        if moved <= 0.0 {
            return Timestamp::ZERO;
        }
        let rounded = moved.round();
        if rounded >= f64::from(u32::MAX) {
            return Timestamp::MAX;
        }

        // Rounded, above zero and below the ceiling by the two branches above,
        // so nothing is lost in the narrowing.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Timestamp::from_millis(rounded as u32)
    }
}

impl Default for Correction {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::{FASTEST, FURTHEST_MS, SLOWEST};
    use crate::{Correction, Timestamp};

    fn at(millis: u32) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    #[test]
    fn the_identity_leaves_every_timestamp_alone() {
        let correction = Correction::IDENTITY;
        assert!(correction.is_identity());

        // Bit for bit, across the range rather than at a few points, because
        // the whole promise of the identity is that a track nobody has
        // corrected cannot drift through arithmetic it did not need.
        for millis in [0, 1, 999, 1_000, 3_600_000, 10_800_000, u32::MAX] {
            assert_eq!(correction.apply(at(millis)), at(millis));
        }
    }

    #[test]
    fn an_offset_moves_a_line_by_the_amount_it_says() {
        let later = Correction::of_offset(2_500);
        assert!(!later.is_identity());
        assert_eq!(later.apply(at(10_000)), at(12_500));

        let earlier = Correction::of_offset(-2_500);
        assert_eq!(earlier.apply(at(10_000)), at(7_500));
    }

    #[test]
    fn a_line_moved_in_front_of_the_film_starts_with_it() {
        let earlier = Correction::of_offset(-5_000);
        assert_eq!(earlier.apply(at(0)), Timestamp::ZERO);
        assert_eq!(earlier.apply(at(4_999)), Timestamp::ZERO);
        assert_eq!(earlier.apply(at(5_000)), Timestamp::ZERO);
        assert_eq!(earlier.apply(at(5_001)), at(1));
    }

    #[test]
    fn a_rate_grows_across_a_three_hour_film() {
        // The conversion a file timed for a 23.976 frame release needs to play
        // against a 25 frame one, which is the commonest of them.
        let faster = Correction::new(0, 25.0 / 23.976);

        // A minute in it is out by two and a half seconds, and three hours in
        // by more than seven minutes, which is the whole reason a constant
        // offset cannot fix this kind of file.
        assert_eq!(faster.apply(at(60_000)), at(62_563));
        assert_eq!(faster.apply(at(10_800_000)), at(11_261_261));

        // The start of the film is where both agree.
        assert_eq!(faster.apply(Timestamp::ZERO), Timestamp::ZERO);
    }

    #[test]
    fn a_rate_and_an_offset_are_applied_in_that_order() {
        // The rate stretches the film's own clock and the offset shifts the
        // result, so a line one minute in moves by the stretch and then by the
        // shift, rather than the shift being stretched along with it.
        let both = Correction::new(-1_000, 2.0);
        assert_eq!(both.apply(at(60_000)), at(119_000));
    }

    #[test]
    fn rounding_lands_on_the_nearer_millisecond() {
        let stretched = Correction::new(0, 1.5);
        assert_eq!(stretched.apply(at(1)), at(2)); // 1.5 up
        assert_eq!(stretched.apply(at(3)), at(5)); // 4.5 up
        assert_eq!(stretched.apply(at(5)), at(8)); // 7.5 up

        let squeezed = Correction::new(0, 0.75);
        assert_eq!(squeezed.apply(at(1)), at(1)); // 0.75 up
        assert_eq!(squeezed.apply(at(2)), at(2)); // 1.5 up
        assert_eq!(squeezed.apply(at(3)), at(2)); // 2.25 down
    }

    #[test]
    fn nothing_can_be_pushed_past_the_end_of_the_range() {
        let far = Correction::new(FURTHEST_MS, FASTEST);
        assert_eq!(far.apply(Timestamp::MAX), Timestamp::MAX);
        assert_eq!(far.apply(at(u32::MAX / 2)), Timestamp::MAX);
    }

    #[test]
    fn a_value_out_of_range_is_brought_back_into_it() {
        assert_eq!(Correction::of_offset(i32::MAX).offset_ms(), FURTHEST_MS);
        assert_eq!(Correction::of_offset(i32::MIN).offset_ms(), -FURTHEST_MS);

        assert!((Correction::new(0, 100.0).rate() - FASTEST).abs() < f64::EPSILON);
        assert!((Correction::new(0, 0.01).rate() - SLOWEST).abs() < f64::EPSILON);
    }

    #[test]
    fn a_rate_that_is_not_a_number_is_read_as_no_rate_at_all() {
        // What a column edited by hand, or written by a version that allowed
        // more than this one does, can hold.
        for rubbish in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(Correction::new(0, rubbish).is_identity());
        }
        assert!(Correction::default().is_identity());
    }

    #[test]
    fn a_correction_that_says_nothing_is_the_identity_however_it_arrived() {
        assert!(Correction::new(0, 1.0).is_identity());
        assert!(Correction::of_offset(0).is_identity());
        assert!(!Correction::new(1, 1.0).is_identity());
        assert!(!Correction::new(0, 1.001).is_identity());
    }
}
