//! Lining one signal up against another.

use core::fmt;
use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

use crate::signal::Signal;

/// How wide the shoulder of a peak is taken to be, in bins.
///
/// One second. A correlation peak between two subtitle-shaped signals is as wide
/// as a line is long, so the bins immediately either side of the best lag are the
/// same peak seen from slightly off centre rather than a rival explanation.
/// Counting them as competition would make every correct answer look uncertain.
const SHOULDER_BINS: isize = 100;

/// What the correlation found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Peak {
    /// How many bins later the cues have to be moved to sit over the speech.
    pub(crate) lag: isize,
    /// The peak as a fraction of what a perfect match would score.
    pub(crate) height: f64,
    /// The best score anywhere outside the peak's own shoulder.
    pub(crate) runner_up: f64,
}

/// One signal, transformed once, ready to be measured against many others.
///
/// The speech side of an alignment is fixed while the cue side is rebuilt at
/// every rate candidate, so the expensive half of the work is done in the
/// constructor and every buffer the search needs is allocated with it. Running
/// the six candidates costs six transforms and six inverses and no allocation
/// at all.
pub(crate) struct Correlator {
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
    /// The transform length, which is longer than either signal. See
    /// [`Correlator::new`] for how much longer and why.
    padded: usize,
    /// The furthest either way the search will look, in bins.
    window: isize,
    reference: Vec<Complex<f32>>,
    reference_energy: f64,
    real: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    output: Vec<f32>,
}

impl Correlator {
    /// Prepares to measure things against `speech`.
    ///
    /// `longest` is the most bins any cue signal that will be handed to
    /// [`Correlator::correlate`] can reach, and `window` the furthest either way
    /// the search should look.
    ///
    /// Returns `None` where the speech says the same thing everywhere, since
    /// there is then nothing to line anything up against.
    pub(crate) fn new(speech: &Signal, longest: usize, window: usize) -> Option<Self> {
        if speech.is_flat() {
            return None;
        }

        // A shift wider than the film itself is not a shift, and clamping here
        // is also what keeps the two ends of the search from meeting in the
        // middle of the transform.
        let window = window.min(speech.len());

        // Correlating through a transform gives a circular answer, where a cue
        // pushed off one end reappears at the other. Padding to the longer
        // signal plus the window keeps every lag the search actually looks at
        // clear of that wrap. Padding to the sum of the two lengths, which is
        // what a full linear correlation needs, would double the transform to
        // compute lags nobody asks about.
        let padded = fast_size(speech.len().max(longest) + window + 1);

        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(padded);
        let inverse = planner.plan_fft_inverse(padded);
        let scratch =
            vec![Complex::new(0.0, 0.0); forward.get_scratch_len().max(inverse.get_scratch_len())];

        let mut correlator = Self {
            padded,
            window: isize::try_from(window).unwrap_or(isize::MAX),
            reference: vec![Complex::new(0.0, 0.0); padded / 2 + 1],
            reference_energy: centred_energy(speech),
            real: vec![0.0; padded],
            spectrum: vec![Complex::new(0.0, 0.0); padded / 2 + 1],
            scratch,
            output: vec![0.0; padded],
            forward,
            inverse,
        };

        write_centred(&mut correlator.real, speech);
        correlator
            .forward
            .process_with_scratch(
                &mut correlator.real,
                &mut correlator.reference,
                &mut correlator.scratch,
            )
            .ok()?;

        Some(correlator)
    }

