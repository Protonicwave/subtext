//! Reading the dialogue out of the subtitle tracks inside a film.
//!
//! A probe reads a header and says what a film carries. This reads the frames
//! themselves, which is the only part of the crate that touches the body of a
//! file, and it is why the reading goes through a window rather than straight
//! at the file: a two hour film is around half a million blocks, and almost all
//! of them are picture and sound this steps over having read four bytes of.
//!
//! What comes back are cues in the same shape the SRT parser produces, because
//! everything downstream of here has no business knowing which of the two a
//! film's dialogue arrived by.
//!
//! Timings are the whole point of doing this at all. A track muxed into a film
//! was timed against those exact frames, so what is reconstructed here needs no
//! correcting: the cluster says what time it starts at, the block says how far
//! into the cluster it is, and the timestamp scale in the header says what unit
//! both of those are counted in.

use std::io::{Read, Seek};
use std::path::Path;

use subtext_core::parse::tags;
use subtext_core::{Cue, Timestamp};

use crate::buffer::Buffered;
use crate::codec::SubtitleCodec;
use crate::ebml::{Element, Reader, as_uint, children, vint};
use crate::ids;
use crate::probe::{self, StreamTrack};
use crate::segment::{self, CHILDREN_LIMIT};

/// What one unit of a timestamp is worth in nanoseconds when the file does not
/// say, which is a millisecond and is what almost every file means.
const DEFAULT_SCALE: u64 = 1_000_000;

/// The largest block read as a line of dialogue.
///
/// A subtitle block is a sentence. Anything approaching this is a frame of
/// something that has been mislabelled or a length that cannot be believed, and
/// either way there is no dialogue in it.
const MAX_BLOCK: u64 = 64 << 10;

/// How much of a block is read to learn which track it belongs to.
///
/// A track number of eight bytes, the two byte offset, and the flags. Real
/// files spend one byte on the track number, so this is already generous.
const BLOCK_HEADER: usize = 11;

/// The most cues taken from one track.
///
/// A film holds around two thousand and a whole series in one file holds tens
/// of thousands. This is far above either and stops a damaged file from being
/// read into memory a bad cue at a time.
const CUE_LIMIT: usize = 200_000;

/// A subtitle track inside a film, and the dialogue in it.
#[derive(Clone, Debug)]
pub struct EmbeddedTrack {
    pub track: StreamTrack,
    /// The lines, in playback order and numbered from one. Empty for a track of
    /// pictures, which is reported so that it can be named rather than read.
    pub cues: Vec<Cue>,
}

/// Every subtitle track inside a film, with the dialogue of the ones that hold
/// any.
///
/// One pass over the file for all of them together. A film with three languages
/// in it is read once, not three times, which matters because the reading is
/// the expensive part and the tracks are interleaved through the same blocks.
///
/// # Errors
///
/// Only if the file cannot be opened at all. A file that is not Matroska, or is
/// damaged, or carries no subtitles, comes back with nothing in the same way a
/// probe does.
pub fn extract(path: &Path) -> std::io::Result<Vec<EmbeddedTrack>> {
    let file = std::fs::File::open(path)?;
    Ok(subtitles_of(file))
}

/// The same, from anything that can be read and seeked.
#[must_use]
pub fn subtitles_of<R: Read + Seek>(source: R) -> Vec<EmbeddedTrack> {
    read(Buffered::new(source)).unwrap_or_default()
}

fn read<R: Read + Seek>(source: R) -> Option<Vec<EmbeddedTrack>> {
    let mut reader = Reader::open(source)?;
    let segment = segment::of(&mut reader)?;
    let tracks = probe::tracks_of(&mut reader, segment);

    let mut wanted: Vec<Reading> = tracks
        .iter()
        .filter(|track| track.codec.is_text())
        .map(Reading::of)
        .collect();

    if !wanted.is_empty() {
        let scale = scale_of(&mut reader, segment);
        walk(&mut reader, segment, scale, &mut wanted);
    }

    Some(
        tracks
            .into_iter()
            .map(|track| {
                let cues = wanted
                    .iter_mut()
                    .find(|reading| reading.number == track.number)
                    .map(Reading::take)
                    .unwrap_or_default();
                EmbeddedTrack { track, cues }
            })
            .collect(),
    )
}

