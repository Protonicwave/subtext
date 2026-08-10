// Without this the release build on Windows opens a console window behind the
// application. Debug builds keep it, because that is where log output goes.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = tauri::Builder::default().run(tauri::generate_context!()) {
        eprintln!("Subtext could not start: {error}");
        std::process::exit(1);
    }
}
