//! Building small films to listen to.
//!
//! Public, and deliberately so. Testing something that reads audio means having
//! audio to read, and the alternative is committing binaries: a few megabytes of
//! opaque file in the repository for every case, which nobody can review and
//! which say nothing about what they contain. A film built here says in one line
//! where it is supposed to be talking, which is the only thing the tests care
//! about.
//!
//! The audio is uncompressed, because the point of a fixture is to know exactly
//! what went in. Writing an encoder to test a decoder would be a great deal of
//! code whose own correctness nothing checks, and it would test the encoder. The
//! codecs a film actually arrives in are read on real files, and the boundary
//! this crate has to get right is not the decoding, which Symphonia does, but
//! choosing the track, placing the samples against the clock, and refusing what
//! cannot be read.

// A fixture is built entirely from values this module chose, so the arithmetic
// that lays one out cannot overflow or lose anything that matters. Saying so at
// every cast would bury what the files actually contain.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::f64::consts::TAU;

/// The pitch a burst is written at, in hertz.
///
/// Seven hundred, which is inside the band the reading keeps and well clear of
/// both corners, so a burst that goes missing means the bins were wrong rather
/// than the filter.
const TONE_HZ: f64 = 700.0;

/// How loud a burst is, against full scale.
const TONE: f64 = 0.5;

/// How many times a second a burst swells and falls away, in hertz.
///
/// Four, which is the rate a person produces syllables at. A burst has to have
/// it because it is what the reading looks for: a steady tone is a note, and a
/// fixture made of steady tones would be a test of something the product does
/// not claim to find.
const SYLLABLE_HZ: f64 = 4.0;

/// How much of a burst's loudness the swell accounts for.
///
/// Most of it, but not all, so that a burst never falls to silence in the middle
/// of itself. A gap inside a burst would be a gap the fixture did not ask for,
/// and the test would then be measuring where the reading closed it up rather
/// than where the burst was.
const SWELL: f64 = 0.45;

/// How loud the film is when nobody is talking, against full scale.
///
/// Real films are never digitally silent. Room tone, tape hiss and the noise
/// floor of the transfer are always there, and a fixture of perfect silence
/// would be a film no floor could be measured from.
const ROOM: f64 = 0.002;

/// How long one block of samples covers, in milliseconds.
///
/// A hundred, which is what a container carrying uncompressed audio uses. It has
/// to be several bins so that placing a bin means reading into a block rather
/// than taking a block's own timestamp.
const BLOCK_MS: u32 = 100;

/// How long one cluster covers, in milliseconds.
const CLUSTER_MS: u32 = 1_000;

/// What a track claims to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Claim {
    /// Uncompressed, and true.
    Pcm,
    /// Something else, written over samples that are still uncompressed. The
    /// point of one of these is to be refused before anything tries to decode
    /// it, which is what a film with a codec this build does not read has to do.
    Other(&'static str),
}

/// A film with a soundtrack, to be written out and read back.
#[derive(Clone, Debug)]
pub struct Film {
    sample_rate: u32,
    channels: u16,
    length_ms: u32,
    speech: Vec<(u32, u32)>,
    claim: Claim,
    audio: bool,
}

