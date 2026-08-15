//! Reading the artwork a film carries, and the awkward files that carry none.
//!
//! The corpus covers what is actually attached to films: a cover under the name
//! the specification gives it, several images at once, the fonts a signs track
//! brings with it, files with nothing attached, and files that stop half way
//! through because the download did. None of them may panic, and none of them
//! may read more of a film than the attachment it was asked for.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::{Cursor, Read, Seek, SeekFrom};

use subtext_container::fixture::{Attachment, Container, Entry};
use subtext_container::{cover, cover_image, cover_image_in, cover_in};
use tempfile::TempDir;

/// The bytes a fixture stands its artwork up with.
const IMAGE: &[u8] = b"\xFF\xD8\xFF\xE0 a picture";
const OTHER: &[u8] = b"\xFF\xD8\xFF\xE0 another picture";

fn film(attachments: Vec<Attachment>) -> Container {
    Container::new(vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
    ])
    .with_attachments(attachments)
}

fn name_of(container: &Container) -> Option<String> {
    cover_in(Cursor::new(container.bytes())).map(|cover| cover.name)
}

fn image_of(container: &Container) -> Option<Vec<u8>> {
    cover_image_in(Cursor::new(container.bytes()))
}

#[test]
fn reads_the_cover_a_film_carries() {
    let container = film(vec![
        Attachment::new("cover.jpg", IMAGE).of_type("image/jpeg"),
    ]);

    assert_eq!(name_of(&container).as_deref(), Some("cover.jpg"));
    assert_eq!(image_of(&container).as_deref(), Some(IMAGE));

    // The same file with an index to find its parts by, which is the shape most
    // films are in.
    assert_eq!(
        image_of(&container.clone().with_seek_head()).as_deref(),
        Some(IMAGE)
    );
}

#[test]
fn a_film_with_nothing_attached_has_no_cover() {
    assert_eq!(name_of(&film(Vec::new())), None);
    assert_eq!(image_of(&film(Vec::new())), None);
}

#[test]
fn the_conventional_name_wins_over_anything_else_attached() {
    let container = film(vec![
        Attachment::new("backdrop.png", OTHER).of_type("image/png"),
        Attachment::new("cover.jpg", IMAGE).of_type("image/jpeg"),
        Attachment::new("small_cover.png", OTHER).of_type("image/png"),
    ]);

    assert_eq!(name_of(&container).as_deref(), Some("cover.jpg"));
    assert_eq!(image_of(&container).as_deref(), Some(IMAGE));
}

#[test]
fn an_image_under_a_name_of_somebodys_own_is_still_a_cover() {
    let container = film(vec![
        Attachment::new("artwork.png", IMAGE).of_type("image/png"),
    ]);

    assert_eq!(name_of(&container).as_deref(), Some("artwork.png"));
    assert_eq!(image_of(&container).as_deref(), Some(IMAGE));
}

#[test]
fn what_is_not_a_picture_is_left_where_it_is() {
    // The fonts a film brings for its signs track, which is the bulk of what is
    // attached to anything.
    let fonts = film(vec![
        Attachment::new("Roboto.ttf", b"not a picture").of_type("application/x-truetype-font"),
        Attachment::new("chapters.xml", b"<chapters/>").of_type("application/xml"),
    ]);
    assert_eq!(name_of(&fonts), None);

    // The same file with a cover among them, which is the ordinary case.
    let with_cover = film(vec![
        Attachment::new("Roboto.ttf", b"not a picture").of_type("application/x-truetype-font"),
        Attachment::new("cover.png", IMAGE).of_type("image/png"),
    ]);
    assert_eq!(image_of(&with_cover).as_deref(), Some(IMAGE));
}

#[test]
fn an_attachment_that_says_nothing_about_itself_is_read_by_its_name() {
    let container = film(vec![Attachment::new("cover.jpg", IMAGE)]);
    assert_eq!(image_of(&container).as_deref(), Some(IMAGE));

    // A muxer that wrote neither a type nor an extension has said nothing that
    // makes this a picture, and guessing at bytes is not this crate's business.
    assert_eq!(name_of(&film(vec![Attachment::new("cover", IMAGE)])), None);
}

#[test]
fn a_file_that_stops_part_way_through_reads_as_what_was_there() {
    let whole = film(vec![
        Attachment::new("cover.jpg", IMAGE).of_type("image/jpeg"),
    ])
    .with_seek_head()
    .bytes();

    for at in 0..whole.len() {
        let truncated = whole[..at].to_vec();
        // Either the attachment was complete before the cut or it was not.
        // What matters is that neither answer is a panic and neither is an
        // image invented out of the bytes that happened to be there.
        let read = cover_image_in(Cursor::new(truncated));
        assert!(
            read.is_none() || read.as_deref() == Some(IMAGE),
            "{at} bytes read as something else"
        );
    }
}

