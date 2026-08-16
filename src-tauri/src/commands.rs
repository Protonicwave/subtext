//! What the front end is allowed to ask for.
//!
//! Every one of these is generated into TypeScript by `tauri-specta`, so the
//! front end cannot call something that is not here, or call it with the wrong
//! shape, without the compiler saying so.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use subtext_core::Timestamp;
use subtext_index::{Database, TrackChoice};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::chrome::Chrome;
use crate::dto::{
    AccentView, AlignProgressed, AlignmentView, Answer, CorrectionView, CueView, Failure, FilmView,
    FolderView, Id, PosterWanted, PreferenceView, ScanProgressed, TrackView,
};
use crate::state::AppState;
use crate::{allowed, dropped, posters, reveal};

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
    let folders = folder_paths(database)?;
    // One query for the sound of the whole library rather than one a film. Most
    // films carry a track or two, so asking film by film would be several
    // thousand statements to answer what one statement answers.
    let mut audio = database.details().all_audio().map_err(Failure::of)?;

    films
        .into_iter()
        .map(|film| {
            let tracks = database.tracks().for_film(film.id).map_err(Failure::of)?;
            let position = database.positions().get(film.id).map_err(Failure::of)?;
            let sound = audio.remove(&film.id).unwrap_or_default();
            let folder = folders.get(&film.folder_id).map(PathBuf::as_path);
            Ok(FilmView::of(film, folder, tracks, sound, position))
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
    let folders = folder_paths(database)?;

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
            let audio = database
                .details()
                .audio(resumable.film.id)
                .map_err(Failure::of)?;
            let folder = folders.get(&resumable.film.folder_id).map(PathBuf::as_path);
            Ok(FilmView::of(
                resumable.film,
                folder,
                tracks,
                audio,
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

/// Shows a film where it sits, in the platform's own file manager.
///
/// A film is named by its identifier rather than by its path, so the only files
/// this can be asked about are files the library already holds. The path is
/// then put through the same rule the protocols answer to, which is what makes
/// a row pointing somewhere it should not a refusal rather than a window onto
/// somebody's documents.
#[tauri::command]
#[specta::specta]
pub(crate) async fn show_in_folder(app: AppHandle, film_id: Id) -> Answer<()> {
    // Starting a process is short work, and it is still a process, which does
    // not belong on the thread answering commands.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let film = state
            .scanner()
            .database()
            .films()
            .by_id(film_id.get())
            .map_err(Failure::of)?
            .ok_or_else(|| Failure::saying("that film is no longer in the library"))?;

        // What the library recorded is what is shown, and where that resolves
        // to is what is judged. A file that has gone fails here, which is the
        // same answer as a file that was never allowed.
        app.state::<allowed::Roots>()
            .resolve(&film.path)
            .ok_or_else(|| Failure::saying("that film is not where the library left it"))?;

        reveal::in_file_manager(&film.path)
    })
    .await
    .map_err(|_| Failure::saying("the folder could not be opened"))?
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

/// The films with no poster drawn for them yet.
///
/// A poster is wanted when the film has none, when the file it names is not
/// there any more, when the film's own file has changed since it was drawn, and
/// when the cover it was drawn from is no longer where the cover comes from.
/// None of those needs anything stored to compare against: the name a poster is
/// filed under is derived from all three, so a film whose row names a different
/// file is a film whose poster is stale.
///
/// Each film also says whether there is an image to draw it from. Where there
/// is, the front end asks for those bytes; where there is not, it opens the
/// film and takes a frame, which is the expensive answer and the last one.
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
            let wanted = directory.join(posters::file_name(
                &film.path,
                film.modified_at,
                film.cover_path.as_deref(),
            ));
            film.poster_path.as_deref() != Some(wanted.as_path()) || !wanted.is_file()
        })
        .map(|film| PosterWanted {
            id: Id::of(film.id),
            path: film.path.display().to_string(),
            cover: film.cover_path.is_some(),
        })
        .collect())
}