impl Film {
    /// A film of `length_ms` with nobody talking in it.
    #[must_use]
    pub fn new(length_ms: u32) -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            length_ms,
            speech: Vec::new(),
            claim: Claim::Pcm,
            audio: true,
        }
    }

    /// Somebody talks between these two moments, in milliseconds.
    #[must_use]
    pub fn speaking(mut self, from_ms: u32, to_ms: u32) -> Self {
        self.speech.push((from_ms, to_ms));
        self
    }

    /// The audio is at this rate, with this many channels.
    #[must_use]
    pub fn recorded(mut self, sample_rate: u32, channels: u16) -> Self {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self
    }

    /// The track says it is in some other codec, whatever the samples are.
    ///
    /// Matroska only, since it is the codec's name in the header that is being
    /// tested and not anything under it.
    #[must_use]
    pub fn claiming(mut self, codec: &'static str) -> Self {
        self.claim = Claim::Other(codec);
        self
    }

    /// The film has a picture and no soundtrack at all.
    #[must_use]
    pub fn without_audio(mut self) -> Self {
        self.audio = false;
        self
    }

    /// Which bins of this film somebody is talking in, as the fixture wrote it.
    ///
    /// What a reading of the file has to come back with, give or take the edges.
    #[must_use]
    pub fn spoken_bins(&self, bin_ms: u32) -> Vec<(usize, usize)> {
        self.speech
            .iter()
            .map(|(from, to)| ((from / bin_ms) as usize, (to / bin_ms) as usize))
            .collect()
    }

    /// How many samples per channel the film runs to.
    fn frames(&self) -> usize {
        (u64::from(self.length_ms) * u64::from(self.sample_rate) / 1_000) as usize
    }

    /// How many frames are in one block.
    fn block_frames(&self) -> usize {
        (u64::from(BLOCK_MS) * u64::from(self.sample_rate) / 1_000) as usize
    }

    /// The soundtrack, as interleaved sixteen bit samples.
    fn samples(&self) -> Vec<u8> {
        let frames = self.frames();
        let mut out = Vec::with_capacity(frames * usize::from(self.channels) * 2);

        // Walked in order alongside the samples rather than searched at each one,
        // since a film with a line every few seconds holds hundreds of these and
        // a soundtrack holds millions of samples.
        let mut speech = self.speech.clone();
        speech.sort_unstable();
        let mut next = 0;

        // A fixed sequence rather than a random one, so that a failing test fails
        // the same way twice.
        let mut noise: u32 = 0x2545_F491;
        for frame in 0..frames {
            let at = frame as f64 / f64::from(self.sample_rate);
            let ms = (at * 1_000.0) as u32;
            while next < speech.len() && ms >= speech[next].1 {
                next += 1;
            }
            let speaking = next < speech.len() && ms >= speech[next].0;

            noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let hiss = (f64::from(noise >> 16) / f64::from(u16::MAX) - 0.5) * 2.0 * ROOM;
            let value = if speaking {
                let swell = 1.0 - SWELL + SWELL * (TAU * SYLLABLE_HZ * at).sin();
                (TAU * TONE_HZ * at).sin() * TONE * swell + hiss
            } else {
                hiss
            };

            let sample = (value.clamp(-1.0, 1.0) * f64::from(i16::MAX)) as i16;
            for _ in 0..self.channels {
                out.extend_from_slice(&sample.to_le_bytes());
            }
        }

        out
    }
}

mod build {
    use super::{BLOCK_MS, CLUSTER_MS, Claim, Film};

    /// The elements a Matroska fixture is built from.
    const EBML_HEADER: u32 = 0x1A45_DFA3;
    const DOC_TYPE: u32 = 0x4282;
    const SEGMENT: u32 = 0x1853_8067;
    const INFO: u32 = 0x1549_A966;
    const TIMESTAMP_SCALE: u32 = 0x002A_D7B1;
    const DURATION: u32 = 0x4489;
    const MUXING_APP: u32 = 0x4D80;
    const WRITING_APP: u32 = 0x5741;
    const TRACKS: u32 = 0x1654_AE6B;
    const TRACK_ENTRY: u32 = 0xAE;
    const TRACK_NUMBER: u32 = 0xD7;
    const TRACK_UID: u32 = 0x73C5;
    const TRACK_TYPE: u32 = 0x83;
    const FLAG_DEFAULT: u32 = 0x88;
    const CODEC_ID: u32 = 0x86;
    const AUDIO: u32 = 0xE1;
    const SAMPLING_FREQUENCY: u32 = 0xB5;
    const CHANNELS: u32 = 0x9F;
    const BIT_DEPTH: u32 = 0x6264;
    const CLUSTER: u32 = 0x1F43_B675;
    const TIMESTAMP: u32 = 0xE7;
    const SIMPLE_BLOCK: u32 = 0xA3;
    const CUES: u32 = 0x1C53_BB6B;
    const CUE_POINT: u32 = 0xBB;
    const CUE_TIME: u32 = 0xB3;
    const CUE_TRACK_POSITIONS: u32 = 0xB7;
    const CUE_TRACK: u32 = 0xF7;
    const CUE_CLUSTER_POSITION: u32 = 0xF1;

    /// The track types, of which the first two are all that is wanted here.
    const TRACK_TYPE_VIDEO: u64 = 1;
    const TRACK_TYPE_AUDIO: u64 = 2;

    /// One unit of a Matroska timestamp, in nanoseconds, which is a millisecond.
    const SCALE_NS: u64 = 1_000_000;

    /// The number the audio track is given.
    const TRACK: u64 = 1;

