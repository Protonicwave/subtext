//! Where there is activity, on a fixed time base.

use core::fmt;

use subtext_core::Cue;

/// How much of a film one bin covers, in milliseconds.
///
/// Ten milliseconds sits below the resolution anybody can perceive in subtitle
/// timing, so nothing is lost by measuring at it, and it keeps a three hour film
/// inside about a million bins, which is a size a transform can be run over
/// without thinking about it.
pub const BIN_MS: u32 = 10;

/// The longest run a signal may cover, as a count of bins.
///
/// Twelve hours. A timestamp can hold fifty days, and a single malformed cue
/// claiming one would otherwise decide the size of every buffer here. Nothing
/// that is actually a film reaches this, so the bound costs real files nothing.
const MAX_BINS: usize = 12 * 60 * 60 * (1000 / BIN_MS as usize);

/// Where there is activity across a film, one bit to each [`BIN_MS`].
///
/// Two things produce one of these and they are deliberately the same type. A
/// subtitle track says where it claims dialogue falls, and a film's audio says
/// where somebody is actually talking. Lining the two up is what the rest of
/// this crate does, and it does not care which is which.
#[derive(Clone, PartialEq, Eq)]
pub struct Signal {
    bins: Vec<bool>,
    /// How many bins are set. Held rather than counted, because the
    /// normalisation and the degenerate check both want it and both sit inside
    /// a loop over rate candidates.
    active: usize,
}

impl Signal {
    /// The bins a subtitle track covers, taken as written.
    #[must_use]
    pub fn from_cues(cues: &[Cue]) -> Self {
        let mut signal = Self {
            bins: Vec::new(),
            active: 0,
        };
        signal.rebuild(cues, 1.0);
        signal
    }

    /// A signal measured elsewhere, one bit to each [`BIN_MS`] from the start.
    ///
    /// This is how a film's audio arrives. Anything past the bound in
    /// [`MAX_BINS`] is dropped.
    #[must_use]
    pub fn from_bins(mut bins: Vec<bool>) -> Self {
        bins.truncate(MAX_BINS);
        let active = bins.iter().filter(|bin| **bin).count();
        Self { bins, active }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bins.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bins.is_empty()
    }

    /// How many bins are set.
    #[must_use]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Whether there is activity in a bin.
    ///
    /// A bin past the end of the signal is quiet rather than missing, so
    /// anything walking two signals of different lengths, which is every caller
    /// that has one of each, does not have to ask how long they are first.
    #[must_use]
    pub fn is_active(&self, bin: usize) -> bool {
        self.bins.get(bin).copied().unwrap_or(false)
    }

    pub(crate) fn bins(&self) -> &[bool] {
        &self.bins
    }

    /// Whether the signal says the same thing everywhere.
    ///
    /// A run that is entirely set, entirely clear, or empty has no shape to line
    /// anything up against. Correlating it would divide by zero and the answer
    /// would mean nothing if it did not.
    pub(crate) fn is_flat(&self) -> bool {
        self.active == 0 || self.active == self.bins.len()
    }

    /// Rewrites the signal as the cues would fall at `rate`.
    ///
    /// Rebuilding from the cues rather than resampling the bins, because a rate
    /// is a stretch of the authored clock and the cues still hold that clock
    /// exactly, whereas the bins have already been rounded once.
    ///
    /// The buffer is kept across calls. The rate search runs this once per
    /// candidate and the run is most of a megabyte for a long film.
    pub(crate) fn rebuild(&mut self, cues: &[Cue], rate: f64) {
        let length = span(cues, rate);
        self.bins.clear();
        self.bins.resize(length, false);

        for cue in cues {
            let from = bin_of(cue.start.millis(), rate).min(length);
            let to = bin_of(cue.end.millis(), rate).min(length);
            // Out of order and overlapping cues both fall out of this without a
            // case for either: a bin already set is simply set again.
            if from < to {
                self.bins[from..to].fill(true);
            }
        }

        // Counted afterwards rather than as the bins are set, so that marking a
        // cue is a plain write over a run of bytes rather than a test and a
        // branch at every bin. The rate search rebuilds this six times and both
        // halves of it are wide enough to be worth leaving alone.
        self.active = self.bins.iter().filter(|bin| **bin).count();
    }
}

impl fmt::Debug for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The bins themselves run to seven figures, and nobody reading a failing
        // test wants them.
        f.debug_struct("Signal")
            .field("bins", &self.bins.len())
            .field("active", &self.active)
            .finish()
    }
}

