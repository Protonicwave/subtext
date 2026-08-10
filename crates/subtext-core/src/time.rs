//! Positions within a film.

use core::fmt;

/// A position within a film, held as whole milliseconds from the start.
///
/// Subtitle timings carry millisecond resolution and films run to a few hours,
/// so a 32 bit millisecond count is exact over any plausible running time and
/// small enough to copy freely on the playback path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(u32);

impl Timestamp {
    /// The start of the film.
    pub const ZERO: Self = Self(0);

    /// The largest representable position, a little under fifty days.
    pub const MAX: Self = Self(u32::MAX);

    #[must_use]
    pub const fn from_millis(millis: u32) -> Self {
        Self(millis)
    }

    #[must_use]
    pub const fn from_seconds(seconds: u32) -> Self {
        Self(seconds.saturating_mul(1000))
    }

    #[must_use]
    pub const fn millis(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Reads a timestamp in the shapes subtitle files actually use.
    ///
    /// The written form is `HH:MM:SS,mmm`, but files in the wild drop the hours,
    /// write the fraction with a full stop, and give one or two fractional
    /// digits instead of three. All of those are accepted. The fraction is read
    /// as a decimal fraction of a second, so `,5` is half a second rather than
    /// five milliseconds. Anything else returns `None` for the caller to report.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (clock, fraction) = match text.find([',', '.']) {
            Some(at) => (&text[..at], &text[at + 1..]),
            None => (text, ""),
        };

        // Read right to left so that a missing hours field is simply an absent
        // third group rather than a special case.
        let mut units = [0_u64; 3];
        let mut read = 0;
        for part in clock.rsplit(':') {
            if read == units.len() {
                return None;
            }
            units[read] = parse_digits(part)?;
            read += 1;
        }
        if read == 0 {
            return None;
        }

        let total = units[2]
            .checked_mul(3600)?
            .checked_add(units[1].checked_mul(60)?)?
            .checked_add(units[0])?
            .checked_mul(1000)?
            .checked_add(fraction_millis(fraction)?)?;
        u32::try_from(total).ok().map(Self)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let millis = self.0 % 1000;
        let seconds = (self.0 / 1000) % 60;
        let minutes = (self.0 / 60_000) % 60;
        let hours = self.0 / 3_600_000;
        write!(f, "{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
    }
}

fn parse_digits(part: &str) -> Option<u64> {
    let part = part.trim();
    // A group long enough to overflow is malformed rather than large.
    if part.is_empty() || part.len() > 9 || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

fn fraction_millis(fraction: &str) -> Option<u64> {
    if fraction.is_empty() {
        return Some(0);
    }
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut millis = 0;
    for position in 0..3 {
        let digit = fraction
            .as_bytes()
            .get(position)
            .map_or(0, |byte| u64::from(byte - b'0'));
        millis = millis * 10 + digit;
    }
    Some(millis)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::Timestamp;

    #[test]
    fn reads_the_written_form() {
        assert_eq!(
            Timestamp::parse("01:02:03,004").unwrap().millis(),
            3_723_004
        );
    }

    #[test]
    fn accepts_a_full_stop_as_the_fraction_separator() {
        assert_eq!(
            Timestamp::parse("00:00:01.500"),
            Timestamp::parse("00:00:01,500")
        );
    }

    #[test]
    fn accepts_a_missing_hours_field() {
        assert_eq!(Timestamp::parse("02:03,000").unwrap().millis(), 123_000);
        assert_eq!(Timestamp::parse("03,000").unwrap().millis(), 3_000);
    }

    #[test]
    fn reads_a_short_fraction_as_a_decimal_fraction() {
        assert_eq!(Timestamp::parse("00:00:01,5").unwrap().millis(), 1_500);
        assert_eq!(Timestamp::parse("00:00:01,05").unwrap().millis(), 1_050);
        assert_eq!(Timestamp::parse("00:00:01").unwrap().millis(), 1_000);
    }

    #[test]
    fn ignores_surrounding_whitespace() {
        assert_eq!(
            Timestamp::parse("  00:00:01,000  ").unwrap().millis(),
            1_000
        );
    }

    #[test]
    fn discards_a_fraction_beyond_three_digits() {
        assert_eq!(Timestamp::parse("00:00:01,4999").unwrap().millis(), 1_499);
    }

    #[test]
    fn rejects_malformed_input() {
        for text in [
            "",
            "abc",
            "00:00:ab,000",
            "00:00:01,abc",
            "1:2:3:4,000",
            "00::01,000",
            "-00:00:01,000",
            "9999999999:00:00,000",
        ] {
            assert!(Timestamp::parse(text).is_none(), "{text} should not parse");
        }
    }

    #[test]
    fn writes_the_form_it_reads() {
        let text = "01:02:03,004";
        assert_eq!(Timestamp::parse(text).unwrap().to_string(), text);
        assert_eq!(Timestamp::ZERO.to_string(), "00:00:00,000");
    }
}