    impl Film {
        /// The film as a Matroska file.
        #[must_use]
        pub fn matroska(&self) -> Vec<u8> {
            let mut header = Vec::new();
            element(&mut header, DOC_TYPE, b"matroska");

            let mut about = Vec::new();
            element(&mut about, TIMESTAMP_SCALE, &uint(SCALE_NS));
            element(&mut about, DURATION, &float(f64::from(self.length_ms)));
            // Both are required, and a reader is within its rights to refuse a
            // segment with no account of what wrote it.
            element(&mut about, MUXING_APP, b"subtext");
            element(&mut about, WRITING_APP, b"subtext");
            let mut info = Vec::new();
            element(&mut info, INFO, &about);

            let mut entries = Vec::new();
            element(&mut entries, TRACK_ENTRY, &self.track_entry());
            let mut tracks = Vec::new();
            element(&mut tracks, TRACKS, &entries);

            let (clusters, at) = self.clusters();

            // Written before the clusters rather than after them, so that a
            // reader has the index in hand by the time it wants to seek. The
            // positions inside it are counted from the start of the segment's
            // contents, so the index has to be measured before it can be
            // written; every position is a fixed eight bytes wide, which makes
            // the two passes the same length.
            let size = cues(&at, 0).len();
            let ahead = (info.len() + tracks.len() + size) as u64;
            let cues = cues(&at, ahead);
            debug_assert_eq!(cues.len(), size);

            let mut segment = info;
            segment.extend_from_slice(&tracks);
            segment.extend_from_slice(&cues);
            segment.extend_from_slice(&clusters);

            let mut out = Vec::new();
            element(&mut out, EBML_HEADER, &header);
            element(&mut out, SEGMENT, &segment);
            out
        }

        fn track_entry(&self) -> Vec<u8> {
            let mut entry = Vec::new();
            element(&mut entry, TRACK_NUMBER, &uint(TRACK));
            element(&mut entry, TRACK_UID, &uint(TRACK));
            element(&mut entry, FLAG_DEFAULT, &uint(1));

            if !self.audio {
                element(&mut entry, TRACK_TYPE, &uint(TRACK_TYPE_VIDEO));
                element(&mut entry, CODEC_ID, b"V_MPEG4/ISO/AVC");
                return entry;
            }

            element(&mut entry, TRACK_TYPE, &uint(TRACK_TYPE_AUDIO));
            let codec = match self.claim {
                Claim::Pcm => "A_PCM/INT/LIT",
                Claim::Other(codec) => codec,
            };
            element(&mut entry, CODEC_ID, codec.as_bytes());

            let mut audio = Vec::new();
            element(
                &mut audio,
                SAMPLING_FREQUENCY,
                &float(f64::from(self.sample_rate)),
            );
            element(&mut audio, CHANNELS, &uint(u64::from(self.channels)));
            element(&mut audio, BIT_DEPTH, &uint(16));
            element(&mut entry, AUDIO, &audio);

            entry
        }

        /// The clusters, and where each one starts within them.
        fn clusters(&self) -> (Vec<u8>, Vec<(u64, u64)>) {
            let mut out = Vec::new();
            let mut at = Vec::new();
            if !self.audio {
                return (out, at);
            }

            let samples = self.samples();
            let bytes_per_frame = usize::from(self.channels) * 2;
            let block_frames = self.block_frames();
            let blocks_per_cluster = (CLUSTER_MS / BLOCK_MS) as usize;
            let blocks = samples.len().div_ceil(block_frames * bytes_per_frame);

            for first in (0..blocks).step_by(blocks_per_cluster) {
                let start_ms = (first * BLOCK_MS as usize) as u64;
                let mut cluster = Vec::new();
                element(&mut cluster, TIMESTAMP, &uint(start_ms));

                for block in first..(first + blocks_per_cluster).min(blocks) {
                    let from = block * block_frames * bytes_per_frame;
                    let to = ((block + 1) * block_frames * bytes_per_frame).min(samples.len());

                    let mut payload = Vec::new();
                    write_size(&mut payload, TRACK);
                    let offset = ((block - first) as u32 * BLOCK_MS) as i16;
                    payload.extend_from_slice(&offset.to_be_bytes());
                    // Keyframe, which every block of audio is.
                    payload.push(0x80);
                    payload.extend_from_slice(&samples[from..to]);
                    element(&mut cluster, SIMPLE_BLOCK, &payload);
                }

                at.push((start_ms, out.len() as u64));
                element(&mut out, CLUSTER, &cluster);
            }

            (out, at)
        }
    }

