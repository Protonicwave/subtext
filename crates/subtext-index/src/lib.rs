//! Persistence and search for Subtext.
//!
//! Wraps a single SQLite database holding watched folders, films, subtitle
//! tracks, cues, playback positions and preferences, together with the full text
//! index over cue text. Callers work through repositories rather than SQL.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! use subtext_index::{Database, SearchOptions};
//!
//! let database = Database::open("library.db")?;
//! let results = database.search().find("we'll always have", &SearchOptions::default())?;
//! for film in &results.films {
//!     for hit in &film.hits {
//!         println!("{} at {}: {}", film.title, hit.start, hit.text);
//!     }
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
    FilmRecord, Fingerprint, NewFilm, NewTrack, PlaybackPosition, Resumable, Stored, TrackMatch,
    TrackPairing, TrackRecord, WatchedFolder,
};
pub use crate::repository::{
    CueHit, FilmHits, Films, Folders, MATCH_END, MATCH_START, Positions, Preferences, Search,
    SearchOptions, SearchResults, Tracks,
};
