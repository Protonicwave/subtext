//! Reading real files, and the awkward ones.
//!
//! The corpus covers what a folder of films actually holds: text tracks,
//! picture tracks, several tracks in one file, files with no subtitles at all,
//! files with no index to find them by, and files that stop half way through
//! because the download did. None of them may panic, and none of them may read
//! more of a film than its header.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::{Cursor, Read, Seek, SeekFrom};

use subtext_container::fixture::{Container, Entry};
use subtext_container::{StreamTrack, SubtitleCodec, probe, subtitle_tracks};
use tempfile::TempDir;

fn read(container: &Container) -> Vec<StreamTrack> {
    subtitle_tracks(Cursor::new(container.bytes()))
}

/// A film of the shape most are, with its subtitles inside it.
fn film() -> Container {
    Container::new(vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
        Entry::subtitle(4, "S_TEXT/ASS").in_language("jpn"),
    ])
}

#[test]
fn reads_the_text_tracks_of_a_film() {
    let tracks = read(&film().with_seek_head());

    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].number, 3);
    assert_eq!(tracks[0].codec, SubtitleCodec::SubRip);
    assert_eq!(tracks[0].language, Some("en"));
    assert_eq!(tracks[1].number, 4);
    assert_eq!(tracks[1].codec, SubtitleCodec::SubStationAlpha);
    assert_eq!(tracks[1].language, Some("ja"));
    assert!(tracks.iter().all(|track| track.codec.is_text()));
}

#[test]
fn a_file_with_no_index_is_walked_instead() {
    // The same file without the seek head, which has to come back the same way.
    assert_eq!(read(&film()), read(&film().with_seek_head()));
}

#[test]
fn the_index_is_what_finds_tracks_written_after_the_frames() {
    let awkward = film().with_tracks_after_the_cluster();

    // Walking stops at the first cluster, so without the index there is nothing
    // to find. This is the case the seek head exists for.
    assert!(read(&awkward).is_empty());
    assert_eq!(read(&awkward.clone().with_seek_head()).len(), 2);
}

#[test]
fn an_index_pointing_at_the_wrong_place_falls_back_to_walking() {
    let bytes = film().with_seek_head().bytes();
    let at = bytes
        .windows(2)
        .position(|pair| pair == [0x53, 0xAC])
        .expect("the seek position to be in there");

    // The last byte of the position, moved to somewhere there is no element.
    let mut damaged = bytes.clone();
    let last = at + 2 + 8;
    damaged[last] = damaged[last].wrapping_add(3);

    assert_eq!(subtitle_tracks(Cursor::new(damaged)).len(), 2);
}

#[test]
fn picture_tracks_are_reported_and_named() {
    let tracks = read(&Container::new(vec![
        Entry::video(1),
        Entry::subtitle(2, "S_HDMV/PGS").in_language("eng"),
        Entry::subtitle(3, "S_VOBSUB").in_language("fre"),
        Entry::subtitle(4, "S_TEXT/UTF8").in_language("eng"),
    ]));

    assert_eq!(tracks.len(), 3);
    assert_eq!(tracks[0].codec, SubtitleCodec::Pgs);
    assert_eq!(tracks[1].codec, SubtitleCodec::VobSub);
    assert!(tracks[0].codec.is_bitmap());
    assert!(tracks[1].codec.is_bitmap());
    assert!(tracks[2].codec.is_text());
}

#[test]
fn a_film_with_nothing_to_say_says_nothing() {
    let tracks = read(&Container::new(vec![Entry::video(1), Entry::audio(2)]));
    assert!(tracks.is_empty());

    // A file whose tracks element is there and empty, which is not the same
    // shape and comes to the same answer.
    assert!(read(&Container::new(Vec::new())).is_empty());
}

#[test]
fn the_flags_a_track_carries_are_read() {
    let tracks = read(&Container::new(vec![
        Entry::subtitle(1, "S_TEXT/UTF8").in_language("eng"),
        Entry::subtitle(2, "S_TEXT/UTF8")
            .in_language("eng")
            .not_default()
            .forced(),
        Entry::subtitle(3, "S_TEXT/UTF8")
            .in_language("eng")
            .not_default()
            .hearing_impaired(),
    ]));

    assert!(tracks[0].default && !tracks[0].forced && !tracks[0].hearing_impaired);
    assert!(!tracks[1].default && tracks[1].forced);
    assert!(tracks[2].hearing_impaired && !tracks[2].forced);
}

