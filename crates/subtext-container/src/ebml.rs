//! Reading EBML, which is the framing Matroska is written in.
//!
//! An EBML file is elements all the way down, each one an identifier, a length,
//! and that many bytes. Nothing here understands what any of them mean; that is
//! [`crate::probe`]'s business. This part reads the framing and refuses to
//! believe impossible things about it.
//!
//! Every read is bounded, because the input is whatever the person happened to
//! download. A length that runs past the end of the file, a length wider than
//! this build can represent, a payload larger than any header legitimately is:
//! each of them stops the read and hands back what was found so far. There is
//! no recursion anywhere, so a file cannot nest its way into the stack either.

use std::io::{Cursor, Read, Seek, SeekFrom};

/// The largest payload this will read into memory rather than seek over.
///
/// One track header is a few hundred bytes and the whole Tracks element of a
/// film carrying a dozen of them is a few kilobytes, so this is far above
/// anything a real file holds and far below anything that would be felt.
pub(crate) const MAX_PAYLOAD: u64 = 4 << 20;

/// One element, as its header describes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) id: u32,
    /// Where the payload begins, which is just past the header.
    pub(crate) start: u64,
    /// Where the payload ends.
    pub(crate) end: u64,
}

impl Element {
    pub(crate) fn size(self) -> u64 {
        self.end - self.start
    }
}

/// A length, or the absence of one.
///
/// A file still being written says its Segment runs to wherever the writing
/// stops, which is what an unknown length means. It is legal, it is rare, and
/// reading it as a length of zero would end the read at the first element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Size {
    Known(u64),
    Unknown,
}

/// Somewhere to read elements from.
#[derive(Debug)]
pub(crate) struct Reader<R> {
    source: R,
    length: u64,
    /// Where the source is. Kept rather than asked for, since the answer is
    /// wanted after every element and asking is a system call on some
    /// platforms.
    at: u64,
}

impl<R: Read + Seek> Reader<R> {
    /// Opens a source, or gives up if its length cannot be established.
    pub(crate) fn open(mut source: R) -> Option<Self> {
        let length = source.seek(SeekFrom::End(0)).ok()?;
        source.seek(SeekFrom::Start(0)).ok()?;
        Some(Self {
            source,
            length,
            at: 0,
        })
    }

    pub(crate) fn length(&self) -> u64 {
        self.length
    }

    /// The header of the next element, which must lie inside `end`.
    ///
    /// Nothing is read past the header: the payload is either asked for or
    /// stepped over, which is what keeps a probe of a four gigabyte film to a
    /// few hundred bytes.
    pub(crate) fn element(&mut self, end: u64) -> Option<Element> {
        if self.at >= end {
            return None;
        }

        let id = self.read_id()?;
        let size = self.read_size()?;
        // Past the header, which is where the payload begins.
        let start = self.at;

        // An element that claims to be larger than whatever holds it is the
        // shape a truncated download takes, and following it would mean reading
        // to the end of the file for a header that is not there.
        let finish = match size {
            Size::Known(size) => start
                .checked_add(size)
                .filter(|finish| *finish <= end && *finish <= self.length)?,
            Size::Unknown => end.min(self.length),
        };

        Some(Element {
            id,
            start,
            end: finish,
        })
    }

    /// An element's payload, for the ones small enough to be worth holding.
    pub(crate) fn payload(&mut self, element: Element) -> Option<Vec<u8>> {
        let size = element.size();
        if size > MAX_PAYLOAD {
            return None;
        }
        self.seek_to(element.start)?;

        let mut bytes = vec![0; usize::try_from(size).ok()?];
        self.source.read_exact(&mut bytes).ok()?;
        self.at += size;
        Some(bytes)
    }

    pub(crate) fn seek_to(&mut self, to: u64) -> Option<()> {
        if to > self.length {
            return None;
        }
        if to != self.at {
            self.source.seek(SeekFrom::Start(to)).ok()?;
            self.at = to;
        }
        Some(())
    }

    fn read_byte(&mut self) -> Option<u8> {
        let mut byte = [0u8; 1];
        self.source.read_exact(&mut byte).ok()?;
        self.at += 1;
        Some(byte[0])
    }

    /// An element identifier, which keeps the marker bits it was written with.
    ///
    /// The marker is part of the identifier rather than a length prefix to be
    /// stripped, which is why the numbers in the specification are the numbers
    /// on the wire.
    fn read_id(&mut self) -> Option<u32> {
        let first = self.read_byte()?;
        let width = width_of(first, 4)?;

        let mut id = u32::from(first);
        for _ in 1..width {
            id = (id << 8) | u32::from(self.read_byte()?);
        }
        Some(id)
    }

    /// A length, whose marker bits are a prefix and are taken off.
    fn read_size(&mut self) -> Option<Size> {
        let first = self.read_byte()?;
        let width = width_of(first, 8)?;

        let mask = data_mask(width);
        let mut value = u64::from(first & mask);
        // All data bits set says the length is unknown, and that is only true
        // if every byte of it says so.
        let mut unknown = first & mask == mask;

        for _ in 1..width {
            let byte = self.read_byte()?;
            unknown &= byte == 0xFF;
            value = (value << 8) | u64::from(byte);
        }

        Some(if unknown {
            Size::Unknown
        } else {
            Size::Known(value)
        })
    }
}

