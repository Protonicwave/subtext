//! Keeping the part of the sound that a voice lives in.

/// The bottom of the band, in hertz.
///
/// Below this is rumble, room tone, traffic and the low end of a score, none of
/// which is speech and all of which carries a great deal of energy. A film mixed
/// with a loud low end would otherwise read as talking from beginning to end.
const LOW_HZ: f32 = 300.0;

/// The top of the band, in hertz.
///
/// Together with [`LOW_HZ`] this is the telephone band, settled on a century ago
/// for the reason it is wanted here: it is the part of a voice that carries the
/// speech. Above it sit cymbals, sibilance and most of what an effects track is
/// made of.
const HIGH_HZ: f32 = 3_400.0;

/// How sharp the corner of each filter is.
///
/// The Butterworth value, which is the flattest response through the band. The
/// corners are not what matters here, since the signal this feeds is a
/// comparison against a floor rather than something anybody listens to.
const Q: f32 = core::f32::consts::FRAC_1_SQRT_2;

/// How close to the Nyquist frequency a corner may sit before it is dropped.
///
/// A filter cannot have a corner at or above half the sample rate, and one just
/// below it is more warping than filter. Audio recorded low enough that the top
/// of the band is already near the top of the audio has nothing up there to
/// trim, so that filter is left out rather than fudged.
const NYQUIST_MARGIN: f32 = 0.45;

/// A one-section filter, in the transposed direct form.
///
/// Transposed because the state is two values updated after the output rather
/// than a history of inputs and outputs, which is the cheaper of the two shapes
/// per sample and the better behaved of the two in single precision.
#[derive(Clone, Copy, Debug)]
struct Section {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Section {
    /// A section from coefficients that have not yet been normalised.
    fn new(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Everything below `hz` taken out.
    fn high_pass(hz: f32, sample_rate: f32) -> Self {
        let w0 = core::f32::consts::TAU * hz / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * Q);
        Self::new(
            f32::midpoint(1.0, cos),
            -(1.0 + cos),
            f32::midpoint(1.0, cos),
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        )
    }

    /// Everything above `hz` taken out.
    fn low_pass(hz: f32, sample_rate: f32) -> Self {
        let w0 = core::f32::consts::TAU * hz / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * Q);
        Self::new(
            f32::midpoint(1.0, -cos),
            1.0 - cos,
            f32::midpoint(1.0, -cos),
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        )
    }

    #[inline]
    fn apply(&mut self, sample: f32) -> f32 {
        let out = self.b0 * sample + self.z1;
        self.z1 = self.b1 * sample - self.a1 * out + self.z2;
        self.z2 = self.b2 * sample - self.a2 * out;
        out
    }
}

/// The speech band of a film's audio, and nothing else.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Band {
    high_pass: Option<Section>,
    low_pass: Option<Section>,
}

impl Band {
    pub(crate) fn new(sample_rate: u32) -> Self {
        // A sample rate low enough to make either corner meaningless is not a
        // rate any film is mixed at, but it arrives from a file and so decides
        // nothing about whether this runs.
        #[allow(clippy::cast_precision_loss)]
        let rate = sample_rate as f32;
        let ceiling = rate * NYQUIST_MARGIN;
        Self {
            high_pass: (LOW_HZ < ceiling).then(|| Section::high_pass(LOW_HZ, rate)),
            low_pass: (HIGH_HZ < ceiling).then(|| Section::low_pass(HIGH_HZ, rate)),
        }
    }

    #[inline]
    pub(crate) fn apply(&mut self, sample: f32) -> f32 {
        let mut out = sample;
        if let Some(section) = self.high_pass.as_mut() {
            out = section.apply(out);
        }
        if let Some(section) = self.low_pass.as_mut() {
            out = section.apply(out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Band;

    /// How loudly a tone of `hz` comes out of the band, against how loudly it
    /// went in.
    fn response(hz: f32, sample_rate: u32) -> f32 {
        let mut band = Band::new(sample_rate);
        #[allow(clippy::cast_precision_loss)]
        let rate = sample_rate as f32;
        let samples = sample_rate as usize * 2;

        let mut energy = 0.0f32;
        for at in 0..samples {
            #[allow(clippy::cast_precision_loss)]
            let t = at as f32 / rate;
            let out = band.apply((core::f32::consts::TAU * hz * t).sin());
            // The first tenth of a second is the filter settling, and counting it
            // would measure the transient rather than the response.
            if at > sample_rate as usize / 10 {
                energy += out * out;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let mean = energy / samples as f32;
        // Against a full scale sine, whose mean square is a half.
        (mean / 0.5).sqrt()
    }

    #[test]
    fn a_voice_passes_through_the_band() {
        assert!(response(1_000.0, 48_000) > 0.9);
    }

    #[test]
    fn rumble_below_the_band_is_taken_out() {
        assert!(response(50.0, 48_000) < 0.05);
    }

    #[test]
    fn a_cymbal_above_the_band_is_taken_out() {
        assert!(response(12_000.0, 48_000) < 0.05);
    }

    #[test]
    fn a_rate_too_low_for_a_corner_drops_that_corner_rather_than_ringing() {
        // Six kilohertz audio holds nothing above the top of the band, so the
        // filter that would take it out is not built. The one at the bottom
        // still is, since the rumble is still down there.
        let band = Band::new(6_000);
        assert!(band.low_pass.is_none());
        assert!(band.high_pass.is_some());
        assert!(response(1_000.0, 6_000) > 0.9);
        assert!(response(50.0, 6_000) < 0.05);
    }

    #[test]
    fn silence_stays_silent() {
        let mut band = Band::new(48_000);
        for _ in 0..1_000 {
            assert!(band.apply(0.0).abs() < f32::EPSILON);
        }
    }
}
