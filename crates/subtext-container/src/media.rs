//! What a film is, as its own header describes it.
//!
//! The same read the subtitle probe makes, asked a different question. A film
//! says what it is encoded in, how large its picture is, how many channels its
//! sound carries and how long it runs, and all of it sits in the header beside
//! the track list. Reading it costs what reading the track list costs, which is
//! a few hundred bytes and two seeks however long the film is.
//!
//! Nothing here decodes anything. A codec is a name to put on a screen and
//! nothing more, which is why picture and sound have no type of their own the
//! way subtitles do: there is no decision anywhere in the application that
//! turns on which of them a film happens to be in.
//!
//! Everything is optional, and deliberately. A file that does not say how deep
//! its colour is has not said, and reporting eight would be inventing a fact
//! about somebody's film.

use std::io::{Read, Seek};
use std::path::Path;

use crate::ebml::{Element, Reader, as_flag, as_float, as_text, as_uint, children};
use crate::ids;
use crate::probe::{ASSUMED_LANGUAGE, TRACK_LIMIT, language_of};
use crate::segment::{self, CHILDREN_LIMIT};

/// A film's picture, as its header describes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VideoStream {
    /// The identifier the file wrote, which [`video_codec_name`] gives a name
    /// to where it is one anybody says out loud.
    ///
    /// [`video_codec_name`]: crate::video_codec_name
    pub codec: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Bits per colour channel, which is what tells a ten bit encode from an
    /// eight bit one.
    pub bit_depth: Option<u8>,
    /// Frames a second, worked out from how long the file says one frame lasts.
    pub frame_rate: Option<f64>,
}

/// One of a film's sound tracks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioStream {
    /// The number the container knows this track by, which is what tells two
    /// tracks apart when they say the same thing about themselves.
    pub number: u64,
    pub codec: String,
    /// How many channels the track carries, from which the layout is named.
    pub channels: Option<u8>,
    /// The two letter code, in the vocabulary the rest of the application uses.
    pub language: Option<&'static str>,
    /// The track the film suggests, which is the one that will be heard.
    pub default: bool,
}

/// What a film turned out to be.
///
/// A file that is not Matroska, or is damaged, or says nothing about itself,
/// all come back the same way: with nothing filled in. That is the honest
/// answer, and it is one an unreadable file must not be allowed to turn into a
/// failed scan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaStreams {
    /// How long the film runs, where the header says.
    pub duration_ms: Option<u32>,
    /// The picture. One track, since a film has one and a file carrying two is
    /// describing something this application has no screen for.
    pub video: Option<VideoStream>,
    pub audio: Vec<AudioStream>,
}

impl MediaStreams {
    /// Whether the header said anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.duration_ms.is_none() && self.video.is_none() && self.audio.is_empty()
    }
}

/// What a film says it is, read from its header.
///
/// # Errors
///
/// Only if the file cannot be opened at all.
pub fn media(path: &Path) -> std::io::Result<MediaStreams> {
    let file = std::fs::File::open(path)?;
    Ok(media_in(file))
}

/// The same, from anything that can be read and seeked.
#[must_use]
pub fn media_in<R: Read + Seek>(source: R) -> MediaStreams {
    read(source).unwrap_or_default()
}

fn read<R: Read + Seek>(source: R) -> Option<MediaStreams> {
    let mut reader = Reader::open(source)?;
    let segment = segment::of(&mut reader)?;

    let duration_ms = duration_of(&mut reader, segment);
    let (video, audio) = streams_of(&mut reader, segment);

    Some(MediaStreams {
        duration_ms,
        video,
        audio,
    })
}

/// How long the film runs, in milliseconds.
///
/// Written as a count of the file's own timestamp units rather than of
/// anything absolute, so both halves have to be read for either to mean
/// something.
fn duration_of<R: Read + Seek>(reader: &mut Reader<R>, segment: Element) -> Option<u32> {
    let info = segment::locate(reader, segment, ids::INFO)?;
    let payload = reader.payload(info)?;

    let mut scale = None;
    let mut units = None;
    for (id, value) in children(&payload, CHILDREN_LIMIT) {
        match id {
            ids::TIMESTAMP_SCALE => scale = as_uint(&value),
            ids::DURATION => units = as_float(&value),
            _ => {}
        }
    }

    // A running time of nothing is a file that has not said, and a scale of
    // nothing would make every film in the library instantaneous.
    let units = units.filter(|units| *units > 0.0)?;
    let scale = scale.filter(|scale| *scale > 0)?;

    // The arithmetic is done at f64 and the answer is a count of milliseconds
    // that a u32 holds for any film shorter than seven weeks.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    let millis = (units * scale as f64 / 1_000_000.0) as u64;
    u32::try_from(millis).ok()
}

/// The picture track and the sound tracks a film declares.
fn streams_of<R: Read + Seek>(
    reader: &mut Reader<R>,
    segment: Element,
) -> (Option<VideoStream>, Vec<AudioStream>) {
    let Some(payload) =
        segment::locate(reader, segment, ids::TRACKS).and_then(|tracks| reader.payload(tracks))
    else {
        return (None, Vec::new());
    };

    let mut video = None;
    let mut audio = Vec::new();

    for (id, body) in children(&payload, TRACK_LIMIT) {
        if id != ids::TRACK_ENTRY {
            continue;
        }
        match entry_in(&body) {
            Some(Entry::Video(picture)) if video.is_none() => video = Some(picture),
            Some(Entry::Audio(sound)) => audio.push(sound),
            _ => {}
        }
    }

    // In the order the file numbers them, so that the sheet reads the same way
    // twice regardless of how the header was laid out.
    audio.sort_by_key(|track| track.number);
    (video, audio)
}