/// The cover image a film's poster is to be drawn from.
///
/// One command whichever of the two sources it came from, so the front end has
/// one thing to do with an image and no idea where it was: the scan settled
/// that, and the row says so. What comes back is a picture as it sits on the
/// disk, tens of kilobytes of it, which the same worker crops, encodes and
/// takes the colours from as it does a frame.
#[tauri::command]
#[specta::specta]
pub(crate) async fn cover_image(app: AppHandle, film_id: Id) -> Answer<Vec<u8>> {
    // Reading a file, on a thread that is not the one answering commands.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let film = state
            .scanner()
            .database()
            .films()
            .by_id(film_id.get())
            .map_err(Failure::of)?
            .ok_or_else(|| Failure::saying("that film is no longer in the library"))?;

        let cover = film
            .cover_path
            .ok_or_else(|| Failure::saying("that film has no cover to read"))?;

        // The film's own path says the image is attached inside it, which is
        // what the scan wrote when it found one there.
        if cover == film.path {
            return subtext_container::cover_image(&cover)
                .map_err(Failure::of)?
                .ok_or_else(|| Failure::saying("that film no longer carries a cover"));
        }
        std::fs::read(&cover).map_err(Failure::of)
    })
    .await
    .map_err(|_| Failure::saying("the cover could not be read"))?
}

/// Records the poster drawn for a film, and what was learnt drawing it.
///
/// The encoding and the colours are the front end's work, because the frame is
/// already decoded there and sending four megabytes of pixels across to have
/// them squeezed here would cost more than it saved. What arrives is a WebP of
/// a few tens of kilobytes.
///
/// The running time comes with it where there is one: opening the file is the
/// only way to find out how long a film is, and a capture has just done that.
/// A poster drawn from a cover image says nothing about it, since nothing
/// opened the film, and the player fills it in the first time anybody watches.
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

        let poster = directory.join(posters::file_name(
            &film.path,
            film.modified_at,
            film.cover_path.as_deref(),
        ));
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
    let audio = database.details().audio(id).map_err(Failure::of)?;
    let position = database.positions().get(id).map_err(Failure::of)?;
    let folders = folder_paths(database)?;
    let folder = folders.get(&film.folder_id).map(PathBuf::as_path);

    Ok(FilmView::of(film, folder, tracks, audio, position))
}

/// Where each watched folder is, by its identifier.
///
/// Read once for a whole list of films rather than once a film. A library has a
/// handful of watched folders and thousands of films, and every one of those
/// films needs its folder to know which shelf it belongs on.
fn folder_paths(database: &Database) -> Answer<HashMap<i64, PathBuf>> {
    Ok(database
        .folders()
        .list()
        .map_err(Failure::of)?
        .into_iter()
        .map(|folder| (folder.id, folder.path))
        .collect())
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

/// Records how a subtitle track's timings line up with its film.
///
/// Written when somebody has settled on a value rather than while they are
/// arriving at one. Every intermediate step of a nudge would be this write and
/// a re-read of the whole track behind it, so the player shows the steps itself
/// and calls this once at the end.
///
/// The film comes back rather than the track alone, because a correction
/// changes what the cues of that film are and the library screen holds the
/// film. What comes back is read from the database, so a value outside the
/// bounds the core enforces returns as the value that was actually kept.
#[tauri::command]
#[specta::specta]
pub(crate) async fn set_track_correction(
    state: State<'_, AppState>,
    track_id: Id,
    correction: CorrectionView,
) -> Answer<FilmView> {
    let database = state.scanner().database();
    let track = database
        .tracks()
        .by_id(track_id.get())
        .map_err(Failure::of)?
        .ok_or_else(|| Failure::saying("that subtitle is no longer in the library"))?;

    database
        .tracks()
        .set_correction(track.id, correction.wanted())
        .map_err(Failure::of)?;

    read_back(database, track.film_id)
}

/// Works out how a subtitle track's timings line up with its film, by listening
/// to the film.
///
/// The number somebody would otherwise arrive at by ear, measured instead. It
/// is asked for rather than done on its own, because decoding the audio of
/// every film in a library would cost minutes of a machine that is doing
/// something else and would mostly correct files that are already right.
///
/// Real work on a thread of its own, since reading a two hour film takes
/// several seconds and the film is expected to keep playing throughout. What
/// comes back covers every ending, including the ones where nothing was
/// written.
#[tauri::command]
#[specta::specta]
pub(crate) async fn align_track(app: AppHandle, track_id: Id) -> Answer<AlignmentView> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>().aligning(&app, track_id.get())
    })
    .await
    .map_err(|_| Failure::saying("lining the subtitle up did not finish"))?
}

