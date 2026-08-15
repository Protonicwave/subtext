//! Showing somebody where a film sits, in the file manager they already use.
//!
//! Three platforms with three ideas of what that means, and none of them worth
//! a dependency: each is a process, a switch and a path. The alternative was a
//! plugin that brings a bus client, an async runtime and a dozen crates behind
//! them to press one button.
//!
//! Nothing here decides which file may be shown. That is settled before this is
//! called, against the same watched folders the protocols answer to.

use std::path::Path;
use std::process::Command;

use crate::dto::Failure;

/// Opens the platform's file manager with the film picked out.
///
/// Best effort by nature. A machine with no file manager, or one this does not
/// know how to ask, is a machine where nothing happens, and the sentence that
/// comes back says so rather than pretending a window opened.
pub(crate) fn in_file_manager(path: &Path) -> Result<(), Failure> {
    show(path).map_err(|_| Failure::saying("the folder could not be opened"))
}

#[cfg(target_os = "windows")]
fn show(path: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    // Written out as one raw argument rather than passed as two. Explorer does
    // not parse its command line the way everything else does: the file has to
    // be quoted and the switch must not be, which is not an arrangement the
    // usual escaping can be talked into producing.
    let mut explorer = Command::new("explorer");
    explorer.raw_arg(format!("/select,\"{}\"", path.display()));

    // Started rather than waited for. Explorer exits with a failure whenever it
    // is given a switch, whether or not it did what was asked, so its status is
    // not something to report on.
    explorer.spawn().map(drop)
}

#[cfg(target_os = "macos")]
fn show(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg("-R").arg(path).spawn().map(drop)
}

/// Everywhere else, which in practice is a desktop speaking the freedesktop
/// conventions.
///
/// The bus is asked first, since it is the one way to have the file itself
/// picked out rather than the folder merely opened. A desktop with nothing
/// listening gets the folder, which is most of the answer and is better than a
/// button that does nothing.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn show(path: &Path) -> std::io::Result<()> {
    let asked = Command::new("dbus-send")
        .args([
            "--session",
            "--type=method_call",
            "--dest=org.freedesktop.FileManager1",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{}", file_uri(path)))
        .arg("string:")
        .status();

    if matches!(asked, Ok(status) if status.success()) {
        return Ok(());
    }

    let folder = path.parent().unwrap_or(path);
    Command::new("xdg-open").arg(folder).spawn().map(drop)
}

/// A path as a URI, which is what the bus expects to be handed.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn file_uri(path: &Path) -> String {
    use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

    // Everything that would be read as syntax rather than as part of a name.
    // The separator is left alone, since a path is what this is describing.
    const AWKWARD: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'#')
        .add(b'%')
        .add(b'<')
        .add(b'>')
        .add(b'?')
        .add(b'[')
        .add(b'\\')
        .add(b']')
        .add(b'^')
        .add(b'`')
        .add(b'{')
        .add(b'|')
        .add(b'}');

    format!(
        "file://{}",
        utf8_percent_encode(&path.display().to_string(), AWKWARD)
    )
}

#[cfg(all(test, not(any(target_os = "windows", target_os = "macos"))))]
mod tests {
    use super::file_uri;
    use std::path::Path;

    #[test]
    fn a_name_with_awkward_characters_in_it_survives_being_a_uri() {
        assert_eq!(
            file_uri(Path::new("/films/Am\u{e9}lie (2001) #1.mkv")),
            "file:///films/Am%C3%A9lie%20(2001)%20%231.mkv"
        );
    }
}
