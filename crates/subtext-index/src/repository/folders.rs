//! The folders the library is a view onto.

use std::path::Path;

use rusqlite::{OptionalExtension, Row, params};

use crate::clock::now_millis;
use crate::database::Database;
use crate::error::Result;
use crate::model::WatchedFolder;
use crate::repository::path_text;

#[derive(Debug)]
pub struct Folders<'a> {
    database: &'a Database,
}

impl<'a> Folders<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Starts watching a folder, or returns the one already being watched.
    ///
    /// Adding the same folder twice is something a person does by accident, and
    /// it should be a no-op rather than an error or a second copy of every film
    /// in it.
    pub fn add(&self, path: &Path) -> Result<WatchedFolder> {
        let path = path_text(path)?;
        self.database.with(|connection| {
            connection.execute(
                "INSERT OR IGNORE INTO watched_folder (path, added_at) VALUES (?1, ?2)",
                params![path, now_millis()],
            )?;
            let folder = connection.query_row(
                "SELECT id, path, added_at FROM watched_folder WHERE path = ?1",
                [path],
                from_row,
            )?;
            Ok(folder)
        })
    }

    /// Every watched folder, oldest first.
    pub fn list(&self) -> Result<Vec<WatchedFolder>> {
        self.database.with(|connection| {
            let mut statement = connection
                .prepare("SELECT id, path, added_at FROM watched_folder ORDER BY added_at, id")?;
            let folders = statement
                .query_map([], from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(folders)
        })
    }

    pub fn find(&self, path: &Path) -> Result<Option<WatchedFolder>> {
        let path = path_text(path)?;
        self.database.with(|connection| {
            let folder = connection
                .query_row(
                    "SELECT id, path, added_at FROM watched_folder WHERE path = ?1",
                    [path],
                    from_row,
                )
                .optional()?;
            Ok(folder)
        })
    }

    /// Stops watching a folder and forgets what was found in it.
    ///
    /// This is the one destructive operation in the crate: the films, tracks,
    /// cues and playback positions underneath go with it. Anything that should
    /// outlive the folder has to be somewhere else.
    pub fn remove(&self, id: i64) -> Result<bool> {
        self.database.with(|connection| {
            let removed = connection.execute("DELETE FROM watched_folder WHERE id = ?1", [id])?;
            Ok(removed > 0)
        })
    }
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<WatchedFolder> {
    Ok(WatchedFolder {
        id: row.get(0)?,
        path: row.get::<_, String>(1)?.into(),
        added_at: row.get(2)?,
    })
}
