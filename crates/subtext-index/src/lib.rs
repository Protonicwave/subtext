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
mod pool;

pub use crate::database::Database;
pub use crate::error::{Error, Result};