/// One track being read, and what has been read of it so far.
#[derive(Debug)]
struct Reading {
    number: u64,
    codec: SubtitleCodec,
    cues: Vec<Cue>,
}

impl Reading {
    fn of(track: &StreamTrack) -> Self {
        Self {
            number: track.number,
            codec: track.codec,
            cues: Vec::new(),
        }
    }

    /// Records one block as a line, where there is a line in it.
    ///
    /// A payload that is not valid UTF-8, or that has nothing but markup in it,
    /// is a bad cue and is skipped in the way the SRT parser skips one. Text
    /// inside a container is UTF-8 by specification, so a payload that is not
    /// says something is wrong with that block rather than with the encoding of
    /// the track.
    fn add(&mut self, start: Timestamp, end: Timestamp, payload: &[u8]) {
        if self.cues.len() >= CUE_LIMIT {
            return;
        }
        let Ok(text) = std::str::from_utf8(payload) else {
            return;
        };

        let body = match self.codec {
            SubtitleCodec::SubStationAlpha => dialogue_field(text),
            _ => text,
        };
        let lines: Vec<&str> = body.split('\n').collect();
        let cleaned = tags::clean(&lines);
        if cleaned.text.is_empty() {
            return;
        }

        self.cues.push(Cue {
            // Numbered once the whole track has been read, since a cue's number
            // is its place in playback order and nothing here promises to meet
            // the blocks in that order.
            index: 0,
            start,
            end,
            text: cleaned.text,
            position: cleaned.position,
        });
    }

    /// The cues, in playback order and numbered.
    fn take(&mut self) -> Vec<Cue> {
        let mut cues = std::mem::take(&mut self.cues);
        cues.sort_by_key(|cue| (cue.start, cue.end));
        for (position, cue) in cues.iter_mut().enumerate() {
            cue.index = u32::try_from(position + 1).unwrap_or(u32::MAX);
        }
        cues
    }
}

/// The last field of a substation alpha record, which is the line itself.
///
/// A block carries what a file's Dialogue line carries without the two timings,
/// which the block itself supplies: a read order, a layer, a style, a speaker,
/// three margins and an effect, and then the text. Commas inside the text are
/// not a problem as long as the split stops counting at the last field, which
/// is why the number of fields is named rather than the text being taken from
/// the far end.
fn dialogue_field(record: &str) -> &str {
    const FIELDS: usize = 9;
    record.splitn(FIELDS, ',').last().unwrap_or(record)
}

/// How many nanoseconds one unit of the file's timestamps is.
fn scale_of<R: Read + Seek>(reader: &mut Reader<R>, segment: Element) -> u64 {
    let declared = segment::locate(reader, segment, ids::INFO)
        .and_then(|info| reader.payload(info))
        .and_then(|payload| {
            children(&payload, CHILDREN_LIMIT)
                .into_iter()
                .find_map(|(id, value)| (id == ids::TIMESTAMP_SCALE).then(|| as_uint(&value)))
        })
        .flatten();

    // A scale of nothing would put every line of the film at its first frame.
    declared.filter(|scale| *scale > 0).unwrap_or(DEFAULT_SCALE)
}

/// Every cluster in the film, in the order they were written.
///
/// The Cues element at the end of a file indexes some of the blocks by time,
/// and skipping to the ones it names would be faster. It is not used, because
/// nothing in the format says an index covers every block of a track, and an
/// index that covers only some of them would silently drop dialogue rather than
/// fail. Reading every cluster is what makes the answer the whole track.
fn walk<R: Read + Seek>(
    reader: &mut Reader<R>,
    segment: Element,
    scale: u64,
    wanted: &mut [Reading],
) {
    let mut at = segment.start;

    // Each element read moves this on by at least the two bytes of its own
    // header, so the walk ends at the end of the segment whatever the file
    // claims about the sizes inside it.
    while at < segment.end {
        if reader.seek_to(at).is_none() {
            return;
        }
        let Some(child) = reader.element(segment.end) else {
            return;
        };
        at = if child.id == ids::CLUSTER {
            cluster(reader, child, scale, wanted)
        } else {
            child.end
        };
    }
}

