//! What the webview is allowed to read from disk.
//!
//! Two protocols, one rule. Poster images and the frames captured from a film
//! are loaded through Tauri's asset protocol, and films themselves are served
//! through the scheme in [`crate::stream`]. Both refuse every path until
//! something opens one, which is the behaviour worth keeping: only the folders
//! somebody asked Subtext to watch, and the one directory the posters are
//! cached in, are ever opened to them.

use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

use tauri::{AppHandle, Manager, Runtime};

use crate::dto::Failure;
use crate::posters;
use crate::state::AppState;

/// The folders a film may be served out of.
///
/// Held apart from the asset protocol's own scope because that scope answers
/// only for itself, and the stream protocol has to make the same decision about
/// a path it has just been handed. Kept as a list of canonical directories: a
/// request is answered only if the file it names, once resolved, is inside one
/// of them, which is what makes a path full of `..` uninteresting.
#[derive(Debug, Default)]
pub(crate) struct Roots {
    open: RwLock<Vec<PathBuf>>,
}

impl Roots {
    /// Opens a folder and everything under it.
    fn open(&self, path: &Path) {
        // A folder that cannot be resolved is a folder that is not there, and
        // nothing inside it will resolve either, so there is nothing to record.
        let Ok(root) = std::fs::canonicalize(path) else {
            return;
        };

        let mut open = self.open.write().unwrap_or_else(PoisonError::into_inner);
        if !open.contains(&root) {
            open.push(root);
        }
    }

    /// The file behind a request, or nothing if it is not in a watched folder.
    ///
    /// Resolving comes first and comparing second. A path is only ever compared
    /// after both sides have been through the filesystem, so a symbolic link
    /// pointing out of a watched folder is judged by where it lands.
    pub(crate) fn resolve(&self, asked: &Path) -> Option<PathBuf> {
        let path = std::fs::canonicalize(asked).ok()?;
        if !path.is_file() {
            return None;
        }

        let open = self.open.read().unwrap_or_else(PoisonError::into_inner);
        open.iter()
            .any(|root| path.starts_with(root))
            .then_some(path)
    }
}

/// Opens a watched folder to both protocols.
///
/// Nothing is ever taken back. The asset scope treats a denial as final and
/// weighs it above any allowance, so forbidding a folder that had just been
/// removed would also forbid it if somebody added it again a minute later,
/// which is the more likely of the two mistakes. A removed folder stops being
/// readable at the next start, and until then no screen holds a path into it.
pub(crate) fn folder<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<(), Failure> {
    app.state::<Roots>().open(path);
    asset_scope(app, path)
}

/// Opens the poster cache to the asset protocol, and to nothing else.
///
/// It holds images we wrote, not films, so there is no reason for the stream
/// protocol to serve anything out of it.
fn cache<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<(), Failure> {
    asset_scope(app, path)
}

fn asset_scope<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<(), Failure> {
    app.asset_protocol_scope()
        .allow_directory(path, true)
        .map_err(Failure::of)
}

/// Opens everything the library already knows about.
///
/// Called once at startup, after the watched folders have been read. A folder
/// added while the application is running is opened as it is added.
pub(crate) fn what_is_watched<R: Runtime>(app: &AppHandle<R>) -> Result<(), Failure> {
    cache(app, &posters::directory(app)?)?;

    let state = app.state::<AppState>();
    for watched in &state.scanner().folders().map_err(Failure::of)? {
        folder(app, &watched.path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // A test that cannot make the directory it is about to serve out of has
    // nothing to check, so it stops rather than passing quietly.
    #![allow(clippy::expect_used)]

    use super::Roots;

    #[test]
    fn only_what_is_inside_an_open_folder_is_served() {
        let watched = tempfile::tempdir().expect("a folder to watch");
        let elsewhere = tempfile::tempdir().expect("a folder nobody asked for");

        let film = watched.path().join("Heat.1995.mkv");
        std::fs::write(&film, b"frames").expect("a film to serve");
        let private = elsewhere.path().join("accounts.mkv");
        std::fs::write(&private, b"not a film").expect("a file to refuse");

        let roots = Roots::default();
        assert!(roots.resolve(&film).is_none(), "nothing is open yet");

        roots.open(watched.path());
        assert!(roots.resolve(&film).is_some());
        assert!(roots.resolve(&private).is_none());
    }

    #[test]
    fn a_path_that_climbs_out_of_a_watched_folder_is_refused() {
        let watched = tempfile::tempdir().expect("a folder to watch");
        let inside = watched.path().join("films");
        std::fs::create_dir(&inside).expect("a folder inside it");
        let private = watched.path().join("accounts.mkv");
        std::fs::write(&private, b"not a film").expect("a file to refuse");

        let roots = Roots::default();
        roots.open(&inside);

        // The file exists and the path is spellable from inside the watched
        // folder, and it is still outside the folder that was opened.
        assert!(
            roots
                .resolve(&inside.join("..").join("accounts.mkv"))
                .is_none()
        );
    }

    #[test]
    fn a_folder_is_not_a_file_to_serve() {
        let watched = tempfile::tempdir().expect("a folder to watch");
        let roots = Roots::default();
        roots.open(watched.path());

        assert!(roots.resolve(watched.path()).is_none());
        assert!(roots.resolve(&watched.path().join("nothing.mkv")).is_none());
    }
}
