//! What the front end is allowed to ask for.
//!
//! Every one of these is generated into TypeScript by `tauri-specta`, so the
//! front end cannot call something that is not here, or call it with the wrong
//! shape, without the compiler saying so.

use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::dto::{Answer, Failure, FilmView, FolderView, Id, ScanProgressed};
use crate::state::AppState;

/// Opens the platform's own folder picker.
///
/// Separate from adding the folder, so that a front end that already knows the
/// path, from a drag and drop for instance, does not have to open a dialog to
/// use it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn choose_folder(app: AppHandle) -> Answer<Option<String>> {
    // Not on the main thread: the platform dialogs block until they are
    // answered, and the main thread is where the window is drawn.
    let chosen = app.dialog().file().blocking_pick_folder();
    Ok(chosen.map(|folder| folder.to_string()))
}

/// Starts watching a folder and reads what is in it.
///
/// Returns as soon as the folder has been recorded. Reading it happens behind
/// this and reports itself through the scan events, because a folder of a
/// thousand films takes a few seconds and the folder should appear in the list
/// straight away.
#[tauri::command]
#[specta::specta]
pub(crate) async fn add_folder(app: AppHandle, path: String) -> Answer<FolderView> {
    let state = app.state::<AppState>();
    let folder = state
        .scanner()
        .add_folder(Path::new(&path))
        .map_err(Failure::of)?;

    state.watch(&folder.path)?;
    let view = FolderView::of(&folder, 0, state.is_watching());

    let handle = app.clone();
    let scanning = folder.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        state.scanning(&handle, |scanner, sink| {
            scanner.scan(&scanning, sink).map(|outcome| vec![outcome])
        });
    });

    Ok(view)
}

/// Stops watching a folder and forgets what was found in it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn remove_folder(app: AppHandle, id: Id) -> Answer<bool> {
    let state = app.state::<AppState>();
    let folder = state
        .scanner()
        .folders()
        .map_err(Failure::of)?
        .into_iter()
        .find(|folder| folder.id == id.get());

    let Some(folder) = folder else {
        return Ok(false);
    };

    state.unwatch(&folder.path)?;
    state.scanner().remove_folder(id.get()).map_err(Failure::of)
}

/// The folders being watched, with how many films each holds.
#[tauri::command]
#[specta::specta]
pub(crate) async fn list_folders(state: State<'_, AppState>) -> Answer<Vec<FolderView>> {
    let database = state.scanner().database();
    let watching = state.is_watching();

    database
        .folders()
        .list()
        .map_err(Failure::of)?
        .iter()
        .map(|folder| {
            let films = database.films().in_folder(folder.id).map_err(Failure::of)?;
            Ok(FolderView::of(folder, films.len(), watching))
        })
        .collect()
}

/// Every film in the library, with its subtitle tracks and where it was left.
#[tauri::command]
#[specta::specta]
pub(crate) async fn list_library(state: State<'_, AppState>) -> Answer<Vec<FilmView>> {
    let database = state.scanner().database();
    let films = database.films().list().map_err(Failure::of)?;

    films
        .into_iter()
        .map(|film| {
            let tracks = database.tracks().for_film(film.id).map_err(Failure::of)?;
            let position = database.positions().get(film.id).map_err(Failure::of)?;
            Ok(FilmView::of(film, tracks, position))
        })
        .collect()
}

/// Reads every watched folder again.
///
/// Cheap when nothing has moved: an unchanged folder is one stat per file and
/// no parsing at all. Returns straight away, like adding a folder does.
#[tauri::command]
#[specta::specta]
pub(crate) async fn rescan(app: AppHandle) -> Answer<()> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        state.scanning(&handle, subtext_scan::Scanner::scan_all);
    });
    Ok(())
}

/// Where the scan that is running has got to, for a window that opened part way
/// through one and has not seen any events yet.
#[tauri::command]
#[specta::specta]
pub(crate) async fn scan_progress(state: State<'_, AppState>) -> Answer<Option<ScanProgressed>> {
    Ok(state.latest_progress())
}

/// Whether a scan is running at all.
#[tauri::command]
#[specta::specta]
pub(crate) async fn is_scanning(state: State<'_, AppState>) -> Answer<bool> {
    Ok(state.is_scanning())
}
