//! The library, kept in line with what is on disk.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use subtext_core::Matching;
use subtext_index::{Database, WatchedFolder};

use crate::attach::{self, Attached};
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
    /// Whether a subtitle needs a name that agrees exactly with its film. Held
    /// here rather than passed to each scan, because it is a preference that
    /// outlives any one of them and the folder watcher starts scans of its own.
    exact_matching: AtomicBool,
}

impl Scanner {
    #[must_use]
    pub fn new(database: Database) -> Self {
        Self {
            database,
            gate: Mutex::new(()),
            exact_matching: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Says how much evidence a pairing needs from here on.
    ///
    /// Takes effect on the next scan rather than on what is already stored: a
    /// pairing that has been made stays made until something rereads the folder
    /// it was made in.
    pub fn set_matching(&self, matching: Matching) {
        self.exact_matching
            .store(matching == Matching::Exact, Ordering::Relaxed);
    }

    fn matching(&self) -> Matching {
        if self.exact_matching.load(Ordering::Relaxed) {
            Matching::Exact
        } else {
            Matching::Relaxed
        }
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
        ingest::scan_folder(&self.database, folder, self.matching(), sink)
    }

    /// The same for every watched folder, oldest first.
    pub fn scan_all(&self, sink: &dyn ProgressSink) -> Result<Vec<ScanOutcome>> {
        self.folders()?
            .iter()
            .map(|folder| self.scan(folder, sink))
            .collect()
    }

    /// Gives a subtitle file to a film because somebody said so.
    ///
    /// Behind the same gate as a scan, so that a rescan cannot be part way
    /// through deciding where this file belongs while the answer is being
    /// written.
    pub fn attach_subtitle(&self, film_id: i64, path: &Path) -> Result<Attached> {
        let _guard = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        attach::attach_subtitle(&self.database, film_id, path)
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
            .filter(|folder| paths.iter().any(|path| is_inside(path, &folder.path)))
            .map(|folder| self.scan(folder, sink))
            .collect()
    }
}

/// Whether a path that changed falls inside a watched folder.
///
/// Compared as written first, and then again with both sides resolved. The
/// platform reports the path it watched rather than the path it was given, and
/// those differ wherever a symbolic link stands between them: on macOS a folder
/// under `/var` is reported under `/private/var`, and anywhere at all a folder
/// reached through a link is reported at the other end of it.
///
/// A path that has just been deleted cannot be resolved, so its parent is tried
/// instead. Being wrong here costs a rescan that finds nothing, or a change
/// noticed on the next one.
fn is_inside(path: &Path, folder: &Path) -> bool {
    if path.starts_with(folder) {
        return true;
    }

    let resolved = path
        .canonicalize()
        .ok()
        .or_else(|| path.parent()?.canonicalize().ok());
    match (resolved, folder.canonicalize()) {
        (Some(path), Ok(folder)) => path.starts_with(folder),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::is_inside;

    #[test]
    fn a_path_is_inside_the_folder_it_is_written_under() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("films");
        std::fs::create_dir_all(folder.join("Nineteen Nineties")).unwrap();

        assert!(is_inside(&folder.join("Heat.1995.mkv"), &folder));
        assert!(is_inside(
            &folder.join("Nineteen Nineties/Heat.1995.srt"),
            &folder
        ));
        assert!(is_inside(&folder, &folder));

        assert!(!is_inside(
            &directory.path().join("elsewhere/Heat.mkv"),
            &folder
        ));
    }

    #[test]
    fn a_folder_reported_by_another_name_is_still_the_same_folder() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("films");
        std::fs::create_dir_all(&folder).unwrap();

        // What a platform that resolves the path before watching it reports.
        let roundabout = directory.path().join("films/../films/Heat.1995.mkv");
        assert!(is_inside(&roundabout, &folder));
    }
}
