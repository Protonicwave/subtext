//! Finding the parts of a Matroska file.
//!
//! Everything a reader wants is a child of the Segment: the tracks, what the
//! film says about itself, and the frames. This works out where each of them
//! begins, by the shortest route the file offers, and reads nothing but element
//! headers doing it.

use std::io::{Read, Seek};

use crate::ebml::{Element, Reader, as_uint, children};
use crate::ids;

/// How many children of one element are looked at before giving up.
///
/// A Segment holds a handful: the seek head, the running time, the tracks, the
/// chapters, the tags and then the picture. Anything claiming hundreds is not a
/// file this is going to make sense of.
pub(crate) const CHILDREN_LIMIT: usize = 32;

/// How many seek heads are followed. One points at the tracks, and a file may
/// keep a second one for what was added after it was first written.
const SEEK_HOPS: usize = 2;

/// The Segment, which is the element everything else lives inside.
pub(crate) fn of<R: Read + Seek>(reader: &mut Reader<R>) -> Option<Element> {
    let length = reader.length();

    // Every Matroska file opens with an EBML header. Refusing anything that
    // does not is what keeps probing a folder of MP4 files down to one byte
    // each rather than a hunt through every one of them.
    let header = reader
        .element(length)
        .filter(|first| first.id == ids::EBML_HEADER)?;

    let mut at = header.end;
    for _ in 0..CHILDREN_LIMIT {
        reader.seek_to(at)?;
        let element = reader.element(length)?;
        if element.id == ids::SEGMENT {
            return Some(element);
        }
        at = element.end;
    }
    None
}

/// Where one of the Segment's parts is, or nothing if the file has no such
/// part before its frames.
pub(crate) fn locate<R: Read + Seek>(
    reader: &mut Reader<R>,
    segment: Element,
    wanted: u32,
) -> Option<Element> {
    through_seek_head(reader, segment, wanted).or_else(|| by_walking(reader, segment, wanted))
}

/// The route a file with an index takes.
///
/// A seek head is written as the Segment's first child, so finding one costs a
/// single header read and finding none costs the same. What it says is treated
/// as a hint: an entry pointing at something that is not what was asked for
/// means the file is walked instead, since a wrong index is not a reason to
/// report a film as having no subtitles.
fn through_seek_head<R: Read + Seek>(
    reader: &mut Reader<R>,
    segment: Element,
    wanted: u32,
) -> Option<Element> {
    let mut at = segment.start;

    for _ in 0..SEEK_HOPS {
        reader.seek_to(at)?;
        let head = reader
            .element(segment.end)
            .filter(|it| it.id == ids::SEEK_HEAD)?;
        let payload = reader.payload(head)?;
        let entries = seek_entries(&payload);

        if let Some(position) = position_of(&entries, wanted) {
            let element_at = segment.start.checked_add(position)?;
            reader.seek_to(element_at)?;
            return reader.element(segment.end).filter(|it| it.id == wanted);
        }

        // Seek heads chain: the first one is written before the file is, and
        // covers what was known then.
        at = segment
            .start
            .checked_add(position_of(&entries, ids::SEEK_HEAD)?)?;
    }

    None
}

/// The route a file with no index takes.
fn by_walking<R: Read + Seek>(
    reader: &mut Reader<R>,
    segment: Element,
    wanted: u32,
) -> Option<Element> {
    let mut at = segment.start;

    for _ in 0..CHILDREN_LIMIT {
        reader.seek_to(at)?;
        let child = reader.element(segment.end)?;
        if child.id == wanted {
            return Some(child);
        }
        // Everything a file says about itself is written before the frames it
        // describes, so a cluster means there is nothing left to find. Reading
        // on would mean a header read for every cluster in the film to learn
        // what is already known.
        if child.id == ids::CLUSTER {
            return None;
        }
        at = child.end;
    }

    None
}

/// What a seek head points at: an element identifier and where it begins,
/// counted from the start of the Segment's payload.
fn seek_entries(payload: &[u8]) -> Vec<(u32, u64)> {
    let mut entries = Vec::new();

    for (id, body) in children(payload, CHILDREN_LIMIT) {
        if id != ids::SEEK {
            continue;
        }
        let mut wanted = None;
        let mut position = None;
        for (field, value) in children(&body, CHILDREN_LIMIT) {
            match field {
                ids::SEEK_ID => wanted = as_uint(&value).and_then(|id| u32::try_from(id).ok()),
                ids::SEEK_POSITION => position = as_uint(&value),
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
