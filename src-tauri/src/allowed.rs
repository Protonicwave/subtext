//! What the webview is allowed to read from disk.
//!
//! The video element that a frame is captured from, and the poster images the
//! grid draws, are loaded by the webview itself through Tauri's asset protocol.
//! That protocol refuses every path until something opens one, which is the
//! behaviour worth keeping: only the folders somebody asked Subtext to watch,
//! and the one directory the posters are cached in, are ever opened to it.

use std::path::Path;

use tauri::{AppHandle, Manager, Runtime};

use crate::dto::Failure;
use crate::posters;
use crate::state::AppState;

/// Opens a directory and everything under it to the webview.
///
/// Nothing is ever taken back. The scope treats a denial as final and weighs it
/// above any allowance, so forbidding a folder that had just been removed would
/// also forbid it if somebody added it again a minute later, which is the more
/// likely of the two mistakes. A removed folder stops being readable at the
/// next start, and until then no screen holds a path into it.
pub(crate) fn directory<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<(), Failure> {
    app.asset_protocol_scope()
        .allow_directory(path, true)
        .map_err(Failure::of)
}

/// Opens everything the library already knows about.
///
/// Called once at startup, after the watched folders have been read. A folder
/// added while the application is running is opened as it is added.
pub(crate) fn what_is_watched<R: Runtime>(app: &AppHandle<R>) -> Result<(), Failure> {
    directory(app, &posters::directory(app)?)?;

    let state = app.state::<AppState>();
    for folder in &state.scanner().folders().map_err(Failure::of)? {
        directory(app, &folder.path)?;
    }
    Ok(())
}
