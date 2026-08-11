//! What the application holds while it is running.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use subtext_index::Database;
use subtext_scan::{FolderWatcher, ProgressSink, ScanOutcome, ScanProgress, Scanner};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::dto::{Failure, ScanFailed, ScanFinished, ScanProgressed, ScanSummary};
use crate::settings;

/// The library, the scanner, and the watches on the folders it came from.
pub(crate) struct AppState {
    scanner: Arc<Scanner>,
    /// None when the platform would not give us file change notification, in
    /// which case folders are still scanned, just not by themselves.
    watcher: Mutex<Option<FolderWatcher>>,
    /// The last thing a scan said, for a window that opened part way through
    /// one and so has not seen any events yet.
    latest: Mutex<Option<ScanProgressed>>,
    scanning: AtomicBool,
}

impl AppState {
    /// Opens the library and prepares to scan, without scanning yet.
    pub(crate) fn open(database: Database) -> Self {
        Self {
            scanner: Arc::new(Scanner::new(database)),
            watcher: Mutex::new(None),
            latest: Mutex::new(None),
            scanning: AtomicBool::new(false),
        }
    }

    pub(crate) fn scanner(&self) -> &Arc<Scanner> {
        &self.scanner
    }

    pub(crate) fn latest_progress(&self) -> Option<ScanProgressed> {
        *lock(&self.latest)
    }

    pub(crate) fn is_scanning(&self) -> bool {
        self.scanning.load(Ordering::Relaxed)
    }

    pub(crate) fn is_watching(&self) -> bool {
        lock(&self.watcher).is_some()
    }

    /// Starts noticing changes to the folders already being watched.
    ///
    /// A platform that will not watch folders makes for a lesser application,
    /// not a broken one, so the reason is reported and everything else carries
    /// on without it.
    pub(crate) fn start_watching<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), Failure> {
        let handle = app.clone();
        let watcher = FolderWatcher::new(move |paths| {
            let Some(state) = handle.try_state::<AppState>() else {
                return;
            };
            state.scanning(&handle, |scanner, sink| {
                scanner.scan_containing(&paths, sink)
            });
        })
        .map_err(Failure::of)?;

        *lock(&self.watcher) = Some(watcher);

        for folder in &self.scanner.folders().map_err(Failure::of)? {
            self.watch(&folder.path)?;
        }
        Ok(())
    }

    pub(crate) fn watch(&self, path: &Path) -> Result<(), Failure> {
        match lock(&self.watcher).as_mut() {
            Some(watcher) => watcher.watch(path).map_err(Failure::of),
            // Nothing is watching, which the folder list already says.
            None => Ok(()),
        }
    }

    pub(crate) fn unwatch(&self, path: &Path) -> Result<(), Failure> {
        match lock(&self.watcher).as_mut() {
            Some(watcher) => watcher.unwatch(path).map_err(Failure::of),
            None => Ok(()),
        }
    }

    /// Runs a scan, reporting it to the front end as it goes.
    ///
    /// Returns nothing, because a scan is not something a caller waits for. It
    /// says where it has got to through events, and whoever asked for it has
    /// already been told that it started.
    pub(crate) fn scanning<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        work: impl FnOnce(&Scanner, &dyn ProgressSink) -> subtext_scan::Result<Vec<ScanOutcome>>,
    ) {
        self.scanning.store(true, Ordering::Relaxed);

        // The one place every scan passes through, whether it was asked for or
        // started by the watcher, and so the place the pairing preference is
        // read. Read afresh each time rather than pushed when it changes, since
        // a scan already running is one whose mind it is too late to change.
        self.scanner
            .set_matching(settings::matching(self.scanner.database()));

        let report = |progress: &ScanProgress| {
            let event = ScanProgressed::of(progress);
            *lock(&self.latest) = Some(event);
            let _ = event.emit(app);
        };

        let outcome = work(&self.scanner, &report);
        self.scanning.store(false, Ordering::Relaxed);

        let _ = match outcome {
            Ok(outcomes) => ScanFinished(outcomes.iter().map(ScanSummary::of).collect()).emit(app),
            Err(error) => ScanFailed(Failure::of(error)).emit(app),
        };
    }
}

/// Where the library file lives.
///
/// One file in the application's own data directory. Nothing is ever written
/// beside the films themselves: a watched folder is read, never added to.
pub(crate) fn library_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, Failure> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| Failure::saying("there is nowhere to keep the library on this machine"))?;
    std::fs::create_dir_all(&directory).map_err(Failure::of)?;
    Ok(directory.join("library.db"))
}

/// The locks here guard no invariant of their own, so a thread that panicked
/// while holding one left nothing behind to be poisoned by.
fn lock<T>(held: &Mutex<T>) -> MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}
