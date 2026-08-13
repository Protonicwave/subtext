//! Building small Matroska files to read back.
//!
//! Public, and deliberately so. Testing a reader of containers means having
//! containers to read, and producing one otherwise means depending on a muxer,
//! which is a large tool to install on three platforms of build machine to
//! produce a file of two hundred bytes. Writing the framing by hand is a page
//! of code, it says exactly what each fixture contains, and the scanner's tests
//! need the same thing a folder at a time.
//!
//! This writes headers only. There is no picture, no sound and no dialogue in
//! anything it produces, which is all a probe of the header has any use for.

/// The elements a fixture is built from.
const EBML_HEADER: u32 = 0x1A45_DFA3;
const DOC_TYPE: u32 = 0x4282;
const SEGMENT: u32 = 0x1853_8067;
const SEEK_HEAD: u32 = 0x114D_9B74;
const SEEK: u32 = 0x4DBB;
const SEEK_ID: u32 = 0x53AB;
const SEEK_POSITION: u32 = 0x53AC;
const INFO: u32 = 0x1549_A966;
const TIMESTAMP_SCALE: u32 = 0x002A_D7B1;
const VOID: u32 = 0xEC;
const CLUSTER: u32 = 0x1F43_B675;
const TRACKS: u32 = 0x1654_AE6B;
const TRACK_ENTRY: u32 = 0xAE;
const TRACK_NUMBER: u32 = 0xD7;
const TRACK_TYPE: u32 = 0x83;
const CODEC_ID: u32 = 0x86;
const LANGUAGE: u32 = 0x0022_B59C;
const FLAG_DEFAULT: u32 = 0x88;
const FLAG_FORCED: u32 = 0x55AA;
const FLAG_HEARING_IMPAIRED: u32 = 0x55AB;

/// The track types, of which only the last is a subtitle.
const VIDEO: u64 = 1;
const AUDIO: u64 = 2;
const SUBTITLE: u64 = 17;

/// One track in a fixture.
#[derive(Clone, Debug)]
pub struct Entry {
    kind: u64,
    number: u64,
    codec_id: String,
    language: Option<String>,
    default: bool,
    forced: bool,
    hearing_impaired: bool,
}

impl Entry {
    /// A subtitle track carrying the given codec identifier.
    #[must_use]
    pub fn subtitle(number: u64, codec_id: &str) -> Self {
        Self {
            kind: SUBTITLE,
            number,
            codec_id: codec_id.to_owned(),
            language: None,
            default: true,
            forced: false,
            hearing_impaired: false,
        }
    }

    /// A picture track, so that a fixture looks like a film rather than like a
    /// list of subtitles.
    #[must_use]
    pub fn video(number: u64) -> Self {
        Self {
            kind: VIDEO,
            ..Self::subtitle(number, "V_MPEG4/ISO/AVC")
        }
    }

    #[must_use]
    pub fn audio(number: u64) -> Self {
        Self {
            kind: AUDIO,
            ..Self::subtitle(number, "A_AAC")
        }
    }

    #[must_use]
    pub fn in_language(mut self, language: &str) -> Self {
        self.language = Some(language.to_owned());
        self
    }

    #[must_use]
    pub fn forced(mut self) -> Self {
        self.forced = true;
        self
    }

    #[must_use]
    pub fn hearing_impaired(mut self) -> Self {
        self.hearing_impaired = true;
        self
    }

    #[must_use]
    pub fn not_default(mut self) -> Self {
        self.default = false;
        self
    }

    fn bytes(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend(uint(TRACK_NUMBER, self.number));
        body.extend(uint(TRACK_TYPE, self.kind));
        body.extend(text(CODEC_ID, &self.codec_id));
        if let Some(language) = &self.language {
            body.extend(text(LANGUAGE, language));
        }
        // Written only when set, so that a fixture also covers what a file
        // saying nothing about a flag is taken to mean.
        if !self.default {
            body.extend(uint(FLAG_DEFAULT, 0));
        }
        if self.forced {
            body.extend(uint(FLAG_FORCED, 1));
        }
        if self.hearing_impaired {
            body.extend(uint(FLAG_HEARING_IMPAIRED, 1));
        }
        element(TRACK_ENTRY, &body)
    }
}

/// A Matroska file, as far as its header goes.
#[derive(Clone, Debug, Default)]
pub struct Container {
    entries: Vec<Entry>,
    seek_head: bool,
    padding: usize,
    cluster: Option<u64>,
    tracks_after_cluster: bool,
}

