//! Persistence for Subtext.
//!
//! Wraps a single SQLite database holding watched folders, films, subtitle
//! tracks, cues, playback positions and preferences. Callers work through
//! repositories rather than SQL.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! use subtext_index::Database;
//!
//! let database = Database::open("library.db")?;
//! for film in database.films().list()? {
//!     println!("{} at {}", film.title, film.path.display());
//! }
//! # Ok(())
//! # }
//! ```

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
    AudioDetails, FilmRecord, FilmStreams, Fingerprint, MediaDetails, NewFilm, NewTrack,
    PlaybackPosition, Resumable, Stored, StreamEntry, TrackChoice, TrackMatch, TrackOrigin,
    TrackPairing, TrackRecord, VideoDetails, WatchedFolder,
};
pub use crate::repository::{Details, Films, Folders, Positions, Preferences, Tracks};
