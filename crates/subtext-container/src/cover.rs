//! Reading the cover art a film carries.
//!
//! Matroska has a convention for this: an attached file called `cover`, in one
//! of the ordinary picture formats, is the artwork for the film it sits in.
//! Somebody chose that image, which is why it is the first place a cover is
//! looked for and a frame taken from the picture is the last.
//!
//! Two questions get asked, and they cost different things. [`cover`] says
//! whether a film carries one, which is a walk over the attachment headers and
//! reads none of them. [`cover_image`] reads the winning one, which is the tens
//! of kilobytes of the image itself and nothing else in the file.
//!
//! Attachments are also where a film keeps the fonts its subtitles are set in,
//! so most of what is in there is not a picture and is stepped over.

use std::io::{Read, Seek};
use std::path::Path;

use crate::ebml::{Element, Reader, as_text};
use crate::ids;
use crate::segment::{self, CHILDREN_LIMIT};

/// How many attached files are looked at in one film.
///
/// A film carrying a signs track brings a font for every style in it, which is
/// a handful. This is well above anything real and keeps a damaged header from
/// turning into a walk with no end.
const ATTACHMENT_LIMIT: usize = 64;

/// The picture formats a cover may be in.
///
/// What the webview will draw, which is the only consumer there is. A cover in
/// anything else is left where it is rather than half read.
const PICTURE_TYPES: &[&str] = &["image/jpeg", "image/jpg", "image/png", "image/webp"];
const PICTURE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// The names the specification gives the artwork, best first.
///
/// The plain one is the large portrait cover, which is the shape a tile is. The
/// landscape pair are for a wide frame and are worse at two by three than they
/// are at sixteen by nine, so they come after the portrait ones and before an
/// image that was attached under some name of its own.
const CONVENTIONAL: &[&str] = &["cover", "small_cover", "cover_land", "small_cover_land"];

/// A cover a film carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachedCover {
    /// What the file is called inside the container.
    pub name: String,
}

/// Whether a film carries its own cover, without reading it.
///
/// # Errors
///
/// Only if the file cannot be opened at all. A file that is not Matroska, or is
/// damaged, or carries no attachments, all come back with nothing.
pub fn cover(path: &Path) -> std::io::Result<Option<AttachedCover>> {
    let file = std::fs::File::open(path)?;
    Ok(cover_in(file))
}

/// The same, from anything that can be read and seeked.
#[must_use]
pub fn cover_in<R: Read + Seek>(source: R) -> Option<AttachedCover> {
    let (_, best) = best_in(source)?;
    Some(AttachedCover { name: best.name })
}

/// The cover a film carries, as the bytes of an image.
///
/// # Errors
///
/// Only if the file cannot be opened at all.
pub fn cover_image(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    let file = std::fs::File::open(path)?;
    Ok(cover_image_in(file))
}

/// The same, from anything that can be read and seeked.
#[must_use]
pub fn cover_image_in<R: Read + Seek>(source: R) -> Option<Vec<u8>> {
    let (mut reader, best) = best_in(source)?;
    reader.payload(best.data)
}

/// An attached file that might be the cover, and where its bytes are.
#[derive(Clone, Debug)]
struct Candidate {
    name: String,
    /// How well the name matches what the specification calls the artwork.
    rank: usize,
    data: Element,
}

/// The best picture attached to a film, and the reader left open over it.
fn best_in<R: Read + Seek>(source: R) -> Option<(Reader<R>, Candidate)> {
    let mut reader = Reader::open(source)?;
    let segment = segment::of(&mut reader)?;
    let attachments = segment::locate(&mut reader, segment, ids::ATTACHMENTS)?;

    let best = pictures_in(&mut reader, attachments)
        .into_iter()
        // The first of an equal pair wins, which is the order the file wrote
        // them in, so the same film gives the same cover on every machine.
        .min_by_key(|candidate| candidate.rank)?;

    Some((reader, best))
}

