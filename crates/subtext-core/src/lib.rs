//! The domain core of Subtext.
//!
//! This crate holds the types that describe a film, a subtitle track and a cue,
//! the SRT parser, and the logic that works out which subtitle file belongs to
//! which film. It performs no I/O and has no internal dependencies, so it can be
//! tested and benchmarked without an application around it.

mod cue;
mod time;
mod track;

pub use crate::cue::{Cue, CuePosition};
pub use crate::time::Timestamp;
pub use crate::track::SubtitleTrack;