    /// The lag at which `cues` best explains the speech.
    ///
    /// Returns `None` where the cues say the same thing everywhere, which is
    /// what an empty track and a track covering every second of the film both
    /// look like.
    pub(crate) fn correlate(&mut self, cues: &Signal) -> Option<Peak> {
        if cues.is_flat() {
            return None;
        }

        write_centred(&mut self.real, cues);
        self.forward
            .process_with_scratch(&mut self.real, &mut self.spectrum, &mut self.scratch)
            .ok()?;

        // Multiplying one spectrum by the conjugate of the other and coming back
        // is the correlation of the two.
        for (product, reference) in self.spectrum.iter_mut().zip(&self.reference) {
            *product = *reference * product.conj();
        }
        // The inverse asks that the two bins which describe a real signal are
        // real, and the rounding in two transforms and a multiply can leave a
        // trace in them.
        if let Some(first) = self.spectrum.first_mut() {
            first.im = 0.0;
        }
        if let Some(last) = self.spectrum.last_mut() {
            last.im = 0.0;
        }

        self.inverse
            .process_with_scratch(&mut self.spectrum, &mut self.output, &mut self.scratch)
            .ok()?;

        // What a perfect match would score, which turns the correlation into a
        // fraction somebody can compare against a threshold. The inverse leaves
        // everything multiplied by the transform length, so that goes in here
        // rather than in a pass over the output.
        //
        // The energies are those of the whole signals, while the overlap at a
        // large lag is slightly smaller. The difference is under two per cent at
        // the edge of a two minute window on a feature length film, and it
        // leans against large shifts, which is the direction to lean.
        #[allow(clippy::cast_precision_loss)]
        let scale = (self.reference_energy * centred_energy(cues)).sqrt() * self.padded as f64;

        let mut lag = 0;
        let mut height = f64::NEG_INFINITY;
        for candidate in -self.window..=self.window {
            let value = f64::from(self.output[self.index_of(candidate)]);
            if value > height {
                height = value;
                lag = candidate;
            }
        }

        // Zero rather than the lowest value seen, so that a window too narrow to
        // hold anything outside the shoulder reports no competition instead of
        // reporting whatever the correlation happened to trough at.
        let mut runner_up = 0.0;
        for candidate in -self.window..=self.window {
            if (candidate - lag).abs() <= SHOULDER_BINS {
                continue;
            }
            let value = f64::from(self.output[self.index_of(candidate)]);
            if value > runner_up {
                runner_up = value;
            }
        }

        Some(Peak {
            lag,
            height: height / scale,
            runner_up: runner_up / scale,
        })
    }

    /// Where a lag sits in the transform's output.
    ///
    /// Moving the cues later is a positive lag and reads from the front. Moving
    /// them earlier wraps to the back, which is the one place the circular
    /// nature of the transform is used rather than worked around.
    fn index_of(&self, lag: isize) -> usize {
        if lag >= 0 {
            usize::try_from(lag).unwrap_or(0)
        } else {
            self.padded - lag.unsigned_abs()
        }
    }
}

impl fmt::Debug for Correlator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The plans hold no state worth reporting and the buffers are megabytes.
        f.debug_struct("Correlator")
            .field("padded", &self.padded)
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

/// Writes a signal into `dest` with its mean taken out, and zeroes the rest.
///
/// Correlating without this would measure how much of the film has any activity
/// at all, which both signals have plenty of, and the answer would be the same
/// everywhere. With the mean out, a bin only contributes where the two signals
/// agree about being unusual.
fn write_centred(dest: &mut [f32], signal: &Signal) {
    #[allow(clippy::cast_precision_loss)]
    let mean = signal.active() as f32 / signal.len() as f32;
    let mut written = 0;
    for (slot, bin) in dest.iter_mut().zip(signal.bins()) {
        *slot = if *bin { 1.0 - mean } else { -mean };
        written += 1;
    }
    for slot in &mut dest[written..] {
        *slot = 0.0;
    }
}

/// The energy a signal carries once its mean is taken out.
///
/// Counting bits rather than squaring them: a run of `n` bins with `a` of them
/// set has a mean of `a / n`, and the sum of the squared deviations comes to
/// `a - a * a / n` however they are arranged.
fn centred_energy(signal: &Signal) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let active = signal.active() as f64;
    #[allow(clippy::cast_precision_loss)]
    let length = signal.len() as f64;
    active * (1.0 - active / length)
}

