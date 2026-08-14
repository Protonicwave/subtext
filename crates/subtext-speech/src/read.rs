//! One pass over a film's audio, turned into energy against a clock.

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Duration, Time, TimeBase, Timestamp};

use crate::band::Band;
use crate::codec;
use crate::{BIN_MS, Progress, Reading, Refusal};

/// How long a film has to be before decoding it in parallel is worth the cost
/// of opening and demuxing it several times over, in milliseconds.
///
/// Five minutes. Below it the setup is a noticeable share of the work and the
/// answer arrives quickly either way; above it the work is all decoding and
/// divides cleanly.
const SPLIT_MS: u64 = 5 * 60 * 1_000;

/// The least a single range may cover, in milliseconds.
///
/// A minute. Eight cores over a six minute film is not eight times the speed,
/// it is eight demuxers competing for the same disc, so the count of ranges
/// falls away with the length of the film rather than the count of cores.
const RANGE_MS: u64 = 60 * 1_000;

/// How far before its range a run starts decoding, in milliseconds.
///
/// A seek lands on a packet boundary and a decoder needs a moment to settle
/// after one, as does the band filter. Everything decoded before the range
/// begins falls outside the bins the run owns and is dropped, so the only cost
/// is the decoding itself.
const PREROLL_MS: u64 = 500;

/// How many times over a whole film progress is reported.
///
/// Two hundred, which is often enough that a bar moves smoothly and a request
/// to stop is noticed inside a tenth of a second, and rare enough that the
/// front end is not asked to redraw for every packet in a film.
const PROGRESS_STEPS: usize = 200;

/// What the header says about the audio being read.
#[derive(Clone, Copy, Debug)]
struct Audio {
    track_id: u32,
    time_base: TimeBase,
    /// How many bins the film runs to, where the header says so.
    bins: Option<usize>,
    /// Whether the film can be picked up from the middle.
    seekable: bool,
}

/// How loud each bin of a film is, and how much of it was heard.
///
/// Two runs rather than one, because a bin at the tail of a film or across a
/// packet the decoder threw away holds fewer samples than the rest, and dividing
/// by what actually arrived is the difference between a quiet moment and a
/// missing one.
#[derive(Debug)]
pub(crate) struct Energy {
    from: usize,
    sums: Vec<f32>,
    counts: Vec<u32>,
    /// How far this run may reach, where it is one of several.
    limit: Option<usize>,
}

impl Energy {
    fn new(from: usize, limit: Option<usize>) -> Self {
        let size = limit.unwrap_or(0);
        Self {
            from,
            sums: vec![0.0; size],
            counts: vec![0; size],
            limit,
        }
    }

    /// Records a bin, saying whether the run has anything left to do.
    fn add(&mut self, bin: usize, energy: f32, samples: u32) -> bool {
        // Before the range is the preroll, which is decoded to settle the
        // decoder and the filter and is not part of the answer.
        let Some(at) = bin.checked_sub(self.from) else {
            return true;
        };
        if let Some(limit) = self.limit {
            if at >= limit {
                return false;
            }
        } else if at >= self.sums.len() {
            // A film whose header does not say how long it is, which is the one
            // case where the size is not known before the reading starts.
            self.sums.resize(at + 1, 0.0);
            self.counts.resize(at + 1, 0);
        }

        self.sums[at] += energy;
        self.counts[at] += samples;
        true
    }

    /// How loud each bin was, in decibels.
    pub(crate) fn levels(&self) -> Vec<f32> {
        self.sums
            .iter()
            .zip(&self.counts)
            .map(|(sum, count)| crate::floor::level(*sum, *count))
            .collect()
    }

    /// Lays the runs of a split read out end to end.
    fn join(runs: Vec<Self>, bins: usize) -> Self {
        let mut whole = Self::new(0, Some(bins));
        for run in runs {
            let to = (run.from + run.sums.len()).min(bins);
            if run.from < to {
                whole.sums[run.from..to].copy_from_slice(&run.sums[..to - run.from]);
                whole.counts[run.from..to].copy_from_slice(&run.counts[..to - run.from]);
            }
        }
        whole
    }
}

/// How far through a film the reading has got, shared by every run of a split
/// read.
struct Tally<'a> {
    done: AtomicUsize,
    reported: AtomicUsize,
    bins: usize,
    progress: &'a dyn Progress,
}

