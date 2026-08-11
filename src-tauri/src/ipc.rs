//! The boundary between Rust and the front end, described once.
//!
//! The commands and the events are collected here rather than in `main`, so
//! that the same list is what the application serves and what the generated
//! TypeScript is written from. There is no second place for them to disagree.

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands;
use crate::dto::{ScanFailed, ScanFinished, ScanProgressed};

/// Where the generated TypeScript is written.
///
/// Worked out at compile time rather than from the working directory, since a
/// development build can be started from either the project root or this crate
/// and a relative path would land somewhere different each time.
///
/// It is committed. The front end has to type check without anybody having
/// built the Rust first, and a checked-in file is also something a reviewer can
/// read.
pub(crate) const BINDINGS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../src/shared/ipc/bindings.ts");

/// Everything the front end may call, and everything it may listen for.
pub(crate) fn bindings() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::window_chrome,
            commands::choose_folder,
            commands::choose_subtitle,
            commands::folders_for_paths,
            commands::add_folder,
            commands::attach_subtitle,
            commands::remove_folder,
            commands::list_folders,
            commands::list_library,
            commands::continue_watching,
            commands::posters_wanted,
            commands::save_poster,
            commands::rescan,
            commands::scan_progress,
            commands::is_scanning,
        ])
        .events(collect_events![ScanProgressed, ScanFinished, ScanFailed])
}

pub(crate) fn typescript() -> Typescript {
    Typescript::default()
}

#[cfg(test)]
mod tests {
    // A test that cannot write to a directory of its own has nothing to
    // compare, so it stops rather than passing quietly.
    #![allow(clippy::expect_used)]

    use super::{BINDINGS, bindings, typescript};

    /// The committed TypeScript has to describe the commands as they are now.
    ///
    /// Set `SUBTEXT_UPDATE_BINDINGS` to write it out again after changing a
    /// command signature.
    #[test]
    fn the_generated_typescript_matches_the_commands() {
        let bindings = bindings();

        if std::env::var_os("SUBTEXT_UPDATE_BINDINGS").is_some() {
            bindings
                .export(typescript(), BINDINGS)
                .expect("the bindings should be writable");
            return;
        }

        let directory = tempfile::tempdir().expect("a temporary directory");
        let fresh = directory.path().join("bindings.ts");
        bindings
            .export(typescript(), &fresh)
            .expect("the bindings should be writable");

        let fresh = std::fs::read_to_string(&fresh).expect("the freshly written bindings");
        let committed = std::fs::read_to_string(BINDINGS).unwrap_or_default();

        assert_eq!(
            normalise(&committed),
            normalise(&fresh),
            "the committed TypeScript bindings no longer describe the commands. \
             Set SUBTEXT_UPDATE_BINDINGS and run this test again to write them."
        );
    }

    /// Line endings differ between what the exporter writes and what a checkout
    /// on Windows produces, and they are not what this test is about.
    fn normalise(source: &str) -> String {
        source.replace("\r\n", "\n")
    }
}