/// The next transform length at or above `at_least` that is quick to run.
///
/// A transform is fastest at lengths built only from small factors, and slowest
/// at a large prime. Rounding up costs a little memory and saves a great deal of
/// time. Even lengths only, so the last bin of the spectrum is the one the
/// inverse expects to be real.
fn fast_size(at_least: usize) -> usize {
    let mut size = at_least.max(2);
    if !size.is_multiple_of(2) {
        size += 1;
    }
    while !is_smooth(size) {
        size += 2;
    }
    size
}

fn is_smooth(mut size: usize) -> bool {
    for factor in [2, 3, 5] {
        while size.is_multiple_of(factor) {
            size /= factor;
        }
    }
    size == 1
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{Correlator, fast_size, is_smooth};
    use crate::signal::Signal;

    /// A signal of `length` bins with a burst set every `every` bins.
    fn bursts(length: usize, every: usize, width: usize) -> Vec<bool> {
        let mut bins = vec![false; length];
        let mut at = every;
        while at + width < length {
            bins[at..at + width].fill(true);
            at += every;
        }
        bins
    }

    fn shifted(bins: &[bool], by: usize) -> Vec<bool> {
        let mut moved = vec![false; by];
        moved.extend_from_slice(bins);
        moved.truncate(bins.len());
        moved
    }

    #[test]
    fn a_shift_is_found_at_the_bin_it_was_made_at() {
        let bins = bursts(20_000, 130, 40);
        let speech = Signal::from_bins(shifted(&bins, 250));
        let cues = Signal::from_bins(bins);

        let mut correlator = Correlator::new(&speech, cues.len(), 1_000).expect("speech has shape");
        let peak = correlator.correlate(&cues).expect("cues have shape");
        assert_eq!(peak.lag, 250);
        assert!(peak.height > 0.9, "height was {}", peak.height);
    }

    #[test]
    fn a_signal_measured_against_itself_scores_one() {
        let signal = Signal::from_bins(bursts(20_000, 130, 40));
        let mut correlator =
            Correlator::new(&signal, signal.len(), 1_000).expect("the signal has shape");
        let peak = correlator.correlate(&signal).expect("the signal has shape");
        assert_eq!(peak.lag, 0);
        assert!(
            (peak.height - 1.0).abs() < 0.001,
            "height was {}",
            peak.height
        );
    }

    #[test]
    fn a_flat_signal_has_nothing_to_measure() {
        let speech = Signal::from_bins(bursts(2_000, 130, 40));
        assert!(Correlator::new(&Signal::from_bins(vec![false; 2_000]), 2_000, 100).is_none());
        assert!(Correlator::new(&Signal::from_bins(vec![true; 2_000]), 2_000, 100).is_none());

        let mut correlator = Correlator::new(&speech, 2_000, 100).expect("speech has shape");
        assert!(
            correlator
                .correlate(&Signal::from_bins(vec![false; 2_000]))
                .is_none()
        );
    }

    #[test]
    fn the_search_never_looks_further_than_the_film_is_long() {
        // A window wider than the signal would otherwise have the two ends of
        // the search reading each other's answers.
        let signal = Signal::from_bins(bursts(500, 30, 10));
        let mut correlator =
            Correlator::new(&signal, signal.len(), 100_000).expect("the signal has shape");
        let peak = correlator.correlate(&signal).expect("the signal has shape");
        assert_eq!(peak.lag, 0);
    }

    #[test]
    fn transform_lengths_are_even_and_built_from_small_factors() {
        for at_least in [1, 2, 3, 1_000, 100_001, 732_001] {
            let size = fast_size(at_least);
            assert!(size >= at_least.max(2));
            assert_eq!(size % 2, 0);
            assert!(is_smooth(size), "{size} is not a quick length");
        }
    }
}
