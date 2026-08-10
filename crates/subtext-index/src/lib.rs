//! Persistence and search for Subtext.
//!
//! Wraps a single SQLite database holding watched folders, films, subtitle
//! tracks, cues, playback positions and preferences, together with the full text
//! index over cue text. Callers work through repositories rather than SQL.

// Everything here fails for the same reason and returns the same type when it
// does: the database refused the request. Repeating that under forty method
// headings would say nothing the error type does not already say.
#![allow(clippy::missing_errors_doc)]

mod clock;
mod database;
mod error;
mod migrate;
mod model;
mod pool;
mod repository;

pub use crate::database::Database;
pub use crate::error::{Error, Result};
pub use crate::model::{
    FilmRecord, Fingerprint, NewFilm, NewTrack, PlaybackPosition, Resumable, Stored, TrackMatch,
    TrackRecord, WatchedFolder,
};
pub use crate::repository::{Films, Folders, Positions, Preferences, Tracks};