/// One cluster: a timestamp everything in it is counted from, and the blocks.
///
/// Reports where the cluster turned out to end, which is not always where its
/// header said. A cluster written with no length runs to whatever comes next,
/// and what comes next is the following cluster, so the file's own claim would
/// swallow the rest of the film and the dialogue in it.
fn cluster<R: Read + Seek>(
    reader: &mut Reader<R>,
    cluster: Element,
    scale: u64,
    wanted: &mut [Reading],
) -> u64 {
    // The specification puts the timestamp first, before any block that is
    // counted from it. A cluster that does not say has one of nothing, which is
    // what a file with a single cluster means anyway.
    let mut base = 0;
    let mut at = cluster.start;

    while at < cluster.end {
        let began = at;
        if reader.seek_to(at).is_none() {
            return cluster.end;
        }
        let Some(child) = reader.element(cluster.end) else {
            return cluster.end;
        };
        at = child.end;

        match child.id {
            ids::TIMESTAMP => base = uint_of(reader, child).unwrap_or(base),
            ids::SIMPLE_BLOCK => block(reader, child, base, None, scale, wanted),
            ids::BLOCK_GROUP => group(reader, child, base, scale, wanted),
            // The next cluster, which this one only appears to contain. It
            // begins where its header does, which is where this stopped.
            ids::CLUSTER => return began,
            _ => {}
        }
    }

    cluster.end
}

/// A block and the length the file gives it.
///
/// This is what a subtitle is written as, since a simple block has nowhere to
/// say how long a line stays on screen and a line without an end is not one.
fn group<R: Read + Seek>(
    reader: &mut Reader<R>,
    group: Element,
    base: u64,
    scale: u64,
    wanted: &mut [Reading],
) {
    let mut found = None;
    let mut duration = None;
    let mut at = group.start;

    while at < group.end {
        if reader.seek_to(at).is_none() {
            return;
        }
        let Some(child) = reader.element(group.end) else {
            return;
        };
        at = child.end;

        match child.id {
            ids::BLOCK => found = Some(child),
            // Read whether or not this group turns out to be wanted, since the
            // length can be written either side of the block it belongs to.
            ids::BLOCK_DURATION => duration = uint_of(reader, child),
            _ => {}
        }
    }

    if let Some(found) = found {
        block(reader, found, base, duration, scale, wanted);
    }
}

/// One block, if it belongs to a track being read.
///
/// The first few bytes say which track it is, and everything after that is only
/// read for the blocks that turn out to be wanted. That is the whole economy of
/// the walk: a picture frame costs the four bytes it takes to recognise it as
/// one, and the seek to step over it.
fn block<R: Read + Seek>(
    reader: &mut Reader<R>,
    block: Element,
    base: u64,
    duration: Option<u64>,
    scale: u64,
    wanted: &mut [Reading],
) {
    let Some(head) = reader.prefix(block, BLOCK_HEADER) else {
        return;
    };
    let Some(head) = header_of(&head) else {
        return;
    };
    let Some(reading) = wanted.iter_mut().find(|it| it.number == head.track) else {
        return;
    };
    if block.size() > MAX_BLOCK {
        return;
    }

    let Some(payload) = reader.payload(block) else {
        return;
    };
    let Some(text) = payload.get(head.length..) else {
        return;
    };

    let start = base.saturating_add_signed(i64::from(head.offset));
    // A block with no length given ends where it starts. Nothing here invents
    // one: how long a line ought to stay on screen to be read is a question the
    // reading comfort settings answer, for every line in the application and
    // not only for these.
    let end = start.saturating_add(duration.unwrap_or_default());
    reading.add(millis(start, scale), millis(end, scale), text);
}

/// What a block says about itself before it says anything else.
struct BlockHeader {
    track: u64,
    /// How far the block is from its cluster's timestamp, which may be behind
    /// it as well as in front.
    offset: i16,
    /// How many bytes of the payload are this header rather than dialogue.
    length: usize,
}