impl Container {
    #[must_use]
    pub fn new(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            ..Self::default()
        }
    }

    /// Adds the index that says where the tracks are.
    #[must_use]
    pub fn with_seek_head(mut self) -> Self {
        self.seek_head = true;
        self
    }

    /// Puts empty space in front of the tracks, which is what a muxer leaves
    /// itself room to add things in.
    #[must_use]
    pub fn with_padding(mut self, bytes: usize) -> Self {
        self.padding = bytes;
        self
    }

    /// Declares a cluster of frames of the given size without writing any of
    /// them, so that a fixture can claim to be a film of any length at all.
    ///
    /// What follows the header is the caller's business: this leaves the file
    /// saying there are that many bytes of picture next.
    #[must_use]
    pub fn with_declared_cluster(mut self, bytes: u64) -> Self {
        self.cluster = Some(bytes);
        self
    }

    /// Writes the tracks after the frames rather than before them, which is
    /// the case a seek head is the only way through.
    #[must_use]
    pub fn with_tracks_after_the_cluster(mut self) -> Self {
        self.tracks_after_cluster = true;
        self
    }

    /// The file, as bytes.
    ///
    /// A declared cluster is counted in the Segment's length without being
    /// written, so the file claims a size the bytes do not account for. That is
    /// the whole point of it: something else supplies the picture, and the
    /// probe is measured on how little of it it asks for.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let body = self.segment();
        let declared = length(body.len()) + self.cluster.unwrap_or_default();

        let mut file = element(EBML_HEADER, &text(DOC_TYPE, "matroska"));
        file.extend(header(SEGMENT, declared));
        file.extend(body);
        file
    }

    /// What is inside the Segment, with the seek head's position worked out
    /// against it.
    fn segment(&self) -> Vec<u8> {
        let head_length = if self.seek_head {
            seek_head_bytes(0).len()
        } else {
            0
        };

        let mut body = Vec::new();
        if self.padding > 0 {
            body.extend(element(VOID, &vec![0; self.padding]));
        }
        body.extend(element(INFO, &uint(TIMESTAMP_SCALE, 1_000_000)));
        if self.tracks_after_cluster {
            body.extend(element(CLUSTER, &[]));
        }

        let tracks_at = head_length + body.len();
        body.extend(element(TRACKS, &self.tracks()));
        if let Some(size) = self.cluster {
            body.extend(header(CLUSTER, size));
        }

        if !self.seek_head {
            return body;
        }
        let mut segment = seek_head_bytes(length(tracks_at));
        segment.extend(body);
        segment
    }

    fn tracks(&self) -> Vec<u8> {
        self.entries.iter().flat_map(Entry::bytes).collect()
    }
}

/// An index pointing at the tracks, `at` bytes into the Segment.
///
/// The position is written across a fixed eight bytes so that the index is the
/// same length whatever it points at, which is what lets the position be worked
/// out from a length that already includes it.
fn seek_head_bytes(at: u64) -> Vec<u8> {
    let mut entry = Vec::new();
    entry.extend(element(SEEK_ID, &TRACKS.to_be_bytes()));
    entry.extend(element(SEEK_POSITION, &at.to_be_bytes()));
    element(SEEK_HEAD, &element(SEEK, &entry))
}

/// An element, whole.
#[must_use]
pub fn element(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = header(id, length(payload.len()));
    bytes.extend_from_slice(payload);
    bytes
}

/// A length of something in memory, as the format writes lengths.
fn length(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

/// An element's header on its own, for the ones whose payload is not written.
#[must_use]
pub fn header(id: u32, size: u64) -> Vec<u8> {
    let mut bytes = id_bytes(id);
    bytes.extend(size_bytes(size));
    bytes
}

fn uint(id: u32, value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap_or(7);
    element(id, &bytes[first..])
}

fn text(id: u32, value: &str) -> Vec<u8> {
    element(id, value.as_bytes())
}

/// An identifier, written in as many bytes as it was defined with.
fn id_bytes(id: u32) -> Vec<u8> {
    let bytes = id.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap_or(3);
    bytes[first..].to_vec()
}

/// A length, in the narrowest form that will hold it.
fn size_bytes(size: u64) -> Vec<u8> {
    for width in 1..=8usize {
        let bits = width * 7;
        // The value of all ones means an unknown length, so a size that would
        // be written as that needs one more byte.
        let ceiling = (1u64 << bits) - 1;
        if size < ceiling {
            let marker = 1u64 << bits;
            let value = (marker | size).to_be_bytes();
            return value[8 - width..].to_vec();
        }
    }
    vec![0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{Container, Entry, element, size_bytes};
    use crate::{SubtitleCodec, subtitle_tracks};
    use std::io::Cursor;

    fn read(container: &Container) -> Vec<crate::StreamTrack> {
        subtitle_tracks(Cursor::new(container.bytes()))
    }

    #[test]
    fn a_length_is_written_in_the_narrowest_form_that_holds_it() {
        assert_eq!(size_bytes(0), vec![0x80]);
        assert_eq!(size_bytes(4), vec![0x84]);
        assert_eq!(size_bytes(126), vec![0xFE]);
        // One short of the width, which would otherwise say unknown.
        assert_eq!(size_bytes(127), vec![0x40, 0x7F]);
        assert_eq!(size_bytes(300), vec![0x41, 0x2C]);
    }

    #[test]
    fn an_element_is_its_identifier_its_length_and_its_payload() {
        assert_eq!(element(0xAE, &[1, 2, 3]), vec![0xAE, 0x83, 1, 2, 3]);
    }

    /// The builder and the reader are two halves of the same understanding, so
    /// this is the test that says the halves meet.
    #[test]
    fn what_is_written_reads_back() {
        let container = Container::new(vec![
            Entry::video(1),
            Entry::subtitle(2, "S_TEXT/UTF8").in_language("eng"),
        ]);

        let tracks = read(&container);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].number, 2);
        assert_eq!(tracks[0].codec, SubtitleCodec::SubRip);
        assert_eq!(tracks[0].language, Some("en"));
    }
}
