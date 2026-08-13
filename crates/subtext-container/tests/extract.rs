//! Reading the dialogue out of real files, and the awkward ones.
//!
//! What a film carries is one question and what is written in it is another,
//! and this covers the second: plain text and substation alpha, several tracks
//! interleaved through the same clusters, files counting time in units of their
//! own, blocks with no length given, and files that stop or are damaged part way
//! through. None of them may panic, and none of them may invent a line.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::{Cursor, Read, Seek, SeekFrom};

use subtext_container::fixture::{Container, Entry, Line};
use subtext_container::{EmbeddedTrack, SubtitleCodec, extract, subtitles_of};
use subtext_core::{CuePosition, Timestamp};
use tempfile::TempDir;

fn read(container: &Container) -> Vec<EmbeddedTrack> {
    subtitles_of(Cursor::new(container.bytes()))
}

/// The track of the given number, which the test knows is in there.
fn track(tracks: &[EmbeddedTrack], number: u64) -> &EmbeddedTrack {
    tracks
        .iter()
        .find(|found| found.track.number == number)
        .expect("the track to have been found")
}

fn lines(tracks: &[EmbeddedTrack], number: u64) -> Vec<String> {
    track(tracks, number)
        .cues
        .iter()
        .map(|cue| cue.text.clone())
        .collect()
}

/// A film with one text track and three lines in it.
fn film() -> Container {
    Container::new(vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
    ])
    .with_seek_head()
    .with_dialogue(vec![
        Line::new(3, 1_000, 3_000, "Don't let yourself get attached"),
        Line::new(3, 4_000, 6_500, "to anything you are not willing"),
        Line::new(3, 90_000, 92_000, "to walk out on in thirty seconds flat."),
    ])
}

#[test]
fn reads_the_lines_of_a_text_track() {
    let tracks = read(&film());
    assert_eq!(tracks.len(), 1);

    let found = track(&tracks, 3);
    assert_eq!(found.track.codec, SubtitleCodec::SubRip);
    assert_eq!(found.cues.len(), 3);

    assert_eq!(found.cues[0].text, "Don't let yourself get attached");
    assert_eq!(found.cues[0].start, Timestamp::from_millis(1_000));
    assert_eq!(found.cues[0].end, Timestamp::from_millis(3_000));

    // The third line is far enough past the first that it cannot share its
    // cluster, so this is the timing surviving the cut as well as the offset.
    assert_eq!(found.cues[2].start, Timestamp::from_millis(90_000));
    assert_eq!(found.cues[2].end, Timestamp::from_millis(92_000));
}

#[test]
fn the_lines_are_numbered_in_playback_order() {
    let tracks = read(&film());
    let numbers: Vec<u32> = track(&tracks, 3).cues.iter().map(|cue| cue.index).collect();
    assert_eq!(numbers, [1, 2, 3]);
}

#[test]
fn several_tracks_are_read_in_the_one_pass() {
    let container = Container::new(vec![
        Entry::video(1),
        Entry::subtitle(2, "S_TEXT/UTF8").in_language("eng"),
        Entry::subtitle(3, "S_TEXT/UTF8").in_language("fre"),
    ])
    .with_dialogue(vec![
        Line::new(2, 1_000, 2_000, "Hello there"),
        Line::new(3, 1_000, 2_000, "Bonjour"),
        Line::new(2, 3_000, 4_000, "and goodbye"),
        Line::new(3, 3_000, 4_000, "et au revoir"),
    ]);

    let tracks = read(&container);
    assert_eq!(lines(&tracks, 2), ["Hello there", "and goodbye"]);
    assert_eq!(lines(&tracks, 3), ["Bonjour", "et au revoir"]);
}

#[test]
fn a_substation_alpha_track_gives_up_its_last_field() {
    let container = Container::new(vec![Entry::subtitle(1, "S_TEXT/ASS")]).with_dialogue(vec![
        Line::new(1, 1_000, 2_000, "0,0,Default,,0,0,0,,Hello there"),
        Line::new(
            1,
            3_000,
            4_000,
            "1,0,Default,Vincent,0,0,0,,Well, yes, but no",
        ),
    ]);

    assert_eq!(
        lines(&read(&container), 1),
        ["Hello there", "Well, yes, but no"]
    );
}

#[test]
fn the_markup_is_taken_out_the_way_the_parser_takes_it_out() {
    let container = Container::new(vec![Entry::subtitle(1, "S_TEXT/ASS")]).with_dialogue(vec![
        Line::new(
            1,
            1_000,
            2_000,
            "0,0,Default,,0,0,0,,{\\i1}Hello{\\i0} there",
        ),
        // A hard break, which is how one record carries two rows of dialogue.
        Line::new(1, 3_000, 4_000, "0,0,Default,,0,0,0,,First\\Nsecond"),
        // And a caption placed at the top, which is worth keeping.
        Line::new(1, 5_000, 6_000, "0,0,Default,,0,0,0,,{\\an8}Above"),
    ]);

    let tracks = read(&container);
    assert_eq!(lines(&tracks, 1), ["Hello there", "First\nsecond", "Above"]);
    assert_eq!(
        track(&tracks, 1).cues[2].position,
        Some(CuePosition::TopCentre)
    );
}