fn header_of(bytes: &[u8]) -> Option<BlockHeader> {
    let (track, width) = vint(bytes)?;
    let offset = i16::from_be_bytes([*bytes.get(width)?, *bytes.get(width + 1)?]);
    let flags = *bytes.get(width + 2)?;

    // Lacing packs several frames into one block, with a count and their
    // lengths in front of them. Nothing muxes subtitles that way, since each
    // line has its own timing, so a laced block is skipped rather than read as
    // though the packing were dialogue.
    if flags & 0b0000_0110 != 0 {
        return None;
    }

    Some(BlockHeader {
        track,
        offset,
        length: width + 3,
    })
}

/// A count of timestamp units as a position in the film.
fn millis(units: u64, scale: u64) -> Timestamp {
    let nanos = u128::from(units).saturating_mul(u128::from(scale));
    u32::try_from(nanos / 1_000_000).map_or(Timestamp::MAX, Timestamp::from_millis)
}

/// An integer element, for the small ones a cluster carries.
fn uint_of<R: Read + Seek>(reader: &mut Reader<R>, element: Element) -> Option<u64> {
    if element.size() > 8 {
        return None;
    }
    as_uint(&reader.payload(element)?)
}

#[cfg(test)]
mod tests {
    // A header these bytes do not parse to is the failure the test is looking
    // for, and stopping there says so more plainly than an assertion would.
    #![allow(clippy::expect_used)]

    use super::{DEFAULT_SCALE, dialogue_field, header_of, millis};
    use subtext_core::Timestamp;

    #[test]
    fn the_last_field_of_a_record_is_the_line() {
        assert_eq!(
            dialogue_field("0,0,Default,,0,0,0,,Hello there."),
            "Hello there."
        );
        // Commas in the dialogue, which is why the fields are counted rather
        // than the text being taken from the last comma.
        assert_eq!(
            dialogue_field("1,0,Default,Vincent,0,0,0,,Well, yes, but no."),
            "Well, yes, but no."
        );
        // A record with fewer fields than it should have, whose last field is
        // still the likeliest place for the words to be.
        assert_eq!(dialogue_field("Hello there."), "Hello there.");
    }

    #[test]
    fn a_block_says_its_track_and_where_it_is() {
        // Track one, a hundred units past its cluster, no flags set.
        let head = header_of(&[0x81, 0x00, 0x64, 0x00]).expect("a header");
        assert_eq!(head.track, 1);
        assert_eq!(head.offset, 100);
        assert_eq!(head.length, 4);

        // A track numbered too high for one byte, and an offset behind the
        // cluster, which is what a block written out of order carries.
        let wide = header_of(&[0x40, 0x02, 0xFF, 0x9C, 0x00]).expect("a header");
        assert_eq!(wide.track, 2);
        assert_eq!(wide.offset, -100);
        assert_eq!(wide.length, 5);
    }

    #[test]
    fn a_laced_block_is_not_read_as_dialogue() {
        assert!(header_of(&[0x81, 0x00, 0x64, 0b0000_0010]).is_none());
        assert!(header_of(&[0x81, 0x00, 0x64, 0b0000_0110]).is_none());
        // The bit next to them says the frame is not to be shown, which has
        // nothing to do with whether it can be read.
        assert!(header_of(&[0x81, 0x00, 0x64, 0b0000_1000]).is_some());
    }

    #[test]
    fn a_block_that_stops_before_its_header_does_is_not_one() {
        assert!(header_of(&[]).is_none());
        assert!(header_of(&[0x81]).is_none());
        assert!(header_of(&[0x81, 0x00, 0x64]).is_none());
        // A first byte that says the number runs on further than the block does.
        assert!(header_of(&[0x00, 0x00, 0x64, 0x00]).is_none());
    }

    #[test]
    fn a_timestamp_is_counted_in_whatever_unit_the_file_declared() {
        assert_eq!(millis(1_500, DEFAULT_SCALE), Timestamp::from_millis(1_500));
        // A file counting in tenths of a millisecond, which is legal and rare.
        assert_eq!(millis(15_000, 100_000), Timestamp::from_millis(1_500));
        // And one counting in whole seconds.
        assert_eq!(
            millis(90, 1_000_000_000),
            Timestamp::from_millis(90 * 1_000)
        );
    }

    #[test]
    fn a_position_beyond_anything_a_film_runs_to_is_held_at_the_end() {
        assert_eq!(millis(u64::MAX, DEFAULT_SCALE), Timestamp::MAX);
    }
}
