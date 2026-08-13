//! Finding out which subtitle tracks a Matroska file carries.
//!
//! Only the header is read. The Tracks element sits within the first few
//! kilobytes of almost every file, and where a file says otherwise its seek head
//! says where instead, so a four gigabyte film is answered for by a few hundred
//! bytes and two seeks rather than by reading any of the picture.
//!
//! Nothing here decodes, extracts or writes. It reads a header and reports what
//! it said.

use std::io::{Cursor, Read, Seek};
use std::path::Path;

use subtext_core::language_code;

use crate::codec::SubtitleCodec;
use crate::ebml::{Element, Reader};

/// The elements this reads, as the specification numbers them.
const EBML_HEADER: u32 = 0x1A45_DFA3;
const SEGMENT: u32 = 0x1853_8067;
const SEEK_HEAD: u32 = 0x114D_9B74;
const SEEK: u32 = 0x4DBB;
const SEEK_ID: u32 = 0x53AB;
const SEEK_POSITION: u32 = 0x53AC;
const CLUSTER: u32 = 0x1F43_B675;
const TRACKS: u32 = 0x1654_AE6B;
const TRACK_ENTRY: u32 = 0xAE;
const TRACK_NUMBER: u32 = 0xD7;
const TRACK_TYPE: u32 = 0x83;
const CODEC_ID: u32 = 0x86;
const LANGUAGE: u32 = 0x0022_B59C;
const LANGUAGE_BCP47: u32 = 0x0022_B59D;
const FLAG_DEFAULT: u32 = 0x88;
const FLAG_FORCED: u32 = 0x55AA;
const FLAG_HEARING_IMPAIRED: u32 = 0x55AB;

/// The track type subtitles are declared under.
const SUBTITLE: u64 = 0x11;

/// What a track says it is in when it says nothing.
///
/// The specification's default, and files that mean anything else say so. A
/// muxer with no idea writes "und", which reads as nothing rather than as
/// English.
const ASSUMED_LANGUAGE: &str = "eng";

/// How many children of one element are looked at before giving up.
///
/// A Segment holds a handful: the seek head, the running time, the tracks, the
/// chapters, the tags and then the picture. Anything claiming hundreds is not a
/// file this is going to make sense of.
const CHILDREN_LIMIT: usize = 32;

/// How many seek heads are followed. One points at the tracks, and a file may
/// keep a second one for what was added after it was first written.
const SEEK_HOPS: usize = 2;

/// The most subtitle tracks reported from one file.
///
/// A film with commentary, signs and thirty languages is around forty tracks in
/// total. This is above anything real and keeps a damaged header from turning
/// into a list nobody wants.
const TRACK_LIMIT: usize = 64;

/// A subtitle track carried inside a film.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamTrack {
    /// The number the container knows this track by, which is what its blocks
    /// are labelled with. Not a position in a list: numbers are chosen by
    /// whoever wrote the file and need not start at one or run without gaps.
    pub number: u64,
    pub codec: SubtitleCodec,
    /// The two letter code, in the vocabulary the rest of the application uses,
    /// or nothing where the file said nothing this build recognises.
    pub language: Option<&'static str>,
    /// The track the file suggests when nothing else decides.
    pub default: bool,
    pub forced: bool,
    pub hearing_impaired: bool,
}

/// The subtitle tracks declared inside a film file.
///
/// A file that is not Matroska, or is damaged, or carries no subtitles, all
/// come back the same way: with nothing. That is the honest answer to the only
/// question being asked, and it is one an unreadable file cannot be allowed to
/// turn into a failed scan.
///
/// # Errors
///
/// Only if the file cannot be opened at all.
pub fn probe(path: &Path) -> std::io::Result<Vec<StreamTrack>> {
    let file = std::fs::File::open(path)?;
    Ok(subtitle_tracks(file))
}

/// The same, from anything that can be read and seeked.
#[must_use]
pub fn subtitle_tracks<R: Read + Seek>(source: R) -> Vec<StreamTrack> {
    read(source).unwrap_or_default()
}

fn read<R: Read + Seek>(source: R) -> Option<Vec<StreamTrack>> {
    let mut reader = Reader::open(source)?;
    let segment = segment_of(&mut reader)?;
    let tracks = tracks_of(&mut reader, segment)?;
    let payload = reader.payload(tracks)?;
    Some(tracks_in(&payload))
}

