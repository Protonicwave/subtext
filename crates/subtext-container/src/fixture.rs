//! Building small Matroska files to read back.
//!
//! Public, and deliberately so. Testing a reader of containers means having
//! containers to read, and producing one otherwise means depending on a muxer,
//! which is a large tool to install on three platforms of build machine to
//! produce a file of two hundred bytes. Writing the framing by hand is a page
//! of code, it says exactly what each fixture contains, and the scanner's tests
//! need the same thing a folder at a time.
//!
//! A container with no dialogue in it is a header and nothing else, which is
//! all a probe has any use for. Adding dialogue writes real clusters holding
//! real blocks, since reading those back is the other half of what the crate
//! does, and filler can be written alongside them so that a fixture costs the
//! reader what a film costs it.

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
const TIMESTAMP: u32 = 0xE7;
const SIMPLE_BLOCK: u32 = 0xA3;
const BLOCK_GROUP: u32 = 0xA0;
const BLOCK: u32 = 0xA1;
const BLOCK_DURATION: u32 = 0x9B;
const ATTACHMENTS: u32 = 0x1941_A469;
const ATTACHED_FILE: u32 = 0x61A7;
const FILE_NAME: u32 = 0x466E;
const FILE_MIME_TYPE: u32 = 0x4660;
const FILE_DATA: u32 = 0x465C;
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

/// What one unit of a timestamp is worth in nanoseconds unless a fixture says
/// otherwise, which is a millisecond.
const DEFAULT_SCALE: u64 = 1_000_000;

/// How many lines share a cluster.
///
/// Real files cut a cluster every few seconds, so a line's timing is the
/// cluster's timestamp plus an offset rather than either on its own. Grouping
/// them here is what makes a fixture exercise that arithmetic.
const LINES_PER_CLUSTER: usize = 8;

/// The furthest a block can be written from its cluster's timestamp, which is
/// as far as a signed two byte offset reaches.
const MAX_OFFSET: u64 = 32_767;

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

/// One file carried alongside the picture: a cover, or a font for the signs.
#[derive(Clone, Debug)]
pub struct Attachment {
    name: String,
    mime: Option<String>,
    data: Vec<u8>,
}

impl Attachment {
    /// An attachment under the given name, carrying the given bytes.
    ///
    /// The bytes are written as they stand and are never read as a picture by
    /// anything here, so a fixture says what it is about with a few of them
    /// rather than with a real image.
    #[must_use]
    pub fn new(name: &str, data: &[u8]) -> Self {
        Self {
            name: name.to_owned(),
            mime: None,
            data: data.to_vec(),
        }
    }

    /// What the file says the attachment is, which a muxer may get wrong or
    /// leave out.
    #[must_use]
    pub fn of_type(mut self, mime: &str) -> Self {
        self.mime = Some(mime.to_owned());
        self
    }

    fn bytes(&self) -> Vec<u8> {
        let mut body = text(FILE_NAME, &self.name);
        if let Some(mime) = &self.mime {
            body.extend(text(FILE_MIME_TYPE, mime));
        }
        body.extend(element(FILE_DATA, &self.data));
        element(ATTACHED_FILE, &body)
    }
}

/// One line of dialogue in a fixture, and when it is on screen.
#[derive(Clone, Debug)]
pub struct Line {
    track: u64,
    start_ms: u64,
    end_ms: u64,
    payload: String,
}

impl Line {
    /// A line of the given track, on screen between the two times.
    ///
    /// The payload is written into the block as it stands, so a fixture of
    /// substation alpha dialogue passes the whole comma separated record and
    /// one of plain text passes the words. Which of the two a reader should
    /// expect is the track's codec identifier, not anything written here.
    #[must_use]
    pub fn new(track: u64, start_ms: u64, end_ms: u64, payload: &str) -> Self {
        Self {
            track,
            start_ms,
            end_ms,
            payload: payload.to_owned(),
        }
    }
}

