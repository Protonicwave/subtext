//! The library, kept in line with what is on disk.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use subtext_index::{Database, WatchedFolder};

use crate::error::Result;
use crate::ingest::{self, ScanOutcome};
use crate::progress::ProgressSink;

/// Scans watched folders into the library.
///
/// Scans are serialised. A large one stands the search index triggers down for
/// the whole database and builds the index again at the end, which two scans
/// running at once would tread on. Serialising them costs nothing worth having:
/// the expensive part of a scan is already spread across every core, and two
/// folders scanned at once would only contend for the same single writer.
#[derive(Debug)]
pub struct Scanner {
    database: Database,
    gate: Mutex<()>,
}

impl Scanner {
    #[must_use]
    pub fn new(database: Database) -> Self {
        Self {
            database,
            gate: Mutex::new(()),
        }
    }

    #[must_use]
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Starts watching a folder, or returns the one already being watched.
    ///
    /// This only records it. Scanning it is a separate step, because the
    /// application wants to show the folder in the list before it has finished
    /// reading everything in it.
    pub fn add_folder(&self, path: &Path) -> Result<WatchedFolder> {
        Ok(self.database.folders().add(path)?)
    }

    /// Stops watching a folder and forgets what was found in it.
    pub fn remove_folder(&self, id: i64) -> Result<bool> {
        let removed = self.database.folders().remove(id)?;
        if removed {
            // Removing a folder is the only thing that deletes rows in any
            // quantity, so it is the one moment worth giving the pages back.
            self.database.compact()?;
        }
        Ok(removed)
    }

    pub fn folders(&self) -> Result<Vec<WatchedFolder>> {
        Ok(self.database.folders().list()?)
    }

    /// Brings one folder into line with what is on disk.
    pub fn scan(&self, folder: &WatchedFolder, sink: &dyn ProgressSink) -> Result<ScanOutcome> {
        // The gate guards no data of its own, so a thread that panicked while
        // holding it has left nothing behind to be poisoned by.
        let _guard = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        ingest::scan_folder(&self.database, folder, sink)
    }

    /// The same for every watched folder, oldest first.
    pub fn scan_all(&self, sink: &dyn ProgressSink) -> Result<Vec<ScanOutcome>> {
        self.folders()?
            .iter()
            .map(|folder| self.scan(folder, sink))
            .collect()
    }

    /// Rescans the folders that the given paths fall inside.
    ///
    /// This is what the watcher's callback ends in. A path in no watched folder
    /// is ignored, and a hundred paths in one folder are one scan.
    pub fn scan_containing(
        &self,
        paths: &[PathBuf],
        sink: &dyn ProgressSink,
    ) -> Result<Vec<ScanOutcome>> {
        let folders = self.folders()?;
        folders
            .iter()
            .filter(|folder| paths.iter().any(|path| path.starts_with(&folder.path)))
            .map(|folder| self.scan(folder, sink))
            .collect()
    }
}