#[test]
fn nothing_in_a_damaged_file_is_taken_as_a_length_to_trust() {
    let whole = film(vec![
        Attachment::new("cover.jpg", IMAGE).of_type("image/jpeg"),
    ])
    .with_seek_head()
    .bytes();

    for at in 0..whole.len() {
        for damage in [0x00, 0x01, 0xFF, 0x7F] {
            let mut damaged = whole.clone();
            damaged[at] = damage;
            let read = cover_image_in(Cursor::new(damaged));
            assert!(
                read.as_ref()
                    .is_none_or(|image| image.len() <= IMAGE.len() + 64),
                "byte {at} set to {damage:#x} read as {} bytes",
                read.unwrap_or_default().len()
            );
        }
    }
}

#[test]
fn something_that_is_not_a_container_at_all_reads_as_nothing() {
    let mp4 = b"\x00\x00\x00\x20ftypisom".to_vec();
    assert!(cover_image_in(Cursor::new(mp4)).is_none());
    assert!(cover_image_in(Cursor::new(b"not a film at all".to_vec())).is_none());
    assert!(cover_image_in(Cursor::new(Vec::new())).is_none());
}

#[test]
fn a_film_on_disk_is_read_the_same_way() {
    let folder = TempDir::new().unwrap();
    let path = folder.path().join("Heat.1995.mkv");
    let container = film(vec![
        Attachment::new("cover.jpg", IMAGE).of_type("image/jpeg"),
    ])
    .with_seek_head();
    std::fs::write(&path, container.bytes()).unwrap();

    let found = cover(&path).expect("a film that is there to be readable");
    assert_eq!(found.map(|cover| cover.name).as_deref(), Some("cover.jpg"));
    assert_eq!(
        cover_image(&path)
            .expect("a film that is there to be readable")
            .as_deref(),
        Some(IMAGE)
    );

    // A file that is not there is the caller's problem rather than an empty
    // answer, since the two mean different things to a scan.
    assert!(cover(&folder.path().join("gone.mkv")).is_err());
}

/// A film three gigabytes long, without three gigabytes to write it in.
///
/// The header is real. Everything after it is zeros this hands out on demand,
/// which is what makes the bytes actually read countable.
#[derive(Debug)]
struct Film {
    header: Vec<u8>,
    length: u64,
    at: u64,
    read: u64,
}

impl Film {
    fn of(container: Container, picture: u64) -> Self {
        let header = container.with_declared_cluster(picture).bytes();
        Self {
            length: header.len() as u64 + picture,
            header,
            at: 0,
            read: 0,
        }
    }
}

impl Read for Film {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let wanted = buffer
            .len()
            .min(usize::try_from(self.length - self.at).unwrap_or(usize::MAX));
        for (offset, byte) in buffer.iter_mut().take(wanted).enumerate() {
            let at = usize::try_from(self.at).unwrap_or(usize::MAX) + offset;
            *byte = self.header.get(at).copied().unwrap_or(0);
        }
        self.at += wanted as u64;
        self.read += wanted as u64;
        Ok(wanted)
    }
}

impl Seek for Film {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.at = match to {
            SeekFrom::Start(at) => at,
            SeekFrom::End(back) => self.length.saturating_add_signed(back),
            SeekFrom::Current(by) => self.at.saturating_add_signed(by),
        };
        Ok(self.at)
    }
}

/// What makes taking a cover off a film worth doing at all: the picture is
/// never touched, so the size of the film does not come into it.
#[test]
fn a_cover_is_read_and_a_film_is_not() {
    const THREE_GIGABYTES: u64 = 3 << 30;

    let attached = film(vec![
        Attachment::new("Roboto.ttf", &[0; 2_048]).of_type("application/x-truetype-font"),
        Attachment::new("cover.jpg", IMAGE).of_type("image/jpeg"),
    ])
    .with_seek_head();

    // Asking whether there is one reads no image at all, not even the font it
    // had to step over.
    let mut asked = Film::of(attached.clone(), THREE_GIGABYTES);
    assert!(cover_in(&mut asked).is_some());
    assert!(asked.read < 2_048, "{} bytes read to ask", asked.read);

    let mut taken = Film::of(attached, THREE_GIGABYTES);
    assert_eq!(cover_image_in(&mut taken).as_deref(), Some(IMAGE));
    assert!(
        taken.read < 4_096,
        "{} bytes read of a three gigabyte film",
        taken.read
    );
}