impl Tally<'_> {
    fn advance(&self, bins: usize) -> Reading {
        let done = self.done.fetch_add(bins, Ordering::Relaxed) + bins;
        if self.bins == 0 {
            return Reading::Continue;
        }

        let step = (done * PROGRESS_STEPS / self.bins).min(PROGRESS_STEPS);
        // Two runs crossing a step at once report twice, which costs a duplicate
        // event and is not worth a lock to avoid.
        if step <= self.reported.load(Ordering::Relaxed) {
            return Reading::Continue;
        }
        self.reported.store(step, Ordering::Relaxed);

        #[allow(clippy::cast_precision_loss)]
        let fraction = done.min(self.bins) as f32 / self.bins as f32;
        self.progress.read(fraction)
    }
}

/// How loud a film is, bin by bin, from the beginning to the end of it.
pub(crate) fn energy(path: &Path, progress: &dyn Progress) -> Result<Energy, Refusal> {
    let audio = header_of(path)?;

    let Some(bins) = audio.bins else {
        // Nothing in the header says how long the film is, so there is no way to
        // divide it up. One pass, growing as it goes.
        let tally = tally_for(0, progress);
        return read(path, &audio, 0, None, &tally);
    };

    let tally = tally_for(bins, progress);
    let ranges = ranges(bins, audio.seekable);
    if ranges == 1 {
        return read(path, &audio, 0, Some(bins), &tally);
    }

    let size = bins.div_ceil(ranges);
    let runs: Result<Vec<Energy>, Refusal> = (0..ranges)
        .into_par_iter()
        .map(|at| {
            let from = at * size;
            read(path, &audio, from, Some(size.min(bins - from)), &tally)
        })
        .collect();

    Ok(Energy::join(runs?, bins))
}

fn tally_for(bins: usize, progress: &dyn Progress) -> Tally<'_> {
    Tally {
        done: AtomicUsize::new(0),
        reported: AtomicUsize::new(0),
        bins,
        progress,
    }
}

/// How many runs to divide a film of `bins` between.
fn ranges(bins: usize, seekable: bool) -> usize {
    let length = bins as u64 * u64::from(BIN_MS);
    if !seekable || length < SPLIT_MS {
        return 1;
    }

    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    #[allow(clippy::cast_possible_truncation)]
    let most = (length / RANGE_MS) as usize;
    cores.min(most).max(1)
}

/// Opens a film and reads what its header says about its audio.
fn header_of(path: &Path) -> Result<Audio, Refusal> {
    let mut reader = open(path)?;
    let track = reader
        .default_track(TrackType::Audio)
        .ok_or(Refusal::NoAudio)?;

    let Some(CodecParameters::Audio(params)) = &track.codec_params else {
        return Err(Refusal::NoAudio);
    };

    // Asked here rather than when a run reaches it, so that a film nothing can
    // read is refused before any of the work is started.
    if symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .is_err()
    {
        return Err(Refusal::Codec {
            name: codec::name_of(params.codec).map(str::to_owned),
        });
    }

    let track_id = track.id;
    let time_base = track
        .time_base
        .ok_or_else(|| Refusal::Unreadable("the audio track carries no clock".to_owned()))?;
    let rate = params.sample_rate;
    let bins = track
        .duration
        .map(|duration| bins_of(duration, time_base))
        .or_else(|| {
            let frames = track.num_frames?;
            #[allow(clippy::cast_possible_truncation)]
            Some((frames * 1_000 / u64::from(rate?) / u64::from(BIN_MS)) as usize)
        })
        // Matroska states how long a film is once for the whole segment rather
        // than on each track, so the track says nothing and the media does.
        .or_else(|| {
            let info = reader.media_info();
            Some(bins_of(info.duration?, info.time_base?))
        });

    // A film that cannot be picked up from the middle is read in one pass. Asked
    // once, since the answer is a property of the container rather than of where
    // in it the question is asked, and only of a film long enough for the answer
    // to change anything.
    let long_enough = bins.is_some_and(|bins| bins as u64 * u64::from(BIN_MS) >= SPLIT_MS);
    let seekable = long_enough
        && reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: time_of(SPLIT_MS / 2),
                    track_id: Some(track_id),
                },
            )
            .is_ok();

    Ok(Audio {
        track_id,
        time_base,
        bins,
        seekable,
    })
}

