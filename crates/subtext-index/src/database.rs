//! The library database.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use crate::error::Result;
use crate::migrate;
use crate::pool::Pool;
use crate::repository::{Films, Folders, Tracks};

/// One SQLite file holding everything Subtext knows.
///
/// Cloning is cheap and shares the same connections, so this is held once in
/// the application state and handed to whatever needs it.
#[derive(Clone, Debug)]
pub struct Database {
    pool: Arc<Pool>,
}

impl Database {
    /// Opens the database at `path`, creating it if it is not there, and brings
    /// the schema up to date.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let database = Self {
            pool: Arc::new(Pool::new(PathBuf::from(path.as_ref()))),
        };
        database.pool.with(migrate::apply)?;

        // A scan that was interrupted left the search index behind the cues it
        // is meant to describe. Nothing may search it until that is put right.
        if database.bulk_ingest_was_interrupted()? {
            database.rebuild_search_index()?;
        }

        Ok(database)
    }

    /// The schema version this build understands.
    #[must_use]
    pub fn schema_version() -> u32 {
        migrate::supported_version()
    }

    /// Watched folders.
    #[must_use]
    pub fn folders(&self) -> Folders<'_> {
        Folders::new(self)
    }

    /// Films and their fingerprints.
    #[must_use]
    pub fn films(&self) -> Films<'_> {
        Films::new(self)
    }

    /// Subtitle tracks and their cues.
    #[must_use]
    pub fn tracks(&self) -> Tracks<'_> {
        Tracks::new(self)
    }

    /// Runs a large ingest with the search index built once at the end.
    ///
    /// Keeping the index in step cue by cue is right for a film being added to
    /// a library that is already there, and wrong for the first scan of a
    /// thousand of them: building the index in one pass is roughly ten times
    /// faster than feeding a million cues through it one at a time.
    ///
    /// The index is rebuilt whether the work succeeded or not, so a scan that
    /// gave up half way still leaves the index describing what was written.
    /// Calls do not nest.
    pub fn bulk_ingest<T>(&self, work: impl FnOnce() -> Result<T>) -> Result<T> {
        self.set_bulk(true)?;
        let outcome = work();
        self.rebuild_search_index()?;
        outcome
    }

    /// Builds the search index again from the cues, and puts the database back
    /// into keeping it in step.
    ///
    /// Also what the rebuild button in the settings does.
    pub fn rebuild_search_index(&self) -> Result<()> {
        self.with(|connection| {
            connection.execute_batch(
                "INSERT INTO cue_search (cue_search) VALUES ('rebuild');
                 UPDATE index_state SET bulk = 0 WHERE id = 0;",
            )?;
            Ok(())
        })
    }

    fn bulk_ingest_was_interrupted(&self) -> Result<bool> {
        self.with(|connection| {
            let bulk: i64 =
                connection.query_row("SELECT bulk FROM index_state WHERE id = 0", [], |row| {
                    row.get(0)
                })?;
            Ok(bulk != 0)
        })
    }

    fn set_bulk(&self, bulk: bool) -> Result<()> {
        self.with(|connection| {
            connection.execute("UPDATE index_state SET bulk = ?1 WHERE id = 0", [bulk])?;
            Ok(())
        })
    }

    /// Hands one piece of work a connection.
    pub(crate) fn with<T>(&self, work: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        self.pool.with(work)
    }

    /// Returns unused pages to the filesystem and updates the query planner's
    /// statistics.
    ///
    /// Worth doing after a folder has been removed, which is the only operation
    /// that deletes rows in any quantity.
    pub fn compact(&self) -> Result<()> {
        self.with(|connection| {
            connection.execute_batch("PRAGMA optimize; VACUUM;")?;
            Ok(())
        })
    }
}
