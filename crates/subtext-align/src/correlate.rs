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
    /// The best lag anywhere outside the peak's own shoulder, and what it
    /// scored.
    ///
    /// A second explanation rather than a measure of doubt. A film whose
    /// dialogue falls into a repeating pattern gives two lags that describe it
    /// about equally well, and neither the peak nor the arithmetic that found it
    /// can say which. Carrying the rival forward as a candidate in its own right
    /// is what puts the question to the film instead, and where the film cannot
    /// tell them apart either, the two score alike and say so.
    pub(crate) rival: Option<(isize, f64)>,
}

/// One signal, transformed once, ready to be measured against many others.
///
/// The reference side of a correlation is fixed while the other side is rebuilt
/// at every candidate, so the expensive half of the work is done when the
/// reference is set and every buffer the search needs is allocated before that.
/// Running a candidate costs one transform and one inverse and no allocation at
/// all.
///
/// Two of these exist during an alignment and they are sized differently. The
/// coarse pass measures a whole film in one go and its transform runs to about a
/// million bins. The local stage measures a few minutes at a time, and putting
/// its chunks through the coarse pass's plan would pay for a million bin
/// transform to answer a question about thirty thousand, several hundred times
/// over. Each is allocated once for the whole alignment and neither allocates
/// again.
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
    /// [`Correlator::against`] can reach, and `window` the furthest either way
    /// the search should look.
    ///
    /// Returns `None` where the speech says the same thing everywhere, since
    /// there is then nothing to line anything up against.
    pub(crate) fn new(speech: &Signal, longest: usize, window: usize) -> Option<Self> {
        // A shift wider than the film itself is not a shift, and clamping here
        // is also what keeps the two ends of the search from meeting in the
        // middle of the transform.
        let mut correlator = Self::sized(speech.len().max(longest), window.min(speech.len()));
        correlator.measuring(speech.bins()).then_some(correlator)
    }

    /// Buffers and plans for signals of at most `span` bins, searching `window`
    /// bins either way, with nothing to measure against yet.
    ///
    /// Every allocation an alignment makes for correlation is made here. What
    /// follows is transforms over buffers that already exist.
    pub(crate) fn sized(span: usize, window: usize) -> Self {
        let window = window.min(span);

        // Correlating through a transform gives a circular answer, where a cue
        // pushed off one end reappears at the other. Padding to the longer
        // signal plus the window keeps every lag the search actually looks at
        // clear of that wrap. Padding to the sum of the two lengths, which is
        // what a full linear correlation needs, would double the transform to
        // compute lags nobody asks about.
        let padded = fast_size(span + window + 1);

        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(padded);
        let inverse = planner.plan_fft_inverse(padded);
        let scratch =
            vec![Complex::new(0.0, 0.0); forward.get_scratch_len().max(inverse.get_scratch_len())];

        Self {
            padded,
            window: isize::try_from(window).unwrap_or(isize::MAX),
            reference: vec![Complex::new(0.0, 0.0); padded / 2 + 1],
            reference_energy: 0.0,
            real: vec![0.0; padded],
            spectrum: vec![Complex::new(0.0, 0.0); padded / 2 + 1],
            scratch,
            output: vec![0.0; padded],
            forward,
            inverse,
        }
    }

    /// Takes `bins` as the thing everything handed to [`Correlator::against`]
    /// will be measured against.
    ///
    /// Returns false where those bins say the same thing everywhere, since there
    /// is then nothing to line anything up against. The reference is transformed
    /// once here and reused, which is why the local stage measures a chunk of
    /// film against several candidates rather than a candidate against several
    /// chunks.
    pub(crate) fn measuring(&mut self, bins: &[bool]) -> bool {
        let Some(shape) = Shape::of(bins) else {
            return false;
        };

        write_centred(&mut self.real, bins, shape.mean);
        if self
            .forward
            .process_with_scratch(&mut self.real, &mut self.reference, &mut self.scratch)
            .is_err()
        {
            return false;
        }

        self.reference_energy = shape.energy;
        true
    }

    /// The lag at which `cues` best explains the reference.
    ///
    /// Returns `None` where the cues say the same thing everywhere, which is
    /// what an empty track and a track covering every second of the film both
    /// look like.
    pub(crate) fn against(&mut self, cues: &[bool]) -> Option<Peak> {
        let shape = Shape::of(cues)?;

        write_centred(&mut self.real, cues, shape.mean);
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
        let scale = (self.reference_energy * shape.energy).sqrt() * self.padded as f64;

        let mut lag = 0;
        let mut height = f64::NEG_INFINITY;
        for candidate in -self.window..=self.window {
            let value = f64::from(self.output[self.index_of(candidate)]);
            if value > height {
                height = value;
                lag = candidate;
            }
        }

        // Nothing rather than the lowest value seen, so that a window too narrow
        // to hold anything outside the shoulder reports no second explanation
        // instead of reporting whatever the correlation happened to trough at.
        let mut rival: Option<(isize, f64)> = None;
        for candidate in -self.window..=self.window {
            if (candidate - lag).abs() <= SHOULDER_BINS {
                continue;
            }
            let value = f64::from(self.output[self.index_of(candidate)]);
            if value > 0.0 && rival.is_none_or(|(_, best)| value > best) {
                rival = Some((candidate, value));
            }
        }

        Some(Peak {
            lag,
            height: height / scale,
            rival: rival.map(|(at, value)| (at, value / scale)),
        })
    }

    /// The furthest either way this will look, in bins.
    pub(crate) fn window(&self) -> isize {
        self.window
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

/// What a run of bins comes to once its mean is taken out.
#[derive(Clone, Copy, Debug)]
struct Shape {
    mean: f32,
    energy: f64,
}

impl Shape {
    /// Nothing where the bins say the same thing everywhere. A run that is
    /// entirely set, entirely clear, or empty has no shape to line anything up
    /// against, and correlating it would divide by zero.
    fn of(bins: &[bool]) -> Option<Self> {
        let active = bins.iter().filter(|bin| **bin).count();
        if active == 0 || active == bins.len() {
            return None;
        }

        #[allow(clippy::cast_precision_loss)]
        let mean = active as f32 / bins.len() as f32;
        // Counting bits rather than squaring them: a run of `n` bins with `a` of
        // them set has a mean of `a / n`, and the sum of the squared deviations
        // comes to `a - a * a / n` however they are arranged.
        #[allow(clippy::cast_precision_loss)]
        let energy = active as f64 * (1.0 - active as f64 / bins.len() as f64);
        Some(Self { mean, energy })
    }
}

/// Writes bins into `dest` with their mean taken out, and zeroes the rest.
///
/// Correlating without this would measure how much of the film has any activity
/// at all, which both signals have plenty of, and the answer would be the same
/// everywhere. With the mean out, a bin only contributes where the two signals
/// agree about being unusual.
fn write_centred(dest: &mut [f32], bins: &[bool], mean: f32) {
    let mut written = 0;
    for (slot, bin) in dest.iter_mut().zip(bins) {
        *slot = if *bin { 1.0 - mean } else { -mean };
        written += 1;
    }
    for slot in &mut dest[written..] {
        *slot = 0.0;
    }
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
        let peak = correlator.against(cues.bins()).expect("cues have shape");
        assert_eq!(peak.lag, 250);
        assert!(peak.height > 0.9, "height was {}", peak.height);
    }

    #[test]
    fn a_signal_measured_against_itself_scores_one() {
        let signal = Signal::from_bins(bursts(20_000, 130, 40));
        let mut correlator =
            Correlator::new(&signal, signal.len(), 1_000).expect("the signal has shape");
        let peak = correlator
            .against(signal.bins())
            .expect("the signal has shape");
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
        assert!(correlator.against(&[false; 2_000]).is_none());
    }

    #[test]
    fn the_search_never_looks_further_than_the_film_is_long() {
        // A window wider than the signal would otherwise have the two ends of
        // the search reading each other's answers.
        let signal = Signal::from_bins(bursts(500, 30, 10));
        let mut correlator =
            Correlator::new(&signal, signal.len(), 100_000).expect("the signal has shape");
        let peak = correlator
            .against(signal.bins())
            .expect("the signal has shape");
        assert_eq!(peak.lag, 0);
    }

    /// A film whose dialogue repeats gives two lags that explain it about
    /// equally well. Which of them is right is not a question the arithmetic can
    /// answer, so the second is reported rather than folded into a figure, and
    /// the caller puts both to the film.
    #[test]
    fn a_second_explanation_is_reported_alongside_the_first() {
        // The same run of bursts twice over, a thousand bins apart. Irregularly
        // spaced within itself, so that the only lag other than nought which
        // explains it is the thousand between the two copies.
        let mut bins = vec![false; 20_000];
        for copy in [0, 1_000] {
            for at in [0, 137, 349, 512, 733, 861] {
                let from = copy + at;
                bins[from..from + 30].fill(true);
            }
        }
        let signal = Signal::from_bins(bins);

        let mut correlator =
            Correlator::new(&signal, signal.len(), 2_000).expect("the signal has shape");
        let peak = correlator
            .against(signal.bins())
            .expect("the signal has shape");
        let (at, height) = peak.rival.expect("a repeating film has a second answer");
        assert_eq!(at.abs(), 1_000, "the second answer was at {at}");
        assert!(height > peak.height * 0.4, "the rival was {height}");
    }

    /// A reference and a candidate can be set independently, which is what lets
    /// one correlator serve every chunk of every candidate in the local stage
    /// without allocating again.
    #[test]
    fn the_thing_being_measured_against_can_be_changed_without_new_buffers() {
        let bins = bursts(2_000, 130, 40);
        let mut correlator = Correlator::sized(2_000, 200);

        assert!(!correlator.measuring(&[false; 2_000]));
        assert!(correlator.measuring(&shifted(&bins, 60)));
        assert_eq!(correlator.against(&bins).expect("shape").lag, 60);

        assert!(correlator.measuring(&shifted(&bins, 130)));
        assert_eq!(correlator.against(&bins).expect("shape").lag, 130);
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
