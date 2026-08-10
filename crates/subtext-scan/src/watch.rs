//! Noticing that a watched folder has changed.

use core::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::debounce::{PATIENCE, QUIET, Settling};
use crate::error::{Error, Result};
use crate::media;

/// Watches folders and says when one has finished changing.
///
/// The callback is handed the paths that changed, once. What it does with them
/// is the application's business: in practice it rescans the folders they are
/// in, which is cheap when nothing else has moved.
pub struct FolderWatcher {
    // In an Option so that it can be dropped before the settling thread is
    // waited on. Dropping it closes the channel, which is the only way the
    // thread is told to stop.
    watcher: Option<RecommendedWatcher>,
    settling: Option<JoinHandle<()>>,
}

impl FolderWatcher {
    /// Starts watching, with the standard quiet period.
    pub fn new(on_settled: impl Fn(Vec<PathBuf>) + Send + 'static) -> Result<Self> {
        Self::with_quiet(QUIET, on_settled)
    }

    /// The same, with the quiet period given, which the tests shorten.
    pub fn with_quiet(
        quiet: Duration,
        on_settled: impl Fn(Vec<PathBuf>) + Send + 'static,
    ) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |event| {
            // The receiver has gone, which means the application is closing.
            let _ = sender.send(event);
        })
        .map_err(|error| Error::WatchUnavailable(error.to_string()))?;

        let settling = std::thread::Builder::new()
            .name("subtext-watch".to_owned())
            .spawn(move || settle(&receiver, quiet, &on_settled))
            .map_err(|error| Error::WatchUnavailable(error.to_string()))?;

        Ok(Self {
            watcher: Some(watcher),
            settling: Some(settling),
        })
    }

    /// Watches a folder and everything under it.
    ///
    /// Watching the same folder twice is harmless: the platform replaces the
    /// first watch with the second rather than reporting everything twice.
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        self.watcher()?
            .watch(path, RecursiveMode::Recursive)
            .map_err(|error| Error::Watch {
                path: path.to_path_buf(),
                reason: error.to_string(),
            })
    }

    /// Stops watching a folder. A folder that was not being watched is not an
    /// error, since the reason for removing it is usually that it is gone.
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        match self.watcher()?.unwatch(path) {
            Ok(())
            | Err(notify::Error {
                kind: notify::ErrorKind::WatchNotFound,
                ..
            }) => Ok(()),
            Err(error) => Err(Error::Watch {
                path: path.to_path_buf(),
                reason: error.to_string(),
            }),
        }
    }

    fn watcher(&mut self) -> Result<&mut RecommendedWatcher> {
        self.watcher
            .as_mut()
            .ok_or_else(|| Error::WatchUnavailable("the watcher has been shut down".to_owned()))
    }
}

impl fmt::Debug for FolderWatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FolderWatcher")
            .field("running", &self.settling.is_some())
            .finish_non_exhaustive()
    }
}

impl Drop for FolderWatcher {
    fn drop(&mut self) {
        // Closing the channel is what stops the settling thread, so the watcher
        // has to go first and the thread is waited on rather than left to find
        // out during shutdown.
        drop(self.watcher.take());
        if let Some(settling) = self.settling.take() {
            let _ = settling.join();
        }
    }
}

/// Collects changes until the folder is quiet, then hands them over.
///
/// Ends when the watcher is dropped. Anything still waiting at that point is
/// discarded: the application is closing, and the next scan will find it.
fn settle(
    receiver: &Receiver<notify::Result<Event>>,
    quiet: Duration,
    on_settled: &(impl Fn(Vec<PathBuf>) + Send + 'static),
) {
    // Often enough that the quiet period is what decides the delay rather than
    // the checking interval, and rarely enough to cost nothing while idle.
    let tick = (quiet / 3).max(Duration::from_millis(25));
    let mut settling = Settling::new(quiet, PATIENCE);

    loop {
        match receiver.recv_timeout(tick) {
            Ok(Ok(event)) => note(&mut settling, &event),
            // The platform dropped events, usually because too many arrived at
            // once. The paths it managed to name are worth looking at, and a
            // rescan settles anything it did not.
            Ok(Err(error)) => {
                for path in error.paths {
                    settling.touch(path, Instant::now());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        if let Some(paths) = settling.take_if_settled(Instant::now()) {
            on_settled(paths);
        }
    }
}

fn note(settling: &mut Settling, event: &Event) {
    // Something being read changes nothing.
    if matches!(event.kind, EventKind::Access(_)) {
        return;
    }

    let now = Instant::now();
    for path in &event.paths {
        if media::might_matter(path) {
            settling.touch(path.clone(), now);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::PathBuf;
    use std::sync::{Mutex, PoisonError};
    use std::time::{Duration, Instant};

    use notify::Event;
    use notify::event::{AccessKind, CreateKind, EventKind};

    use super::{FolderWatcher, note};
    use crate::debounce::{PATIENCE, Settling};

    #[test]
    fn only_changes_to_films_and_subtitles_count() {
        let mut settling = Settling::new(Duration::from_millis(1), PATIENCE);
        let created = |path: &str| Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from(path)],
            attrs: notify::event::EventAttributes::default(),
        };

        note(&mut settling, &created("/films/Heat.nfo"));
        note(&mut settling, &created("/films/.DS_Store"));
        assert!(settling.take_if_settled(Instant::now()).is_none());

        note(&mut settling, &created("/films/Heat.mkv"));
        note(
            &mut settling,
            &Event {
                kind: EventKind::Access(AccessKind::Read),
                paths: vec![PathBuf::from("/films/Ronin.mkv")],
                attrs: notify::event::EventAttributes::default(),
            },
        );

        let settled = settling
            .take_if_settled(Instant::now() + Duration::from_secs(1))
            .unwrap_or_default();
        assert_eq!(settled, [PathBuf::from("/films/Heat.mkv")]);
    }

    #[test]
    fn a_file_appearing_in_a_watched_folder_is_reported_once() {
        let folder = tempfile::tempdir().unwrap();
        let seen: &'static Mutex<Vec<PathBuf>> = Box::leak(Box::new(Mutex::new(Vec::new())));

        let mut watcher = FolderWatcher::with_quiet(Duration::from_millis(80), |paths| {
            seen.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .extend(paths);
        })
        .unwrap();
        watcher.watch(folder.path()).unwrap();

        let film = folder.path().join("Heat.1995.mkv");
        std::fs::write(&film, b"not really a film").unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_empty()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let reported = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
        assert!(
            reported.contains(&film),
            "the new file should have been reported, got {reported:?}"
        );

        // Unwatching a folder that is no longer watched is not an error.
        watcher.unwatch(folder.path()).unwrap();
        watcher.unwatch(folder.path()).unwrap();
    }
}