    /// The index, with every cluster's position offset by `ahead`.
    fn cues(at: &[(u64, u64)], ahead: u64) -> Vec<u8> {
        let mut cues = Vec::new();
        for (time, position) in at {
            let mut positions = Vec::new();
            element(&mut positions, CUE_TRACK, &uint(TRACK));
            // Always eight bytes, so that the index is the same length however
            // large the positions inside it turn out to be.
            element(
                &mut positions,
                CUE_CLUSTER_POSITION,
                &(position + ahead).to_be_bytes(),
            );

            let mut point = Vec::new();
            element(&mut point, CUE_TIME, &uint(*time));
            element(&mut point, CUE_TRACK_POSITIONS, &positions);
            element(&mut cues, CUE_POINT, &point);
        }

        let mut out = Vec::new();
        element(&mut out, CUES, &cues);
        out
    }

    /// An element, being its identifier, its length and its contents.
    fn element(out: &mut Vec<u8>, id: u32, payload: &[u8]) {
        let bytes = id.to_be_bytes();
        let from = bytes.iter().position(|byte| *byte != 0).unwrap_or(3);
        out.extend_from_slice(&bytes[from..]);
        write_size(out, payload.len() as u64);
        out.extend_from_slice(payload);
    }

    /// A length, in as few bytes as will hold it.
    fn write_size(out: &mut Vec<u8>, size: u64) {
        for length in 1..=8u32 {
            // All ones is reserved for a length nobody knows, so the largest a
            // run of this many bytes can state is one below it.
            let most = (1u64 << (7 * length)) - 2;
            if size <= most {
                let value = (1u64 << (7 * length)) | size;
                out.extend_from_slice(&value.to_be_bytes()[8 - length as usize..]);
                return;
            }
        }
    }

    fn uint(value: u64) -> Vec<u8> {
        let bytes = value.to_be_bytes();
        let from = bytes.iter().position(|byte| *byte != 0).unwrap_or(7);
        bytes[from..].to_vec()
    }

    fn float(value: f64) -> Vec<u8> {
        value.to_be_bytes().to_vec()
    }
}

mod mp4 {
    use super::Film;

    /// The handler an audio track carries.
    const SOUND: [u8; 4] = *b"soun";

    /// Sixteen bit signed samples, little endian, which is what the fixture
    /// writes and what a Macintosh has called `sowt` since long before MP4.
    const SAMPLE_ENTRY: [u8; 4] = *b"sowt";

    /// The language nobody has said, which is what a fixture's is.
    const UNDETERMINED: u16 = 0x55C4;

    impl Film {
        /// The film as an MP4 file.
        ///
        /// Which is the shape a single file rip arrives in, and so the shape
        /// most likely to want aligning: a film with no container to carry a
        /// subtitle track inside it and nothing beside it but an SRT somebody
        /// found.
        #[must_use]
        pub fn mp4(&self) -> Vec<u8> {
            let samples = if self.audio {
                self.samples()
            } else {
                Vec::new()
            };
            let block = self.block_frames() * usize::from(self.channels) * 2;
            let blocks = if block == 0 {
                0
            } else {
                samples.len().div_ceil(block)
            };

            let mut out = Vec::new();
            atom(&mut out, *b"ftyp", &{
                let mut ftyp = Vec::new();
                ftyp.extend_from_slice(b"isom");
                ftyp.extend_from_slice(&512u32.to_be_bytes());
                ftyp.extend_from_slice(b"isomiso2mp41");
                ftyp
            });

            // Where the samples land depends on how long the header is, and the
            // header states where the samples land. The way out is that every
            // offset in it is a fixed four bytes, so a first pass with the
            // offset unknown is exactly as long as the second pass with it
            // filled in.
            let size = self.moov(blocks, block, 0).len();
            let mdat = (out.len() + size + 8) as u32;
            let moov = self.moov(blocks, block, mdat);
            debug_assert_eq!(moov.len(), size);

            out.extend_from_slice(&moov);
            atom(&mut out, *b"mdat", &samples);
            out
        }