#[test]
fn a_track_that_says_nothing_about_its_language_is_taken_as_english() {
    // What the specification says a missing language element means, and files
    // that mean anything else say so.
    let tracks = read(&Container::new(vec![Entry::subtitle(1, "S_TEXT/UTF8")]));
    assert_eq!(tracks[0].language, Some("en"));

    // What a muxer writes when it has no idea, which is not a language.
    let unknown = read(&Container::new(vec![
        Entry::subtitle(1, "S_TEXT/UTF8").in_language("und"),
    ]));
    assert_eq!(unknown[0].language, None);
}

#[test]
fn tracks_come_back_in_the_order_the_file_numbers_them() {
    let tracks = read(&Container::new(vec![
        Entry::subtitle(9, "S_TEXT/UTF8").in_language("fre"),
        Entry::subtitle(2, "S_TEXT/UTF8").in_language("eng"),
    ]));

    assert_eq!(
        tracks.iter().map(|track| track.number).collect::<Vec<_>>(),
        [2, 9]
    );
}

#[test]
fn a_file_that_stops_part_way_through_reads_as_what_was_there() {
    let whole = film().with_seek_head().bytes();

    for at in 0..whole.len() {
        let truncated = whole[..at].to_vec();
        let tracks = subtitle_tracks(Cursor::new(truncated));
        // Either the header was complete before the cut or it was not. What
        // matters is that neither answer is a panic and neither is a track
        // invented out of the bytes that happened to be there.
        assert!(
            tracks.len() <= 2,
            "{at} bytes read as {} tracks",
            tracks.len()
        );
    }
}

#[test]
fn nothing_in_a_damaged_file_is_taken_as_a_length_to_trust() {
    let whole = film().with_seek_head().bytes();

    for at in 0..whole.len() {
        for damage in [0x00, 0x01, 0xFF, 0x7F] {
            let mut damaged = whole.clone();
            damaged[at] = damage;
            let tracks = subtitle_tracks(Cursor::new(damaged));
            assert!(tracks.len() <= 2, "byte {at} set to {damage:#x}");
        }
    }
}

#[test]
fn something_that_is_not_a_container_at_all_reads_as_nothing() {
    // An MP4, whose first four bytes are a length and then the word ftyp.
    let mp4 = b"\x00\x00\x00\x20ftypisom".to_vec();
    assert!(subtitle_tracks(Cursor::new(mp4)).is_empty());

    assert!(subtitle_tracks(Cursor::new(b"not a film at all".to_vec())).is_empty());
    assert!(subtitle_tracks(Cursor::new(Vec::new())).is_empty());
}

#[test]
fn a_film_on_disk_is_read_the_same_way() {
    let folder = TempDir::new().unwrap();
    let path = folder.path().join("Heat.1995.mkv");
    std::fs::write(&path, film().with_seek_head().bytes()).unwrap();

    let tracks = probe(&path).expect("a film that is there to be readable");
    assert_eq!(tracks.len(), 2);

    // A file that is not there is the caller's problem rather than an empty
    // answer, since the two mean different things to a scan.
    assert!(probe(&folder.path().join("gone.mkv")).is_err());
}

/// A film four gigabytes long, without four gigabytes to write it in.
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
    /// A container with `picture` bytes of frames declared after its header,
    /// and a file long enough to hold them.
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

/// The claim the whole crate rests on, measured rather than asserted by
/// inspection: a probe costs what a header costs and not what a film costs.
#[test]
fn a_probe_reads_a_header_and_not_a_film() {
    const FOUR_GIGABYTES: u64 = 4 << 30;

    let mut film = Film::of(film().with_seek_head(), FOUR_GIGABYTES);
    let tracks = subtitle_tracks(&mut film);

    assert_eq!(tracks.len(), 2);
    assert!(
        film.read < 4_096,
        "{} bytes read of a four gigabyte film",
        film.read
    );
}

/// The same, for the file that has no index to shorten the walk and a megabyte
/// of empty space in front of its tracks.
#[test]
fn a_probe_of_a_file_with_no_index_is_bounded_too() {
    const FOUR_GIGABYTES: u64 = 4 << 30;

    let mut film = Film::of(film().with_padding(1 << 20), FOUR_GIGABYTES);
    let tracks = subtitle_tracks(&mut film);

    assert_eq!(tracks.len(), 2);
    assert!(
        film.read < 4_096,
        "{} bytes read of a four gigabyte film",
        film.read
    );
}
