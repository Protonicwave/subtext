//! Reading what is inside a film file.
//!
//! Many films carry their subtitles inside the container rather than beside it,
//! muxed by whoever produced that encode and therefore timed against those
//! exact frames. Without this a film with no sidecar file contributes nothing
//! to the library beyond a poster: no transcript, no search, no dialogue in the
//! scrubber.
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
//! ```
//! use subtext_container::{SubtitleCodec, fixture, subtitle_tracks};
//! use std::io::Cursor;
//!
//! let film = fixture::Container::new(vec![
//!     fixture::Entry::video(1),
//!     fixture::Entry::subtitle(2, "S_TEXT/UTF8").in_language("fre"),
//! ]);
//!
//! let tracks = subtitle_tracks(Cursor::new(film.bytes()));
//! assert_eq!(tracks.len(), 1);
//! assert_eq!(tracks[0].codec, SubtitleCodec::SubRip);
//! assert_eq!(tracks[0].language, Some("fr"));
//! ```

mod codec;
mod ebml;
pub mod fixture;
mod probe;

pub use crate::codec::SubtitleCodec;
pub use crate::probe::{StreamTrack, probe, subtitle_tracks};