/// Stops the alignment that is running.
///
/// Leaves the track as it was. Nothing is written until a measurement has been
/// made and believed, and a reading that stops makes no measurement.
#[tauri::command]
#[specta::specta]
pub(crate) async fn cancel_alignment(state: State<'_, AppState>) -> Answer<()> {
    state.stop_aligning();
    Ok(())
}

/// Where the alignment that is running has got to, for a screen that has not
/// seen any events yet.
#[tauri::command]
#[specta::specta]
pub(crate) async fn alignment_progress(
    state: State<'_, AppState>,
) -> Answer<Option<AlignProgressed>> {
    Ok(state.latest_alignment())
}

/// Records which subtitle track a film is watched with.
///
/// A track to read it with, or nothing at all, which is a decision in its own
/// right and is why turning subtitles off is written down rather than simply
/// not choosing. The state a film starts in, where nobody has chosen, is not
/// something this can write: it is what the row already says, and going back to
/// it is not one of the answers the menu offers.
///
/// The film comes back rather than an acknowledgement, because the choice
/// changes which cues that film has and the library screen holds the film.
#[tauri::command]
#[specta::specta]
pub(crate) async fn set_film_track(
    state: State<'_, AppState>,
    film_id: Id,
    track_id: Option<Id>,
) -> Answer<FilmView> {
    let database = state.scanner().database();

    let choice = match track_id {
        Some(track_id) => {
            let track = database
                .tracks()
                .by_id(track_id.get())
                .map_err(Failure::of)?
                .ok_or_else(|| Failure::saying("that subtitle is no longer in the library"))?;

            // A track belongs to one film, and a film may only be watched with
            // its own. Left unchecked this would write a row that no rule and
            // no menu could ever explain.
            if track.film_id != film_id.get() {
                return Err(Failure::saying("that subtitle belongs to a different film"));
            }
            TrackChoice::Track(track.id)
        }
        None => TrackChoice::Off,
    };

    database
        .films()
        .set_choice(film_id.get(), choice)
        .map_err(Failure::of)?;

    read_back(database, film_id.get())
}

/// Every preference that has been set, by key.
///
/// The whole lot in one call rather than a call per control. There are a few
/// dozen of them, they are read once when the window opens, and the settings
/// screen is not the only thing that wants them: the player and the window
/// itself are both drawn from these before anybody has opened settings at
/// all.
#[tauri::command]
#[specta::specta]
pub(crate) async fn read_preferences(state: State<'_, AppState>) -> Answer<Vec<PreferenceView>> {
    let stored = state
        .scanner()
        .database()
        .preferences()
        .all()
        .map_err(Failure::of)?;

    Ok(stored
        .into_iter()
        .map(|(key, value)| PreferenceView { key, value })
        .collect())
}

/// Records one preference.
///
/// One row replaced, which is why this is not batched or thrown away on a
/// timer: a control that has been changed has been changed, and a window closed
/// a moment later should still open the way it was left.
#[tauri::command]
#[specta::specta]
pub(crate) async fn write_preference(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Answer<()> {
    state
        .scanner()
        .database()
        .preferences()
        .set(&key, &value)
        .map_err(Failure::of)
}

/// Forgets preferences the application no longer has anything to do with.
///
/// Which keys those are is the front end's to say, because the front end is
/// where the table of settings lives and duplicating it here would be a second
/// list to keep in step. This end only does as it is told, and a key that was
/// not there was already forgotten.
#[tauri::command]
#[specta::specta]
pub(crate) async fn forget_preferences(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Answer<()> {
    let preferences = state.scanner().database().preferences();
    for key in &keys {
        preferences.remove(key).map_err(Failure::of)?;
    }

    Ok(())
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