/// A Matroska file.
///
/// The flags are the awkward shapes a file can take, each of them one a reader
/// has to cope with. They are independent of one another and a fixture picks
/// whichever it is about, which is what a builder is for.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug)]
pub struct Container {
    entries: Vec<Entry>,
    seek_head: bool,
    padding: usize,
    cluster: Option<u64>,
    tracks_after_cluster: bool,
    lines: Vec<Line>,
    durations: bool,
    unbounded_clusters: bool,
    scale: u64,
    /// A track number, how many filler blocks each cluster carries, and how
    /// large each of them is.
    picture: Option<(u64, usize, usize)>,
    attachments: Vec<Attachment>,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            seek_head: false,
            padding: 0,
            cluster: None,
            tracks_after_cluster: false,
            lines: Vec::new(),
            durations: true,
            unbounded_clusters: false,
            scale: DEFAULT_SCALE,
            picture: None,
            attachments: Vec::new(),
        }
    }
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

    /// The files the film carries alongside its picture.
    #[must_use]
    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    /// The dialogue the film carries, written as blocks in real clusters.
    #[must_use]
    pub fn with_dialogue(mut self, lines: Vec<Line>) -> Self {
        self.lines = lines;
        self
    }

    /// Writes the clusters with no length at all, which is legal, and is what a
    /// file being recorded as it is written carries: each one runs until the
    /// next begins.
    #[must_use]
    pub fn with_unbounded_clusters(mut self) -> Self {
        self.unbounded_clusters = true;
        self
    }

    /// Writes the lines as simple blocks, which have nowhere to say how long a
    /// line stays on screen.
    #[must_use]
    pub fn without_durations(mut self) -> Self {
        self.durations = false;
        self
    }

    /// What one unit of every timestamp in the file is worth, in nanoseconds.
    #[must_use]
    pub fn with_timestamp_scale(mut self, nanos: u64) -> Self {
        self.scale = nanos;
        self
    }

    /// Filler blocks of the given track in every cluster, standing in for the
    /// picture and sound a film carries between one line and the next.
    ///
    /// What this is for is cost rather than content: a reader of dialogue has
    /// to step over all of it, and a fixture with none of it would let a reader
    /// that read every block wholesale look as quick as one that did not.
    #[must_use]
    pub fn with_picture(mut self, track: u64, blocks: usize, bytes: usize) -> Self {
        self.picture = Some((track, blocks, bytes));
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
        body.extend(element(INFO, &uint(TIMESTAMP_SCALE, self.scale)));
        if self.tracks_after_cluster {
            body.extend(element(CLUSTER, &[]));
        }

        let tracks_at = head_length + body.len();
        body.extend(element(TRACKS, &self.tracks()));
        body.extend(self.attached());
        body.extend(self.clusters());
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

    /// The attachments element, or nothing where a film carries no files.
    fn attached(&self) -> Vec<u8> {
        if self.attachments.is_empty() {
            return Vec::new();
        }
        let body: Vec<u8> = self
            .attachments
            .iter()
            .flat_map(Attachment::bytes)
            .collect();
        element(ATTACHMENTS, &body)
    }

    /// Writes the film out one cluster at a time.
    ///
    /// The Segment's length is left unsaid, which is legal and is what a file
    /// still being written carries, so that a fixture can be produced without
    /// first being held in memory. That is what makes a film of several
    /// gigabytes something a benchmark can measure against.
    ///
    /// # Errors
    ///
    /// Whatever the writer reports.
    pub fn write_to(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        out.write_all(&element(EBML_HEADER, &text(DOC_TYPE, "matroska")))?;
        out.write_all(&id_bytes(SEGMENT))?;
        out.write_all(&[0xFF])?;
        out.write_all(&element(INFO, &uint(TIMESTAMP_SCALE, self.scale)))?;
        out.write_all(&element(TRACKS, &self.tracks()))?;
        out.write_all(&self.attached())?;

        let (lines, groups) = self.cut();
        for (base, from, to) in groups {
            out.write_all(&self.cluster(base, &lines[from..to]))?;
        }
        Ok(())
    }

    /// The dialogue, cut into clusters the way a muxer cuts it.
    fn clusters(&self) -> Vec<u8> {
        let (lines, groups) = self.cut();
        groups
            .into_iter()
            .flat_map(|(base, from, to)| self.cluster(base, &lines[from..to]))
            .collect()
    }

    /// Where the clusters fall: the lines in order, and for each cluster the
    /// timestamp it carries and the lines it holds.
    ///
    /// A new cluster is started once it holds enough lines, and also once a
    /// line is far enough past the cluster's timestamp that the two byte offset
    /// in a block could not reach it, which is the reason real files cut them
    /// at all.
    fn cut(&self) -> (Vec<Line>, Vec<(u64, usize, usize)>) {
        let mut lines = self.lines.clone();
        lines.sort_by_key(|line| line.start_ms);

        let mut groups = Vec::new();
        let mut at = 0;
        while at < lines.len() {
            let base = self.units(lines[at].start_ms);
            let mut end = at + 1;
            while end < lines.len()
                && end - at < LINES_PER_CLUSTER
                && self.units(lines[end].start_ms).saturating_sub(base) <= MAX_OFFSET
            {
                end += 1;
            }
            groups.push((base, at, end));
            at = end;
        }

        (lines, groups)
    }

    fn cluster(&self, base: u64, lines: &[Line]) -> Vec<u8> {
        let mut body = uint(TIMESTAMP, base);

        if let Some((track, blocks, bytes)) = self.picture {
            for _ in 0..blocks {
                body.extend(element(SIMPLE_BLOCK, &block(track, 0, &vec![0; bytes])));
            }
        }

        for line in lines {
            let offset =
                i16::try_from(self.units(line.start_ms).saturating_sub(base)).unwrap_or(i16::MAX);
            let written = block(line.track, offset, line.payload.as_bytes());

            if self.durations {
                let mut group = element(BLOCK, &written);
                let length = self
                    .units(line.end_ms)
                    .saturating_sub(self.units(line.start_ms));
                group.extend(uint(BLOCK_DURATION, length));
                body.extend(element(BLOCK_GROUP, &group));
            } else {
                body.extend(element(SIMPLE_BLOCK, &written));
            }
        }

        if !self.unbounded_clusters {
            return element(CLUSTER, &body);
        }
        // A length of all ones, which says nothing about how far the cluster
        // runs.
        let mut bytes = id_bytes(CLUSTER);
        bytes.push(0xFF);
        bytes.extend(body);
        bytes
    }

    /// A time in milliseconds, in the unit this file counts in.
    fn units(&self, millis: u64) -> u64 {
        millis
            .saturating_mul(1_000_000)
            .checked_div(self.scale)
            .unwrap_or_default()
    }
}

/// A block's payload: which track it belongs to, how far it is from its
/// cluster's timestamp, a byte of flags, and then the content.
fn block(track: u64, offset: i16, payload: &[u8]) -> Vec<u8> {
    // A track number is written the way a length is, so the same code writes
    // both. What differs is only what the number means.
    let mut bytes = size_bytes(track);
    bytes.extend(offset.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(payload);
    bytes
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