fn open(path: &Path) -> Result<Box<dyn FormatReader>, Refusal> {
    let file = File::open(path).map_err(|err| Refusal::Unreadable(err.to_string()))?;

    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(std::ffi::OsStr::to_str) {
        hint.with_extension(extension);
    }

    let stream = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
    symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|err| Refusal::Unreadable(err.to_string()))
}

/// Reads `limit` bins of a film from `from`, or all of it from `from` where
/// there is no limit.
fn read(
    path: &Path,
    audio: &Audio,
    from: usize,
    limit: Option<usize>,
    tally: &Tally<'_>,
) -> Result<Energy, Refusal> {
    let mut reader = open(path)?;
    let Some(track) = reader
        .tracks()
        .iter()
        .find(|track| track.id == audio.track_id)
    else {
        return Err(Refusal::NoAudio);
    };
    let Some(CodecParameters::Audio(params)) = &track.codec_params else {
        return Err(Refusal::NoAudio);
    };

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|_| Refusal::Codec {
            name: codec::name_of(params.codec).map(str::to_owned),
        })?;

    if from > 0 {
        let start = (from as u64 * u64::from(BIN_MS)).saturating_sub(PREROLL_MS);
        reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: time_of(start),
                    track_id: Some(audio.track_id),
                },
            )
            .map_err(|err| Refusal::Unreadable(err.to_string()))?;
        decoder.reset();
    }

    let mut energy = Energy::new(from, limit);
    let mut band: Option<Band> = None;
    let mut samples: Vec<f32> = Vec::new();
    let mut counted: u64 = 0;

    // The loop ends at the end of the film, or at the end of what there is of
    // it. A download that stopped half way through is a shorter film rather than
    // a failure, and so is a container whose last cluster is a lie.
    while let Ok(Some(packet)) = reader.next_packet() {
        if packet.track_id != audio.track_id {
            continue;
        }

        let start = millis_of(packet.pts, audio.time_base);
        let heard = match decoder.decode(&packet) {
            Ok(heard) => heard,
            // One packet the decoder would not take is one moment of the film
            // measured as quieter than it was, which the bins either side of it
            // will outvote.
            Err(Error::DecodeError(_) | Error::IoError(_)) => continue,
            Err(_) => break,
        };

        let rate = heard.spec().rate();
        let channels = heard.spec().channels().count();
        if rate == 0 || channels == 0 {
            continue;
        }
        heard.copy_to_vec_interleaved(&mut samples);

        let band = band.get_or_insert_with(|| Band::new(rate));
        let frames = samples.len() / channels;
        if !fill(&mut energy, band, &samples, channels, rate, start) {
            break;
        }

        // Counted in bins so that the two sides of a split film advance the same
        // bar at the same rate, whatever their packets happen to hold.
        counted += frames as u64;
        let per_bin = u64::from(rate) * u64::from(BIN_MS);
        let bins = counted * 1_000 / per_bin;
        if bins > 0 {
            counted -= bins * per_bin / 1_000;
            if tally.advance(usize::try_from(bins).unwrap_or(usize::MAX)) == Reading::Stop {
                return Err(Refusal::Stopped);
            }
        }
    }

    Ok(energy)
}

/// Adds one packet of samples to the bins they fall in, saying whether the run
/// has anything left to do.
///
/// The samples are walked a bin at a time rather than one at a time, so that
/// working out which bin a sample belongs to happens once for every few hundred
/// of them. Each bin is placed from the packet's own timestamp, so a rate that
/// does not divide into a bin cannot accumulate a drift beyond one packet.
fn fill(
    energy: &mut Energy,
    band: &mut Band,
    samples: &[f32],
    channels: usize,
    rate: u32,
    start_ms: f64,
) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let rate = f64::from(rate);
    let frames = samples.len() / channels;
    let per_bin = rate * f64::from(BIN_MS) / 1_000.0;

    let mut at = 0;
    while at < frames {
        #[allow(clippy::cast_precision_loss)]
        let along = (start_ms + at as f64 * 1_000.0 / rate) / f64::from(BIN_MS);
        // A packet before the start of the film, which happens where a container
        // carries a negative timestamp for the encoder's delay.
        if along < 0.0 {
            return true;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bin = along.floor() as usize;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_sign_loss
        )]
        let take = (((bin + 1) as f64 - along) * per_bin).ceil() as usize;
        let take = take.clamp(1, frames - at);

        let mut sum = 0.0f32;
        for frame in samples[at * channels..(at + take) * channels].chunks_exact(channels) {
            // Downmixed by averaging. A voice sits in the centre of a film mix
            // and so appears in both of a stereo pair, where averaging keeps it;
            // anything out of phase between them is not dialogue.
            #[allow(clippy::cast_precision_loss)]
            let mono = frame.iter().sum::<f32>() / channels as f32;
            let heard = band.apply(mono);
            sum += heard * heard;
        }

        #[allow(clippy::cast_possible_truncation)]
        if !energy.add(bin, sum, take as u32) {
            return false;
        }
        at += take;
    }

    true
}