/// How many bytes a variable length integer beginning with `first` occupies.
///
/// The leading zeros say how many bytes follow. A byte of nothing but zeros
/// says more follow than the format allows, which is either a file that is not
/// EBML at all or one that has been damaged, and either way there is nothing to
/// read here.
fn width_of(first: u8, widest: u32) -> Option<u32> {
    let extra = first.leading_zeros();
    (extra < widest).then_some(extra + 1)
}

/// The bits of the first byte that carry the value rather than the width.
///
/// Widened to sixteen bits for the shift, since a length written across all
/// eight bytes leaves nothing of the first one and shifting a byte by eight is
/// not a thing a byte can do.
fn data_mask(width: u32) -> u8 {
    u8::try_from(0x00FF_u16 >> width).unwrap_or_default()
}

/// The children of a payload already in memory, as identifiers and bytes.
///
/// Elements inside a header are small and few, and reading them from a slice
/// rather than from the file means one read for the whole thing instead of one
/// for each. The framing is read by the same code either way, since a slice is
/// something that can be read and seeked like anything else.
pub(crate) fn children(payload: &[u8], limit: usize) -> Vec<(u32, Vec<u8>)> {
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
pub(crate) fn as_uint(bytes: &[u8]) -> Option<u64> {
    (bytes.len() <= 8).then(|| {
        bytes
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
    })
}

/// A flag, which is an integer that means yes when it is not zero.
pub(crate) fn as_flag(bytes: &[u8]) -> bool {
    as_uint(bytes).is_some_and(|value| value != 0)
}

/// A string as EBML writes one, which may be padded with zero bytes.
pub(crate) fn as_text(bytes: &[u8]) -> Option<String> {
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
    #![allow(clippy::unwrap_used)]

    use std::io::Cursor;

    use super::{Element, Reader, Size, as_flag, as_text, as_uint, data_mask, width_of};

    fn reader(bytes: &[u8]) -> Reader<Cursor<Vec<u8>>> {
        Reader::open(Cursor::new(bytes.to_vec())).unwrap()
    }

    #[test]
    fn a_width_is_the_leading_zeros_of_the_first_byte() {
        assert_eq!(width_of(0x82, 4), Some(1));
        assert_eq!(width_of(0x42, 4), Some(2));
        assert_eq!(width_of(0x1A, 4), Some(4));
        // Five bytes, which is a length but never an identifier.
        assert_eq!(width_of(0x08, 4), None);
        assert_eq!(width_of(0x08, 8), Some(5));
        // Nothing but zeros says a width the format has no room for.
        assert_eq!(width_of(0x00, 8), None);
    }

    #[test]
    fn the_data_mask_leaves_the_first_byte_alone_at_full_width() {
        assert_eq!(data_mask(1), 0x7F);
        assert_eq!(data_mask(4), 0x0F);
        assert_eq!(data_mask(8), 0x00);
    }

    #[test]
    fn reads_an_identifier_of_every_width() {
        assert_eq!(reader(&[0xAE]).read_id(), Some(0xAE));
        assert_eq!(reader(&[0x55, 0xAA]).read_id(), Some(0x55AA));
        assert_eq!(reader(&[0x22, 0xB5, 0x9C]).read_id(), Some(0x0022_B59C));
        assert_eq!(
            reader(&[0x1A, 0x45, 0xDF, 0xA3]).read_id(),
            Some(0x1A45_DFA3)
        );
    }

    #[test]
    fn reads_a_length_of_every_width() {
        assert_eq!(reader(&[0x84]).read_size(), Some(Size::Known(4)));
        assert_eq!(reader(&[0x40, 0x04]).read_size(), Some(Size::Known(4)));
        assert_eq!(
            reader(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04]).read_size(),
            Some(Size::Known(4))
        );
    }

    #[test]
    fn a_length_of_all_ones_is_no_length_at_all() {
        assert_eq!(reader(&[0xFF]).read_size(), Some(Size::Unknown));
        assert_eq!(reader(&[0x7F, 0xFF]).read_size(), Some(Size::Unknown));
        // One bit short of every bit, which is a length and not a refusal.
        assert_eq!(reader(&[0x7F, 0xFE]).read_size(), Some(Size::Known(16_382)));
    }

    #[test]
    fn an_element_carries_where_its_payload_begins_and_ends() {
        let mut reader = reader(&[0xAE, 0x83, 1, 2, 3]);
        assert_eq!(
            reader.element(5),
            Some(Element {
                id: 0xAE,
                start: 2,
                end: 5
            })
        );
    }

    #[test]
    fn an_element_claiming_more_than_the_file_holds_is_refused() {
        let mut reader = reader(&[0xAE, 0x8F, 1, 2, 3]);
        assert_eq!(reader.element(5), None);
    }

    #[test]
    fn an_element_of_unknown_length_runs_to_the_end_of_what_holds_it() {
        let mut reader = reader(&[0xAE, 0xFF, 1, 2, 3]);
        assert_eq!(
            reader.element(5),
            Some(Element {
                id: 0xAE,
                start: 2,
                end: 5
            })
        );
    }

    #[test]
    fn a_file_that_stops_mid_header_reads_as_nothing() {
        assert_eq!(reader(&[0x1A, 0x45]).element(2), None);
        assert_eq!(reader(&[]).element(0), None);
        assert_eq!(reader(&[0xAE]).element(1), None);
    }

    #[test]
    fn a_payload_larger_than_the_ceiling_is_not_held() {
        let mut reader = reader(&[1, 2, 3]);
        let huge = Element {
            id: 0xAE,
            start: 0,
            end: super::MAX_PAYLOAD + 1,
        };
        assert_eq!(reader.payload(huge), None);
    }

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
}
