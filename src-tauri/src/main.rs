// Without this the release build on Windows opens a console window behind the
// application. Debug builds keep it, because that is where log output goes.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod align;
mod allowed;
mod chrome;
mod commands;
mod dropped;
mod dto;
mod ipc;
mod posters;
mod settings;
mod state;
mod stream;

use subtext_index::Database;
use tauri::Manager;

use crate::allowed::Roots;
use crate::state::{AppState, library_path};

fn main() {
    let bindings = ipc::bindings();

    // Written again on every development build, so that a command signature and
    // the TypeScript describing it cannot drift apart while somebody is working
    // on both. A release build has nothing to generate them from.
    #[cfg(debug_assertions)]
    if let Err(error) = bindings.export(ipc::typescript(), ipc::BINDINGS) {
        eprintln!("the TypeScript bindings could not be written: {error}");
    }

    let started = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Managed here rather than in the setup below, because a request for a
        // film can arrive as soon as the webview exists and the answer to it is
        // held in this.
        .manage(Roots::default())
        .register_asynchronous_uri_scheme_protocol(stream::SCHEME, stream::handle)
        .invoke_handler(bindings.invoke_handler())
        .setup(move |app| {
            bindings.mount_events(app);

            let handle = app.handle().clone();
            let path = library_path(&handle).map_err(|failure| failure.message)?;
            let state = AppState::open(Database::open(path)?);

            if let Err(error) = state.start_watching(&handle) {
                eprintln!(
                    "Subtext will not notice changes to folders: {}",
                    error.message
                );
            }

            // Before the window, which cannot be built until the preference
            // deciding how its webview decodes video has been read, and which
            // asks for the library as soon as it has been.
            let hardware = settings::hardware_decoding(state.scanner().database());
            app.manage(state);

            chrome::open(&handle, hardware).map_err(|failure| failure.message)?;
            app.manage(chrome::dress(&handle));

            // The webview reads films and posters itself, and may read nothing
            // until it is told what. Done after the state is managed, since what
            // is readable is exactly what the library says is being watched.
            if let Err(error) = allowed::what_is_watched(&handle) {
                eprintln!(
                    "Subtext will not be able to show posters: {}",
                    error.message
                );
            }

            // Folders change while the application is closed, so what is on
            // disk is caught up with as soon as there is a window to report it
            // to. Cheap when nothing has moved.
            let rescanning = handle.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state = rescanning.state::<AppState>();
                state.scanning(&rescanning, subtext_scan::Scanner::scan_all);
            });

            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(error) = started {
        eprintln!("Subtext could not start: {error}");
        std::process::exit(1);
    }
}
