//! Reading what is inside a film file.
//!
//! Many films carry their subtitles inside the container rather than beside it,
//! muxed by whoever produced that encode and therefore timed against those
//! exact frames. Without this a film with no sidecar file has no dialogue at
//! all: no subtitles to draw, nothing to step between, and nothing to line up
//! against the film.
//!
//! The crate reads. It does not decode, transcode, or write anything back. A
//! file is opened, a bounded number of bytes are read from known offsets, and
//! structured data comes back. It knows nothing about databases, applications
//! or the filesystem beyond opening one path, which is what lets it be tested
//! against a handful of bytes.
//!
//! Matroska only, which is what films with embedded subtitles are distributed
//! in. MP4 carries timed text so rarely that a second reader for it would be
//! code nobody exercises.
//!
//! There are two questions to ask of a film and they cost different things.
//! [`probe`] reads the header and says which tracks are in there, which is a
//! few hundred bytes however long the film is. [`extract`] reads the dialogue
//! as well, which means stepping over every frame between one line and the
//! next.
//!
//! A film also says what it is: what its picture and sound are encoded in, how
//! large the picture is and how long the film runs. [`media`] reads that, out
//! of the same header the probe reads, and it is the difference between a film
//! sheet that describes the file and one that describes the file name.
//!
//! A film may also carry its own artwork, which [`cover`] finds and
//! [`cover_image`] reads. Neither of them touches the picture either.
//!
//! ```
//! use subtext_container::{SubtitleCodec, fixture, subtitles_of};
//! use std::io::Cursor;
//!
//! let film = fixture::Container::new(vec![
//!     fixture::Entry::video(1),
//!     fixture::Entry::subtitle(2, "S_TEXT/UTF8").in_language("fre"),
//! ])
//! .with_dialogue(vec![fixture::Line::new(2, 1_000, 3_000, "Bonjour")]);
//!
//! let tracks = subtitles_of(Cursor::new(film.bytes()));
//! assert_eq!(tracks.len(), 1);
//! assert_eq!(tracks[0].track.codec, SubtitleCodec::SubRip);
//! assert_eq!(tracks[0].track.language, Some("fr"));
//! assert_eq!(tracks[0].cues[0].text, "Bonjour");
//! ```

mod buffer;
mod codec;
mod cover;
mod ebml;
mod extract;
pub mod fixture;
mod ids;
mod media;
mod probe;
mod segment;

pub use crate::codec::{SubtitleCodec, audio_codec_name, video_codec_name};
pub use crate::cover::{AttachedCover, cover, cover_image, cover_image_in, cover_in};
pub use crate::extract::{EmbeddedTrack, extract, subtitles_of};
pub use crate::media::{AudioStream, MediaStreams, VideoStream, media, media_in};
pub use crate::probe::{StreamTrack, probe, subtitle_tracks};