/// How many bins the cues reach at `rate`.
pub(crate) fn span(cues: &[Cue], rate: f64) -> usize {
    cues.iter()
        .map(|cue| bin_of(cue.end.millis(), rate))
        .max()
        .unwrap_or(0)
}

/// Which bin a moment in the authored clock falls in once stretched by `rate`.
///
/// Rounded down, so a cue covers the bins whose starts lie inside it. A cue
/// shorter than one bin therefore covers nothing, which is the honest answer at
/// this resolution and is what a cue of no length already means everywhere else.
// Both bounds are tested against the float before anything is narrowed, so the
// cast only ever sees a value between zero and the bound.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn bin_of(millis: u32, rate: f64) -> usize {
    let scaled = f64::from(millis) * rate / f64::from(BIN_MS);
    if scaled <= 0.0 {
        0
    } else if scaled >= MAX_BINS as f64 {
        MAX_BINS
    } else {
        scaled as usize
    }
}

#[cfg(test)]
mod tests {
    use super::{BIN_MS, MAX_BINS, Signal};
    use subtext_core::{Cue, Timestamp};

    fn cue(start: u32, end: u32) -> Cue {
        Cue {
            index: 1,
            start: Timestamp::from_millis(start),
            end: Timestamp::from_millis(end),
            text: "line".to_owned(),
            position: None,
        }
    }

    #[test]
    fn a_cue_covers_the_bins_it_spans() {
        let signal = Signal::from_cues(&[cue(1_000, 3_000)]);
        assert_eq!(signal.len(), 300);
        assert_eq!(signal.active(), 200);
        assert!(!signal.bins()[99]);
        assert!(signal.bins()[100]);
        assert!(signal.bins()[299]);
    }

    #[test]
    fn overlapping_cues_are_counted_once() {
        let overlapping = Signal::from_cues(&[cue(1_000, 3_000), cue(2_000, 4_000)]);
        assert_eq!(overlapping.active(), 300);
        assert_eq!(overlapping.len(), 400);
    }

    #[test]
    fn cues_out_of_order_give_the_same_signal_as_cues_in_order() {
        let ordered = Signal::from_cues(&[cue(1_000, 2_000), cue(5_000, 6_000)]);
        let jumbled = Signal::from_cues(&[cue(5_000, 6_000), cue(1_000, 2_000)]);
        assert_eq!(ordered, jumbled);
    }

    #[test]
    fn a_cue_shorter_than_a_bin_covers_nothing() {
        let signal = Signal::from_cues(&[cue(1_000, 1_000 + BIN_MS - 1)]);
        assert_eq!(signal.active(), 0);
    }

    #[test]
    fn no_cues_give_an_empty_signal() {
        let signal = Signal::from_cues(&[]);
        assert!(signal.is_empty());
        assert!(signal.is_flat());
    }

    #[test]
    fn a_rate_stretches_where_the_cues_fall() {
        let mut signal = Signal::from_cues(&[cue(10_000, 11_000)]);
        signal.rebuild(&[cue(10_000, 11_000)], 2.0);
        assert_eq!(signal.len(), 2_200);
        assert!(signal.bins()[2_000]);
        assert!(!signal.bins()[1_999]);
    }

    #[test]
    fn rebuilding_leaves_nothing_of_the_previous_rate_behind() {
        let cues = [cue(10_000, 11_000)];
        let mut reused = Signal::from_cues(&cues);
        reused.rebuild(&cues, 2.0);
        reused.rebuild(&cues, 1.0);
        assert_eq!(reused, Signal::from_cues(&cues));
    }

    #[test]
    fn a_timestamp_beyond_any_film_cannot_decide_the_size_of_the_buffers() {
        let signal = Signal::from_cues(&[cue(0, u32::MAX)]);
        assert_eq!(signal.len(), MAX_BINS);
    }

    #[test]
    fn bins_from_elsewhere_are_bounded_the_same_way() {
        let signal = Signal::from_bins(vec![true; MAX_BINS + 1_000]);
        assert_eq!(signal.len(), MAX_BINS);
        assert_eq!(signal.active(), MAX_BINS);
        assert!(signal.is_flat());
    }

    #[test]
    fn a_signal_saying_the_same_thing_everywhere_is_flat() {
        assert!(Signal::from_bins(vec![false; 100]).is_flat());
        assert!(Signal::from_bins(vec![true; 100]).is_flat());

        let mut bins = vec![false; 100];
        bins[50] = true;
        assert!(!Signal::from_bins(bins).is_flat());
    }
}