/// One track entry, if it is picture or sound.
enum Entry {
    Video(VideoStream),
    Audio(AudioStream),
}

fn entry_in(payload: &[u8]) -> Option<Entry> {
    let mut number = None;
    let mut kind = None;
    let mut codec = None;
    let mut language = None;
    let mut tagged = None;
    let mut frame_nanos = None;
    let mut picture = None;
    let mut sound = None;
    let mut default = true;

    for (id, value) in children(payload, CHILDREN_LIMIT) {
        match id {
            ids::TRACK_NUMBER => number = as_uint(&value),
            ids::TRACK_TYPE => kind = as_uint(&value),
            ids::CODEC_ID => codec = as_text(&value),
            ids::LANGUAGE => language = as_text(&value),
            ids::LANGUAGE_BCP47 => tagged = as_text(&value),
            ids::FLAG_DEFAULT => default = as_flag(&value),
            ids::DEFAULT_DURATION => frame_nanos = as_uint(&value),
            ids::VIDEO_SETTINGS => picture = Some(value),
            ids::AUDIO_SETTINGS => sound = Some(value),
            _ => {}
        }
    }

    let codec = codec?;
    match kind? {
        ids::VIDEO => {
            let (width, height, bit_depth) =
                picture.map_or((None, None, None), |body| picture_in(&body));
            Some(Entry::Video(VideoStream {
                codec,
                width,
                height,
                bit_depth,
                frame_rate: frame_rate_of(frame_nanos),
            }))
        }
        ids::AUDIO => {
            let channels = sound.and_then(|body| channels_in(&body));
            // A track written after the newer element exists carries both, and
            // the newer one wins where they disagree.
            let declared = tagged.or(language);
            Some(Entry::Audio(AudioStream {
                number: number?,
                codec,
                channels,
                language: language_of(declared.as_deref().unwrap_or(ASSUMED_LANGUAGE)),
                default,
            }))
        }
        _ => None,
    }
}

/// What a picture track says about its pictures.
fn picture_in(payload: &[u8]) -> (Option<u32>, Option<u32>, Option<u8>) {
    let mut width = None;
    let mut height = None;
    let mut bit_depth = None;

    for (id, value) in children(payload, CHILDREN_LIMIT) {
        match id {
            ids::PIXEL_WIDTH => width = as_uint(&value).and_then(|it| u32::try_from(it).ok()),
            ids::PIXEL_HEIGHT => height = as_uint(&value).and_then(|it| u32::try_from(it).ok()),
            ids::COLOUR => bit_depth = bit_depth_in(&value),
            _ => {}
        }
    }

    // Nothing has a dimension of nothing, and a file that writes one has said
    // less than it appears to.
    (
        width.filter(|it| *it > 0),
        height.filter(|it| *it > 0),
        bit_depth,
    )
}

/// How deep the colour is, out of the colour description it is written inside.
fn bit_depth_in(payload: &[u8]) -> Option<u8> {
    children(payload, CHILDREN_LIMIT)
        .into_iter()
        .find_map(|(id, value)| (id == ids::BITS_PER_CHANNEL).then(|| as_uint(&value)))
        .flatten()
        .and_then(|depth| u8::try_from(depth).ok())
        .filter(|depth| *depth > 0)
}

/// How many channels a sound track carries.
fn channels_in(payload: &[u8]) -> Option<u8> {
    children(payload, CHILDREN_LIMIT)
        .into_iter()
        .find_map(|(id, value)| (id == ids::CHANNELS).then(|| as_uint(&value)))
        .flatten()
        .and_then(|channels| u8::try_from(channels).ok())
        .filter(|channels| *channels > 0)
}

/// Frames a second, from how long the file says one frame lasts.
///
/// The only place a Matroska file states a frame rate, and plenty of them do
/// not state it at all. A film shot at twenty-three point nine seven six comes
/// back as that rather than as twenty-four, which is the number it is.
fn frame_rate_of(frame_nanos: Option<u64>) -> Option<f64> {
    // A frame lasting no time at all would be a rate of infinity, and one
    // lasting a second and a half is not a film.
    let nanos = frame_nanos.filter(|nanos| *nanos > 0)?;
    #[allow(clippy::cast_precision_loss)]
    let rate = 1_000_000_000.0 / nanos as f64;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}

#[cfg(test)]
mod tests {
    use super::frame_rate_of;

    #[test]
    fn a_frame_rate_is_what_a_frames_length_makes_it() {
        // Twenty-five, and the twenty-four that is not quite twenty-four.
        assert_eq!(frame_rate_of(Some(40_000_000)), Some(25.0));
        let cinema = frame_rate_of(Some(41_708_333)).unwrap_or_default();
        assert!((cinema - 23.976).abs() < 0.001);
    }

    #[test]
    fn a_frame_that_lasts_no_time_is_not_a_rate() {
        assert_eq!(frame_rate_of(None), None);
        assert_eq!(frame_rate_of(Some(0)), None);
    }
}
