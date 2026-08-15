//! Finding out which subtitle tracks a Matroska file carries.
//!
//! Only the header is read. The Tracks element sits within the first few
//! kilobytes of almost every file, and where a file says otherwise its seek head
//! says where instead, so a four gigabyte film is answered for by a few hundred
//! bytes and two seeks rather than by reading any of the picture.
//!
//! Nothing here decodes, extracts or writes. It reads a header and reports what
//! it said.

use std::io::{Read, Seek};
use std::path::Path;

use subtext_core::language_code;

use crate::codec::SubtitleCodec;
use crate::ebml::{Element, Reader, as_flag, as_text, as_uint, children};
use crate::ids;
use crate::segment::{self, CHILDREN_LIMIT};

/// What a track says it is in when it says nothing.
///
/// The specification's default, and files that mean anything else say so. A
/// muxer with no idea writes "und", which reads as nothing rather than as
/// English.
pub(crate) const ASSUMED_LANGUAGE: &str = "eng";

/// The most subtitle tracks reported from one file.
///
/// A film with commentary, signs and thirty languages is around forty tracks in
/// total. This is above anything real and keeps a damaged header from turning
/// into a list nobody wants.
pub(crate) const TRACK_LIMIT: usize = 64;

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
    let segment = segment::of(&mut reader)?;
    Some(tracks_of(&mut reader, segment))
}

/// The subtitle tracks a segment declares, for a reader already inside a file.
pub(crate) fn tracks_of<R: Read + Seek>(
    reader: &mut Reader<R>,
    segment: Element,
) -> Vec<StreamTrack> {
    let Some(tracks) = segment::locate(reader, segment, ids::TRACKS) else {
        return Vec::new();
    };
    let Some(payload) = reader.payload(tracks) else {
        return Vec::new();
    };
    tracks_in(&payload)
}

/// The subtitle tracks declared in a Tracks payload.
fn tracks_in(payload: &[u8]) -> Vec<StreamTrack> {
    let mut tracks = Vec::new();

    for (id, body) in children(payload, TRACK_LIMIT) {
        if id != ids::TRACK_ENTRY {
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
            ids::TRACK_NUMBER => number = as_uint(&value),
            ids::TRACK_TYPE => kind = as_uint(&value),
            ids::CODEC_ID => codec = as_text(&value),
            ids::LANGUAGE => language = as_text(&value),
            ids::LANGUAGE_BCP47 => tagged = as_text(&value),
            ids::FLAG_DEFAULT => default = as_flag(&value),
            ids::FLAG_FORCED => forced = as_flag(&value),
            ids::FLAG_HEARING_IMPAIRED => hearing_impaired = as_flag(&value),
            _ => {}
        }
    }

    if kind? != ids::SUBTITLE {
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
pub(crate) fn language_of(declared: &str) -> Option<&'static str> {
    let base = declared.split(['-', '_']).next()?;
    language_code(base)
}

#[cfg(test)]
mod tests {
    use super::language_of;

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
