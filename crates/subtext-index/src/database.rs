//! The library database.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use crate::error::Result;
use crate::migrate;
use crate::pool::Pool;
use crate::repository::{Films, Folders, Positions, Preferences, Tracks};

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

    /// Where each film was left.
    #[must_use]
    pub fn positions(&self) -> Positions<'_> {
        Positions::new(self)
    }

    /// Application preferences.
    #[must_use]
    pub fn preferences(&self) -> Preferences<'_> {
        Preferences::new(self)
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