/// Every attached picture, as headers. None of the images are read here.
fn pictures_in<R: Read + Seek>(reader: &mut Reader<R>, attachments: Element) -> Vec<Candidate> {
    let mut found = Vec::new();
    let mut at = attachments.start;

    for _ in 0..ATTACHMENT_LIMIT {
        if at >= attachments.end || reader.seek_to(at).is_none() {
            break;
        }
        let Some(element) = reader.element(attachments.end) else {
            break;
        };
        at = element.end;

        if element.id != ids::ATTACHED_FILE {
            continue;
        }
        if let Some(candidate) = picture_in(reader, element) {
            found.push(candidate);
        }
    }

    found
}

/// One attached file, if it is a picture and says where its bytes are.
///
/// The name and the type are small elements and are read; the image itself is
/// stepped over and only its position is kept, so asking a film whether it has
/// a cover costs the same whether the cover is ten kilobytes or a megabyte.
fn picture_in<R: Read + Seek>(reader: &mut Reader<R>, attached: Element) -> Option<Candidate> {
    let mut name = None;
    let mut mime = None;
    let mut data = None;
    let mut at = attached.start;

    for _ in 0..CHILDREN_LIMIT {
        if at >= attached.end || reader.seek_to(at).is_none() {
            break;
        }
        let element = reader.element(attached.end)?;
        at = element.end;

        match element.id {
            ids::FILE_NAME => name = reader.payload(element).as_deref().and_then(as_text),
            ids::FILE_MIME_TYPE => mime = reader.payload(element).as_deref().and_then(as_text),
            ids::FILE_DATA => data = Some(element),
            _ => {}
        }
    }

    let name = name?;
    if !is_picture(&name, mime.as_deref()) {
        return None;
    }

    Some(Candidate {
        rank: rank_of(&name),
        name,
        data: data?,
    })
}

/// Whether an attachment is a picture at all.
///
/// Either the declared type or the name is enough. A muxer that writes
/// `application/octet-stream` beside a file called `cover.jpg` has still
/// attached a picture, and a font is neither.
fn is_picture(name: &str, mime: Option<&str>) -> bool {
    let declared = mime.is_some_and(|mime| {
        PICTURE_TYPES
            .iter()
            .any(|kind| kind.eq_ignore_ascii_case(mime.trim()))
    });

    declared
        || name.rsplit_once('.').is_some_and(|(_, extension)| {
            PICTURE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

/// How close an attachment's name is to what the specification calls the cover.
///
/// Anything else scores past the end of the list, so an image somebody attached
/// under a name of their own is still a cover when it is the only one.
fn rank_of(name: &str) -> usize {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);

    CONVENTIONAL
        .iter()
        .position(|known| known.eq_ignore_ascii_case(stem))
        .unwrap_or(CONVENTIONAL.len())
}

#[cfg(test)]
mod tests {
    use super::{is_picture, rank_of};

    #[test]
    fn the_conventional_names_come_first() {
        assert_eq!(rank_of("cover.jpg"), 0);
        assert_eq!(rank_of("COVER.PNG"), 0);
        assert_eq!(rank_of("small_cover.png"), 1);
        assert_eq!(rank_of("cover_land.jpg"), 2);

        // An image attached under a name of somebody's own, which is still the
        // cover when nothing better is in there.
        assert!(rank_of("artwork.jpg") > rank_of("small_cover_land.jpg"));
    }

    #[test]
    fn a_picture_is_one_by_its_type_or_by_its_name() {
        assert!(is_picture("cover.jpg", Some("image/jpeg")));
        assert!(is_picture("cover.jpg", Some("application/octet-stream")));
        assert!(is_picture("cover", Some("image/png")));
        assert!(is_picture("cover.WEBP", None));

        // The fonts a film carries for its signs track, which are the bulk of
        // what is attached to anything.
        assert!(!is_picture(
            "Roboto.ttf",
            Some("application/x-truetype-font")
        ));
        assert!(!is_picture("chapters.xml", None));
        assert!(!is_picture("cover", None));
    }
}