#[test]
fn a_track_of_pictures_is_named_and_left_unread() {
    let container = Container::new(vec![
        Entry::subtitle(1, "S_HDMV/PGS").in_language("eng"),
        Entry::subtitle(2, "S_TEXT/UTF8").in_language("eng"),
    ])
    .with_dialogue(vec![
        Line::new(1, 1_000, 2_000, "not text at all"),
        Line::new(2, 1_000, 2_000, "Hello there"),
    ]);

    let tracks = read(&container);
    assert_eq!(tracks.len(), 2);
    assert_eq!(track(&tracks, 1).track.codec, SubtitleCodec::Pgs);
    // Reported, since a menu has to be able to say what it is, and empty, since
    // nothing here turns pictures into words.
    assert!(track(&tracks, 1).cues.is_empty());
    assert_eq!(lines(&tracks, 2), ["Hello there"]);
}

#[test]
fn the_file_says_what_unit_its_timestamps_are_in() {
    let container = Container::new(vec![Entry::subtitle(1, "S_TEXT/UTF8")])
        // A tenth of a millisecond, which is legal and rare.
        .with_timestamp_scale(100_000)
        .with_dialogue(vec![Line::new(1, 1_500, 4_250, "Hello there")]);

    let tracks = read(&container);
    let cues = &track(&tracks, 1).cues;
    assert_eq!(cues[0].start, Timestamp::from_millis(1_500));
    assert_eq!(cues[0].end, Timestamp::from_millis(4_250));
}

#[test]
fn a_line_with_no_length_given_ends_where_it_starts() {
    let container = Container::new(vec![Entry::subtitle(1, "S_TEXT/UTF8")])
        .without_durations()
        .with_dialogue(vec![Line::new(1, 1_000, 5_000, "Hello there")]);

    // A simple block has nowhere to say how long a line is on screen, and
    // nothing here invents one. How long a line ought to be held to be read is
    // a question the reading comfort settings answer for every line.
    let tracks = read(&container);
    let cues = &track(&tracks, 1).cues;
    assert_eq!(cues[0].start, Timestamp::from_millis(1_000));
    assert_eq!(cues[0].end, Timestamp::from_millis(1_000));
}

#[test]
fn a_block_that_is_not_text_is_skipped_rather_than_read_as_rubbish() {
    let mut container =
        Container::new(vec![Entry::subtitle(1, "S_TEXT/UTF8")]).with_dialogue(vec![
            Line::new(1, 1_000, 2_000, "Hello there"),
            Line::new(1, 3_000, 4_000, "\u{fffd}"),
            Line::new(1, 5_000, 6_000, "and goodbye"),
        ]);

    // The replacement character written above is valid UTF-8, so the bytes have
    // to be damaged in the file itself to make the case this is about.
    let mut bytes = container.bytes();
    let at = bytes
        .windows(3)
        .position(|window| window == [0xEF, 0xBF, 0xBD])
        .expect("the marked block to be in there");
    bytes[at] = 0xFF;

    let tracks = subtitles_of(Cursor::new(bytes));
    assert_eq!(lines(&tracks, 1), ["Hello there", "and goodbye"]);

    // And a cue with nothing but markup in it, which the parser drops too.
    container = Container::new(vec![Entry::subtitle(1, "S_TEXT/ASS")]).with_dialogue(vec![
        Line::new(1, 1_000, 2_000, "0,0,Default,,0,0,0,,{\\an8}"),
        Line::new(1, 3_000, 4_000, "0,0,Default,,0,0,0,,Hello there"),
    ]);
    assert_eq!(lines(&read(&container), 1), ["Hello there"]);
}

#[test]
fn dialogue_is_found_among_the_picture_it_is_muxed_with() {
    let container = Container::new(vec![
        Entry::video(1),
        Entry::audio(2),
        Entry::subtitle(3, "S_TEXT/UTF8"),
    ])
    .with_seek_head()
    .with_picture(1, 40, 4_096)
    .with_dialogue(vec![
        Line::new(3, 1_000, 2_000, "Hello there"),
        Line::new(3, 60_000, 61_000, "and goodbye"),
    ]);

    assert_eq!(lines(&read(&container), 3), ["Hello there", "and goodbye"]);
}

/// A file recorded as it was written, where no cluster knows how long it is
/// until the next one starts.
#[test]
fn clusters_with_no_length_do_not_swallow_the_ones_after_them() {
    let bounded = film();
    let unbounded = film().with_unbounded_clusters();

    let lines: Vec<String> = read(&bounded)[0]
        .cues
        .iter()
        .map(|cue| cue.text.clone())
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines, lines_of(&read(&unbounded)));
}

