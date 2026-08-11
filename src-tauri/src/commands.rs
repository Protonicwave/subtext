//! What the front end is allowed to ask for.
//!
//! Every one of these is generated into TypeScript by `tauri-specta`, so the
//! front end cannot call something that is not here, or call it with the wrong
//! shape, without the compiler saying so.

use std::path::{Path, PathBuf};

use subtext_core::Timestamp;
use subtext_index::{Database, SearchOptions};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::chrome::Chrome;
use crate::dto::{
    AccentView, Answer, CueView, Failure, FilmView, FolderView, Id, PosterWanted, ScanProgressed,
    SearchView, TrackView,
};
use crate::state::AppState;
use crate::{allowed, dropped, posters};

/// What the window the front end is drawing into turned out to be.
///
/// Asked once at startup. The window has already been dressed by then, so this
/// reports what happened rather than causing it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn window_chrome(chrome: State<'_, Chrome>) -> Answer<Chrome> {
    Ok(*chrome)
}

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

/// Opens the platform's own file picker, filtered to subtitle files.
///
/// What the attach action in the import sheet opens, for a film whose subtitle
/// was not found beside it.
#[tauri::command]
#[specta::specta]
pub(crate) async fn choose_subtitle(app: AppHandle) -> Answer<Option<String>> {
    let chosen = app
        .dialog()
        .file()
        .add_filter("Subtitle files", &["srt"])
        .blocking_pick_file();
    Ok(chosen.map(|file| file.to_string()))
}

/// The folders that a set of dropped paths stand for.
///
/// Dropping a film means the folder it is in, since that is where its subtitles
/// are and where the next film will be put. The front end adds what comes back
/// the same way it adds a folder that was picked.
#[tauri::command]
#[specta::specta]
pub(crate) async fn folders_for_paths(paths: Vec<String>) -> Answer<Vec<String>> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    Ok(dropped::folders_of(&paths)
        .iter()
        .map(|folder| folder.display().to_string())
        .collect())
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
    // Before anything is read, so that the first film to appear can already be
    // opened by the webview for its frame to be taken.
    allowed::folder(&app, &folder.path)?;
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

/// The films to carry on with, most recently watched first.
///
/// A separate command rather than a filter over the library, because the order
/// is by when each was last watched and the row shows a handful rather than
/// everything. Films whose files have gone are included and marked missing,
/// since an unplugged drive is exactly the case positions outlive their files
/// for.
#[tauri::command]
#[specta::specta]
pub(crate) async fn continue_watching(
    state: State<'_, AppState>,
    limit: u32,
) -> Answer<Vec<FilmView>> {
    let database = state.scanner().database();
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);

    database
        .positions()
        .resumable(limit)
        .map_err(Failure::of)?
        .into_iter()
        .map(|resumable| {
            let tracks = database
                .tracks()
                .for_film(resumable.film.id)
                .map_err(Failure::of)?;
            Ok(FilmView::of(
                resumable.film,
                tracks,
                Some(resumable.position),
            ))
        })
        .collect()
}

/// Every line of dialogue in one subtitle track, in playback order.
///
/// The whole track at once rather than a window around the current moment. A
/// five thousand cue film is a few hundred kilobytes, it is read once when the
/// player opens, and having all of it in memory is what lets the active line be
/// found by a binary search on every frame instead of by asking across the
/// boundary sixty times a second.
#[tauri::command]
#[specta::specta]
pub(crate) async fn track_cues(state: State<'_, AppState>, track_id: Id) -> Answer<Vec<CueView>> {
    let cues = state
        .scanner()
        .database()
        .tracks()
        .cues(track_id.get())
        .map_err(Failure::of)?;

    Ok(cues.into_iter().map(CueView::of).collect())
}

/// How many lines one search asks the index for.
///
/// More than the palette can show at once, because the count under the field
/// says how many were found and the list is scrolled. Far fewer than the index
/// would happily return, because nobody reads the four hundredth match.
const SEARCH_LIMIT: usize = 100;

/// How many lines any one film contributes to a search across the library.
///
/// Small on purpose. A search for a common word would otherwise come back as
/// one film said a hundred times, and the useful answer to "who says this" is
/// which films say it.
const PER_FILM: usize = 4;

/// Finds a line of dialogue anywhere in the library, or in one film.
///
/// Called on a keystroke, so the work happens on a thread of its own rather
/// than on the runtime that answers every other command. What comes back is
/// grouped by film and ranked, and the query itself is treated as words rather
/// than as index syntax, so nothing anybody types is an error.
#[tauri::command]
#[specta::specta]
pub(crate) async fn search_dialogue(
    app: AppHandle,
    query: String,
    film_id: Option<Id>,
) -> Answer<SearchView> {
    tauri::async_runtime::spawn_blocking(move || {
        let film = film_id.map(Id::get);
        let options = SearchOptions {
            film,
            limit: SEARCH_LIMIT,
            // Within one film every match is worth showing: the list is that
            // film's own lines in order, and holding some back would be
            // answering half the question.
            per_film: if film.is_some() {
                SEARCH_LIMIT
            } else {
                PER_FILM
            },
            ..SearchOptions::default()
        };

        app.state::<AppState>()
            .scanner()
            .database()
            .search()
            .find(&query, &options)
            .map(|results| SearchView::of(&results))
            .map_err(Failure::of)
    })
    .await
    .map_err(|_| Failure::saying("the search did not finish"))?
}