        fn moov(&self, blocks: usize, block: usize, mdat: u32) -> Vec<u8> {
            let mut mvhd = version(0);
            mvhd.extend_from_slice(&0u32.to_be_bytes());
            mvhd.extend_from_slice(&0u32.to_be_bytes());
            mvhd.extend_from_slice(&1_000u32.to_be_bytes());
            mvhd.extend_from_slice(&self.length_ms.to_be_bytes());
            mvhd.extend_from_slice(&0x0001_0000u32.to_be_bytes());
            mvhd.extend_from_slice(&0x0100u16.to_be_bytes());
            mvhd.extend_from_slice(&[0; 10]);
            mvhd.extend_from_slice(&matrix());
            mvhd.extend_from_slice(&[0; 24]);
            mvhd.extend_from_slice(&2u32.to_be_bytes());

            let mut moov = Vec::new();
            atom(&mut moov, *b"mvhd", &mvhd);
            atom(&mut moov, *b"trak", &self.trak(blocks, block, mdat));
            let mut out = Vec::new();
            atom(&mut out, *b"moov", &moov);
            out
        }

        fn trak(&self, blocks: usize, block: usize, mdat: u32) -> Vec<u8> {
            // Enabled and in the presentation.
            let mut tkhd = version_flags(0, 3);
            tkhd.extend_from_slice(&0u32.to_be_bytes());
            tkhd.extend_from_slice(&0u32.to_be_bytes());
            tkhd.extend_from_slice(&1u32.to_be_bytes());
            tkhd.extend_from_slice(&0u32.to_be_bytes());
            tkhd.extend_from_slice(&self.length_ms.to_be_bytes());
            tkhd.extend_from_slice(&[0; 8]);
            tkhd.extend_from_slice(&0u16.to_be_bytes());
            tkhd.extend_from_slice(&0u16.to_be_bytes());
            tkhd.extend_from_slice(&0x0100u16.to_be_bytes());
            tkhd.extend_from_slice(&0u16.to_be_bytes());
            tkhd.extend_from_slice(&matrix());
            tkhd.extend_from_slice(&0u32.to_be_bytes());
            tkhd.extend_from_slice(&0u32.to_be_bytes());

            let mut trak = Vec::new();
            atom(&mut trak, *b"tkhd", &tkhd);
            atom(&mut trak, *b"mdia", &self.mdia(blocks, block, mdat));
            trak
        }

        fn mdia(&self, blocks: usize, block: usize, mdat: u32) -> Vec<u8> {
            // The track's own clock, which for audio is the sample rate, so a
            // timestamp is a count of samples.
            let mut mdhd = version(0);
            mdhd.extend_from_slice(&0u32.to_be_bytes());
            mdhd.extend_from_slice(&0u32.to_be_bytes());
            mdhd.extend_from_slice(&self.sample_rate.to_be_bytes());
            mdhd.extend_from_slice(&(self.frames() as u32).to_be_bytes());
            mdhd.extend_from_slice(&UNDETERMINED.to_be_bytes());
            mdhd.extend_from_slice(&0u16.to_be_bytes());

            let mut hdlr = version(0);
            hdlr.extend_from_slice(&0u32.to_be_bytes());
            hdlr.extend_from_slice(&SOUND);
            hdlr.extend_from_slice(&[0; 12]);
            hdlr.push(0);

            let mut mdia = Vec::new();
            atom(&mut mdia, *b"mdhd", &mdhd);
            atom(&mut mdia, *b"hdlr", &hdlr);
            atom(&mut mdia, *b"minf", &self.minf(blocks, block, mdat));
            mdia
        }

        fn minf(&self, blocks: usize, block: usize, mdat: u32) -> Vec<u8> {
            let mut smhd = version(0);
            smhd.extend_from_slice(&0u16.to_be_bytes());
            smhd.extend_from_slice(&0u16.to_be_bytes());

            let mut dref = version(0);
            dref.extend_from_slice(&1u32.to_be_bytes());
            // The data is in this file rather than another one.
            atom(&mut dref, *b"url ", &version_flags(0, 1));
            let mut dinf = Vec::new();
            atom(&mut dinf, *b"dref", &dref);

            let mut minf = Vec::new();
            atom(&mut minf, *b"smhd", &smhd);
            atom(&mut minf, *b"dinf", &dinf);
            atom(&mut minf, *b"stbl", &self.stbl(blocks, block, mdat));
            minf
        }