/// The text of every cue of the first track, for comparing two readings.
fn lines_of(tracks: &[EmbeddedTrack]) -> Vec<String> {
    tracks
        .first()
        .map(|track| track.cues.iter().map(|cue| cue.text.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn a_film_with_no_dialogue_in_it_reads_as_none() {
    let container = Container::new(vec![
        Entry::video(1),
        Entry::subtitle(2, "S_TEXT/UTF8").in_language("eng"),
    ]);

    let tracks = read(&container);
    assert_eq!(tracks.len(), 1);
    assert!(tracks[0].cues.is_empty());

    // And a film with no subtitle tracks at all, which is not the same shape
    // and comes to the same answer.
    assert!(read(&Container::new(vec![Entry::video(1)])).is_empty());
}

#[test]
fn a_file_that_stops_part_way_through_gives_up_what_was_there() {
    let whole = film().bytes();

    for at in 0..whole.len() {
        let tracks = subtitles_of(Cursor::new(whole[..at].to_vec()));
        let read = tracks.first().map_or(0, |track| track.cues.len());
        // Either a line was whole before the cut or it was not. What matters is
        // that neither answer is a panic, and that no line is invented out of
        // the bytes that happened to be there.
        assert!(read <= 3, "{at} bytes read as {read} lines");
    }
}

#[test]
fn nothing_in_a_damaged_file_is_taken_as_a_length_to_trust() {
    let whole = film().bytes();

    for at in 0..whole.len() {
        for damage in [0x00, 0x01, 0xFF, 0x7F] {
            let mut damaged = whole.clone();
            damaged[at] = damage;
            let tracks = subtitles_of(Cursor::new(damaged));
            let read = tracks.first().map_or(0, |track| track.cues.len());
            assert!(read <= 3, "byte {at} set to {damage:#x} read as {read}");
        }
    }
}

#[test]
fn something_that_is_not_a_container_at_all_reads_as_nothing() {
    assert!(subtitles_of(Cursor::new(b"\x00\x00\x00\x20ftypisom".to_vec())).is_empty());
    assert!(subtitles_of(Cursor::new(Vec::new())).is_empty());
}

#[test]
fn a_film_on_disk_is_read_the_same_way() {
    let folder = TempDir::new().unwrap();
    let path = folder.path().join("Heat.1995.mkv");
    std::fs::write(&path, film().bytes()).unwrap();

    let tracks = extract(&path).expect("a film that is there to be readable");
    assert_eq!(track(&tracks, 3).cues.len(), 3);

    // A file that is not there is the caller's problem rather than an empty
    // answer, since the two mean different things to a scan.
    assert!(extract(&folder.path().join("gone.mkv")).is_err());
}

/// A film with a real header, real dialogue, and picture between the two that
/// this hands out on demand rather than holding.
///
/// The point of it is the counter. Reading the dialogue out of a film means
/// stepping over everything else in it, and how much of it is actually read is
/// the difference between opening a library of Matroska films and waiting for
/// one.
#[derive(Debug)]
struct Film {
    bytes: Vec<u8>,
    /// How many lines of dialogue were written into it.
    lines: usize,
    at: u64,
    read: u64,
}

impl Film {
    /// A film carrying `lines` of dialogue, with frames of picture between
    /// them in the quantity a real encode would have.
    fn of(lines: usize) -> Self {
        const FRAME: usize = 24 << 10;
        const FRAMES_PER_CLUSTER: usize = 64;

        let dialogue: Vec<Line> = (0..lines)
            .map(|at| {
                let start = at as u64 * 2_000;
                Line::new(3, start, start + 1_800, &format!("Line {at}"))
            })
            .collect();

        let bytes = Container::new(vec![
            Entry::video(1),
            Entry::audio(2),
            Entry::subtitle(3, "S_TEXT/UTF8").in_language("eng"),
        ])
        .with_seek_head()
        .with_picture(1, FRAMES_PER_CLUSTER, FRAME)
        .with_dialogue(dialogue)
        .bytes();

        Self {
            bytes,
            lines,
            at: 0,
            read: 0,
        }
    }
}

impl Read for Film {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let at = usize::try_from(self.at).unwrap_or(usize::MAX);
        let rest = self.bytes.len().saturating_sub(at);
        let wanted = buffer.len().min(rest);
        buffer[..wanted].copy_from_slice(&self.bytes[at..at + wanted]);

        self.at += wanted as u64;
        self.read += wanted as u64;
        Ok(wanted)
    }
}

impl Seek for Film {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.at = match to {
            SeekFrom::Start(at) => at,
            SeekFrom::End(back) => (self.bytes.len() as u64).saturating_add_signed(back),
            SeekFrom::Current(by) => self.at.saturating_add_signed(by),
        };
        Ok(self.at)
    }
}

/// The claim the extraction rests on, measured rather than asserted by
/// inspection: reading the dialogue out of a film costs the dialogue and the
/// headers in front of it, not the film.
#[test]
fn reading_the_dialogue_does_not_mean_reading_the_picture() {
    let mut film = Film::of(2_000);
    let length = film.bytes.len() as u64;

    let tracks = subtitles_of(&mut film);
    assert_eq!(track(&tracks, 3).cues.len(), film.lines);

    assert!(
        film.read * 16 < length,
        "{} bytes read of a film of {length}",
        film.read
    );
}