/// Records how far through a film somebody is.
///
/// Called on a throttle while playing and once on the way out, so it is one row
/// replaced and nothing else. The running time comes with it because the player
/// is the thing that knows it: a film whose poster was never captured has no
/// duration stored, and without one the library cannot draw how far through it
/// is.
#[tauri::command]
#[specta::specta]
pub(crate) async fn save_position(
    state: State<'_, AppState>,
    film_id: Id,
    position_ms: u32,
    duration_ms: Option<u32>,
    finished: bool,
) -> Answer<()> {
    state
        .scanner()
        .database()
        .positions()
        .save(
            film_id.get(),
            Timestamp::from_millis(position_ms),
            duration_ms.map(Timestamp::from_millis),
            finished,
        )
        .map_err(Failure::of)
}

/// The films with no frame captured from them yet.
///
/// A poster is wanted when the film has none, when the file it names is not
/// there any more, and when the film's own file has changed since the frame was
/// taken. The last of those needs nothing stored to compare against: the name a
/// poster is filed under is derived from the film's path and modification time,
/// so a film whose row names a different file is a film whose frame is stale.
#[tauri::command]
#[specta::specta]
pub(crate) async fn posters_wanted(app: AppHandle) -> Answer<Vec<PosterWanted>> {
    let directory = posters::directory(&app)?;
    let state = app.state::<AppState>();
    let films = state
        .scanner()
        .database()
        .films()
        .list()
        .map_err(Failure::of)?;

    Ok(films
        .into_iter()
        // A file that is not there cannot be opened, and the library draws
        // those as missing rather than waiting for a frame that never comes.
        .filter(|film| !film.is_missing())
        .filter(|film| {
            let wanted = directory.join(posters::file_name(&film.path, film.modified_at));
            film.poster_path.as_deref() != Some(wanted.as_path()) || !wanted.is_file()
        })
        .map(|film| PosterWanted {
            id: Id::of(film.id),
            path: film.path.display().to_string(),
        })
        .collect())
}

/// Records the frame captured from a film, and what was learnt taking it.
///
/// The encoding and the colours are the front end's work, because the frame is
/// already decoded there and sending four megabytes of pixels across to have
/// them squeezed here would cost more than it saved. What arrives is a WebP of
/// a few tens of kilobytes.
///
/// The running time comes with it: opening the file is the only way to find out
/// how long a film is, and the capture has just done that. Without it a partly
/// watched film has a position and no fraction to draw it as.
#[tauri::command]
#[specta::specta]
pub(crate) async fn save_poster(
    app: AppHandle,
    film_id: Id,
    image: Vec<u8>,
    accent: Option<AccentView>,
    duration_ms: Option<u32>,
) -> Answer<FilmView> {
    let accent = accent.map(|accent| accent.stored()).transpose()?;

    // Writing tens of kilobytes and two rows is short work, but it is still a
    // file and a database, and neither belongs on the thread answering commands.
    tauri::async_runtime::spawn_blocking(move || {
        let directory = posters::directory(&app)?;
        let state = app.state::<AppState>();
        let database = state.scanner().database();

        let film = database
            .films()
            .by_id(film_id.get())
            .map_err(Failure::of)?
            .ok_or_else(|| Failure::saying("that film is no longer in the library"))?;

        let poster = directory.join(posters::file_name(&film.path, film.modified_at));
        std::fs::write(&poster, &image).map_err(Failure::of)?;

        // The frame taken from the file this film used to be is of no use to
        // anybody, and leaving it would grow the cache by one file per change.
        if let Some(stale) = film.poster_path.as_deref().filter(|old| *old != poster) {
            let _ = std::fs::remove_file(stale);
        }

        database
            .films()
            .set_poster(film.id, &poster, accent.as_deref())
            .map_err(Failure::of)?;

        if let Some(duration) = duration_ms {
            database
                .films()
                .set_duration(film.id, Timestamp::from_millis(duration))
                .map_err(Failure::of)?;
        }

        // Read back rather than assembled here, so the tile that redraws shows
        // what the library holds and not what this function hoped it would.
        read_back(database, film.id)
    })
    .await
    .map_err(|_| Failure::saying("the poster could not be written"))?
}

/// One film as the library screen sees it, straight from the database.
fn read_back(database: &Database, id: i64) -> Answer<FilmView> {
    let film = database
        .films()
        .by_id(id)
        .map_err(Failure::of)?
        .ok_or_else(|| Failure::saying("that film is no longer in the library"))?;
    let tracks = database.tracks().for_film(id).map_err(Failure::of)?;
    let position = database.positions().get(id).map_err(Failure::of)?;

    Ok(FilmView::of(film, tracks, position))
}

/// Gives a subtitle file to a film because somebody said so.
///
/// The amber row in the import sheet. Reading and parsing the file is real work
/// on a real thread rather than on the one answering commands, and it waits
/// behind any scan that is running, so it must not hold up the runtime while it
/// does.
#[tauri::command]
#[specta::specta]
pub(crate) async fn attach_subtitle(
    app: AppHandle,
    film_id: Id,
    path: String,
) -> Answer<TrackView> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let attached = state
            .scanner()
            .attach_subtitle(film_id.get(), Path::new(&path))
            .map_err(Failure::of)?;

        // Read back rather than assembled here, so that the row the sheet
        // redraws says what the library says and not what we hoped it would.
        state
            .scanner()
            .database()
            .tracks()
            .for_film(attached.film_id)
            .map_err(Failure::of)?
            .into_iter()
            .find(|track| track.id == attached.track_id)
            .map(TrackView::of)
            .ok_or_else(|| Failure::saying("the subtitle was attached but could not be read back"))
    })
    .await
    .map_err(|_| Failure::saying("attaching the subtitle did not finish"))?
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