/// Where a timestamp falls, in milliseconds.
#[allow(clippy::cast_precision_loss)]
fn millis_of(ts: Timestamp, time_base: TimeBase) -> f64 {
    let ts = ts.get() as f64;
    ts * f64::from(time_base.numer.get()) * 1_000.0 / f64::from(time_base.denom.get())
}

/// How many bins a length of film covers.
fn bins_of(duration: Duration, time_base: TimeBase) -> usize {
    let ticks = i64::try_from(duration.get()).unwrap_or(i64::MAX);
    bin_of(Timestamp::new(ticks), time_base)
}

/// Which bin a timestamp falls in.
fn bin_of(ts: Timestamp, time_base: TimeBase) -> usize {
    let millis = millis_of(ts, time_base).max(0.0) / f64::from(BIN_MS);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    if millis >= usize::MAX as f64 {
        usize::MAX
    } else {
        millis as usize
    }
}

/// A moment in the film, from milliseconds.
fn time_of(millis: u64) -> Time {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let seconds = (millis / 1_000) as i64;
    #[allow(clippy::cast_possible_truncation)]
    let nanos = ((millis % 1_000) * 1_000_000) as u32;
    Time::try_new(seconds, nanos).unwrap_or(Time::ZERO)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::cast_possible_truncation, clippy::float_cmp)]

    use super::{Energy, RANGE_MS, SPLIT_MS, ranges};
    use crate::BIN_MS;

    fn bins_of(millis: u64) -> usize {
        (millis / u64::from(BIN_MS)) as usize
    }

    #[test]
    fn a_short_film_is_read_in_one_pass() {
        assert_eq!(ranges(bins_of(SPLIT_MS - 1), true), 1);
    }

    #[test]
    fn a_film_that_cannot_be_seeked_is_read_in_one_pass() {
        assert_eq!(ranges(bins_of(4 * 60 * 60 * 1_000), false), 1);
    }

    #[test]
    fn no_range_is_shorter_than_a_minute() {
        let bins = bins_of(6 * RANGE_MS);
        assert!(ranges(bins, true) <= 6);
    }

    #[test]
    fn a_long_film_is_divided_up() {
        // Only where there is more than one core to divide it between, which a
        // build machine cannot be relied on to have.
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        assert_eq!(ranges(bins_of(4 * 60 * 60 * 1_000), true), cores);
    }

    #[test]
    fn a_run_keeps_only_the_bins_it_owns() {
        let mut energy = Energy::new(100, Some(10));
        // Before its range, which is the preroll a seek leaves in front of it.
        assert!(energy.add(50, 1.0, 1));
        assert!(energy.add(100, 4.0, 2));
        // Past the end of its range, which is where the next run takes over.
        assert!(!energy.add(110, 1.0, 1));

        assert_eq!(energy.sums[0], 4.0);
        assert_eq!(energy.counts[0], 2);
        assert_eq!(energy.sums.iter().sum::<f32>(), 4.0);
    }

    #[test]
    fn a_film_of_unknown_length_grows_to_fit() {
        let mut energy = Energy::new(0, None);
        assert!(energy.add(5, 1.0, 1));
        assert_eq!(energy.sums.len(), 6);
        assert_eq!(energy.levels().len(), 6);
    }

    #[test]
    fn runs_are_laid_out_end_to_end() {
        let mut first = Energy::new(0, Some(3));
        first.add(1, 1.0, 1);
        let mut second = Energy::new(3, Some(3));
        second.add(4, 2.0, 1);

        let whole = Energy::join(vec![first, second], 6);
        assert_eq!(whole.sums, vec![0.0, 1.0, 0.0, 0.0, 2.0, 0.0]);
    }
}