/// The Segment, which is the element everything else lives inside.
fn segment_of<R: Read + Seek>(reader: &mut Reader<R>) -> Option<Element> {
    let length = reader.length();

    // Every Matroska file opens with an EBML header. Refusing anything that
    // does not is what keeps probing a folder of MP4 files down to one byte
    // each rather than a hunt through every one of them.
    let header = reader
        .element(length)
        .filter(|first| first.id == EBML_HEADER)?;

    let mut at = header.end;
    for _ in 0..CHILDREN_LIMIT {
        reader.seek_to(at)?;
        let element = reader.element(length)?;
        if element.id == SEGMENT {
            return Some(element);
        }
        at = element.end;
    }
    None
}

/// Where the Tracks element is, by the shortest route the file offers.
fn tracks_of<R: Read + Seek>(reader: &mut Reader<R>, segment: Element) -> Option<Element> {
    through_seek_head(reader, segment).or_else(|| by_walking(reader, segment))
}

/// The route a file with an index takes.
///
/// A seek head is written as the Segment's first child, so finding one costs a
/// single header read and finding none costs the same. What it says is treated
/// as a hint: an entry pointing at something that is not the tracks means the
/// file is walked instead, since a wrong index is not a reason to report a film
/// as having no subtitles.
fn through_seek_head<R: Read + Seek>(reader: &mut Reader<R>, segment: Element) -> Option<Element> {
    let mut at = segment.start;

    for _ in 0..SEEK_HOPS {
        reader.seek_to(at)?;
        let head = reader
            .element(segment.end)
            .filter(|it| it.id == SEEK_HEAD)?;
        let payload = reader.payload(head)?;
        let entries = seek_entries(&payload);

        if let Some(position) = position_of(&entries, TRACKS) {
            let tracks_at = segment.start.checked_add(position)?;
            reader.seek_to(tracks_at)?;
            return reader.element(segment.end).filter(|it| it.id == TRACKS);
        }

        // Seek heads chain: the first one is written before the file is, and
        // covers what was known then.
        at = segment
            .start
            .checked_add(position_of(&entries, SEEK_HEAD)?)?;
    }

    None
}

/// The route a file with no index takes.
fn by_walking<R: Read + Seek>(reader: &mut Reader<R>, segment: Element) -> Option<Element> {
    let mut at = segment.start;

    for _ in 0..CHILDREN_LIMIT {
        reader.seek_to(at)?;
        let child = reader.element(segment.end)?;
        match child.id {
            TRACKS => return Some(child),
            // Tracks are declared before the frames they describe, so a cluster
            // means there is nothing left to find. Reading on would mean a
            // header read for every cluster in the film to learn what is
            // already known.
            CLUSTER => return None,
            _ => at = child.end,
        }
    }

    None
}

/// What a seek head points at: an element identifier and where it begins,
/// counted from the start of the Segment's payload.
fn seek_entries(payload: &[u8]) -> Vec<(u32, u64)> {
    let mut entries = Vec::new();

    for (id, body) in children(payload, CHILDREN_LIMIT) {
        if id != SEEK {
            continue;
        }
        let mut wanted = None;
        let mut position = None;
        for (field, value) in children(&body, CHILDREN_LIMIT) {
            match field {
                SEEK_ID => wanted = as_uint(&value).and_then(|id| u32::try_from(id).ok()),
                SEEK_POSITION => position = as_uint(&value),
                _ => {}
            }
        }
        if let (Some(wanted), Some(position)) = (wanted, position) {
            entries.push((wanted, position));
        }
    }

    entries
}

fn position_of(entries: &[(u32, u64)], wanted: u32) -> Option<u64> {
    entries
        .iter()
        .find_map(|(id, position)| (*id == wanted).then_some(*position))
}

/// The subtitle tracks declared in a Tracks payload.
fn tracks_in(payload: &[u8]) -> Vec<StreamTrack> {
    let mut tracks = Vec::new();

    for (id, body) in children(payload, TRACK_LIMIT) {
        if id != TRACK_ENTRY {
            continue;
        }
        if let Some(track) = track_in(&body) {
            tracks.push(track);
        }
        if tracks.len() == TRACK_LIMIT {
            break;
        }
    }

    // In the order the file numbers them, so that a menu drawn from this reads
    // the same way twice regardless of how the header was laid out.
    tracks.sort_by_key(|track| track.number);
    tracks
}