        fn stbl(&self, blocks: usize, block: usize, mdat: u32) -> Vec<u8> {
            let mut entry = Vec::new();
            entry.extend_from_slice(&[0; 6]);
            entry.extend_from_slice(&1u16.to_be_bytes());
            entry.extend_from_slice(&0u16.to_be_bytes());
            entry.extend_from_slice(&[0; 6]);
            entry.extend_from_slice(&self.channels.to_be_bytes());
            entry.extend_from_slice(&16u16.to_be_bytes());
            entry.extend_from_slice(&0u16.to_be_bytes());
            entry.extend_from_slice(&0u16.to_be_bytes());
            // The rate as a fixed point number with sixteen bits either side.
            entry.extend_from_slice(&(self.sample_rate << 16).to_be_bytes());

            let mut description = version(0);
            description.extend_from_slice(&1u32.to_be_bytes());
            atom(&mut description, SAMPLE_ENTRY, &entry);

            // Every sample is a block of the same length, so each of these
            // tables is one row rather than one row per block.
            let mut times = version(0);
            times.extend_from_slice(&1u32.to_be_bytes());
            times.extend_from_slice(&(blocks as u32).to_be_bytes());
            times.extend_from_slice(&(self.block_frames() as u32).to_be_bytes());

            let mut chunks = version(0);
            chunks.extend_from_slice(&1u32.to_be_bytes());
            chunks.extend_from_slice(&1u32.to_be_bytes());
            chunks.extend_from_slice(&(blocks.max(1) as u32).to_be_bytes());
            chunks.extend_from_slice(&1u32.to_be_bytes());

            let mut sizes = version(0);
            sizes.extend_from_slice(&(block as u32).to_be_bytes());
            sizes.extend_from_slice(&(blocks as u32).to_be_bytes());

            let mut offsets = version(0);
            offsets.extend_from_slice(&1u32.to_be_bytes());
            offsets.extend_from_slice(&mdat.to_be_bytes());

            let mut stbl = Vec::new();
            atom(&mut stbl, *b"stsd", &description);
            atom(&mut stbl, *b"stts", &times);
            atom(&mut stbl, *b"stsc", &chunks);
            atom(&mut stbl, *b"stsz", &sizes);
            atom(&mut stbl, *b"stco", &offsets);
            stbl
        }
    }

    /// A box, being its length, its name and its contents.
    fn atom(out: &mut Vec<u8>, name: [u8; 4], payload: &[u8]) {
        out.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
        out.extend_from_slice(&name);
        out.extend_from_slice(payload);
    }

    fn version(version: u8) -> Vec<u8> {
        version_flags(version, 0)
    }

    fn version_flags(version: u8, flags: u32) -> Vec<u8> {
        let mut out = vec![version];
        out.extend_from_slice(&flags.to_be_bytes()[1..]);
        out
    }

    /// The transformation a track is displayed through, which for a soundtrack
    /// is the one that changes nothing.
    fn matrix() -> [u8; 36] {
        let mut matrix = [0u8; 36];
        matrix[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        matrix[16..20].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        matrix[32..36].copy_from_slice(&0x4000_0000u32.to_be_bytes());
        matrix
    }
}

#[cfg(test)]
mod tests {
    use super::Film;

    #[test]
    fn a_film_carries_the_samples_it_was_asked_for() {
        let film = Film::new(1_000).recorded(8_000, 1);
        assert_eq!(film.frames(), 8_000);
        assert_eq!(film.samples().len(), 8_000 * 2);
    }

    #[test]
    fn a_burst_is_louder_than_the_room_around_it() {
        let film = Film::new(1_000).recorded(8_000, 1).speaking(400, 600);
        let samples = film.samples();
        let at = |frame: usize| i16::from_le_bytes([samples[frame * 2], samples[frame * 2 + 1]]);

        let quiet: i32 = (0..3_000).map(|frame| i32::from(at(frame)).abs()).sum();
        let loud: i32 = (3_400..4_600).map(|frame| i32::from(at(frame)).abs()).sum();
        assert!(loud > quiet);
    }

    #[test]
    fn a_film_is_written_in_both_containers() {
        let film = Film::new(2_000).speaking(500, 1_500);
        assert!(film.matroska().len() > 1_000);
        assert!(film.mp4().len() > 1_000);
    }

    #[test]
    fn a_film_with_no_soundtrack_carries_no_samples() {
        let film = Film::new(2_000).without_audio();
        assert!(film.matroska().len() < 1_000);
    }
}
