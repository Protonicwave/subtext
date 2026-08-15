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
mod ebml;
mod extract;
pub mod fixture;
mod ids;
mod probe;
mod segment;

pub use crate::codec::SubtitleCodec;
pub use crate::extract::{EmbeddedTrack, extract, subtitles_of};
pub use crate::probe::{StreamTrack, probe, subtitle_tracks};