/// One track entry, if it is a subtitle track and says enough to be one.
fn track_in(payload: &[u8]) -> Option<StreamTrack> {
    let mut number = None;
    let mut kind = None;
    let mut codec = None;
    let mut language = None;
    let mut tagged = None;
    // The specification's defaults, which are what a file that says nothing
    // means. Only the first of the three is anything other than off.
    let mut default = true;
    let mut forced = false;
    let mut hearing_impaired = false;

    for (id, value) in children(payload, CHILDREN_LIMIT) {
        match id {
            TRACK_NUMBER => number = as_uint(&value),
            TRACK_TYPE => kind = as_uint(&value),
            CODEC_ID => codec = as_text(&value),
            LANGUAGE => language = as_text(&value),
            LANGUAGE_BCP47 => tagged = as_text(&value),
            FLAG_DEFAULT => default = as_flag(&value),
            FLAG_FORCED => forced = as_flag(&value),
            FLAG_HEARING_IMPAIRED => hearing_impaired = as_flag(&value),
            _ => {}
        }
    }

    if kind? != SUBTITLE {
        return None;
    }

    // A track written after the newer element exists carries both, and the
    // newer one wins where they disagree.
    let declared = tagged.or(language);

    Some(StreamTrack {
        number: number?,
        codec: SubtitleCodec::of(&codec?),
        language: language_of(declared.as_deref().unwrap_or(ASSUMED_LANGUAGE)),
        default,
        forced,
        hearing_impaired,
    })
}

/// The code a track's language element comes to.
///
/// A tag can carry a region, and "pt-BR" is Portuguese as far as choosing a
/// subtitle goes. Anything the pairing does not recognise reads as nothing,
/// which is the same answer a file name with no language suffix gives.
fn language_of(declared: &str) -> Option<&'static str> {
    let base = declared.split(['-', '_']).next()?;
    language_code(base)
}

/// The children of a payload already in memory, as identifiers and bytes.
///
/// Elements inside a header are small and few, and reading them from a slice
/// rather than from the file means one read for the whole thing instead of one
/// for each. The framing is read by the same code either way, since a slice is
/// something that can be read and seeked like anything else.
fn children(payload: &[u8], limit: usize) -> Vec<(u32, Vec<u8>)> {
    let Some(mut reader) = Reader::open(Cursor::new(payload)) else {
        return Vec::new();
    };
    let end = reader.length();

    let mut found = Vec::new();
    let mut at = 0;
    while found.len() < limit {
        if reader.seek_to(at).is_none() {
            break;
        }
        let Some(element) = reader.element(end) else {
            break;
        };
        at = element.end;
        // An element too large to hold is stepped over rather than ending the
        // walk, since the one after it may be the one being looked for.
        if let Some(body) = reader.payload(element) {
            found.push((element.id, body));
        }
    }

    found
}

/// An unsigned integer as EBML writes one: big endian, and as narrow as it can
/// be written or as wide as eight bytes, since leading zeros are allowed.
fn as_uint(bytes: &[u8]) -> Option<u64> {
    (bytes.len() <= 8).then(|| {
        bytes
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
    })
}

/// A flag, which is an integer that means yes when it is not zero.
fn as_flag(bytes: &[u8]) -> bool {
    as_uint(bytes).is_some_and(|value| value != 0)
}

/// A string as EBML writes one, which may be padded with zero bytes.
fn as_text(bytes: &[u8]) -> Option<String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    std::str::from_utf8(bytes.get(..end)?)
        .ok()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{as_flag, as_text, as_uint, language_of};

    #[test]
    fn reads_an_integer_however_wide_it_was_written() {
        assert_eq!(as_uint(&[0x11]), Some(17));
        assert_eq!(as_uint(&[0x00, 0x00, 0x11]), Some(17));
        assert_eq!(as_uint(&[]), Some(0));
        assert_eq!(as_uint(&[0; 9]), None);
    }

    #[test]
    fn a_flag_is_anything_that_is_not_zero() {
        assert!(as_flag(&[1]));
        assert!(!as_flag(&[0]));
        // Nothing at all, which is how a file can write a flag it does not set.
        assert!(!as_flag(&[]));
    }

    #[test]
    fn a_string_stops_at_its_padding() {
        assert_eq!(as_text(b"S_TEXT/UTF8"), Some("S_TEXT/UTF8".to_owned()));
        assert_eq!(as_text(b"eng\0\0"), Some("eng".to_owned()));
        assert_eq!(as_text(&[0xFF, 0xFE]), None);
    }

    #[test]
    fn a_language_becomes_the_code_the_rest_of_the_application_uses() {
        assert_eq!(language_of("eng"), Some("en"));
        assert_eq!(language_of("fre"), Some("fr"));
        assert_eq!(language_of("fra"), Some("fr"));
        // A tag with a region, which says the same thing about which subtitle
        // somebody wants.
        assert_eq!(language_of("pt-BR"), Some("pt"));
        assert_eq!(language_of("en_GB"), Some("en"));

        // What a muxer writes when it does not know, which is not a language.
        assert_eq!(language_of("und"), None);
        assert_eq!(language_of(""), None);
    }
}
