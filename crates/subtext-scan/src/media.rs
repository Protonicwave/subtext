//! Which files are worth looking at, and which folders are not.

use std::path::Path;

/// Container extensions the player will try to open.
///
/// Containers rather than codecs, because the webview decides what it can
/// decode and Subtext says so plainly when it cannot. Listing a container here
/// is a claim that the file is a film, not a promise that it will play.
const FILM_EXTENSIONS: &[&str] = &["mp4", "m4v", "mkv", "mov", "webm", "avi"];

/// Only SRT, because that is what the parser reads.
const SUBTITLE_EXTENSIONS: &[&str] = &["srt"];

/// Picture formats a cover beside a film may be in, which is what the webview
/// will draw.
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Folders that hold no films and cost real time to walk.
///
/// Recycle bins are the important ones: a deleted film still sits in there,
/// and indexing it would put a film in the library that the person has already
/// thrown away.
const SKIPPED_DIRECTORIES: &[&str] = &[
    "$recycle.bin",
    "system volume information",
    "recycler",
    "#recycle",
    "@eadir",
    "lost+found",
];

/// What a file turned out to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileKind {
    Film,
    Subtitle,
    /// A picture, which may turn out to be the cover somebody put beside a film.
    Image,
}

/// Whether a file is one of the two kinds a scan cares about.
pub(crate) fn classify(file_name: &str) -> Option<FileKind> {
    // Files beginning with a full stop are the operating system's business.
    // On macOS every film copied to a non-native filesystem is shadowed by an
    // AppleDouble file with the same name and a leading `._`, which would
    // otherwise be indexed as a film of its own.
    if file_name.starts_with('.') {
        return None;
    }

    let extension = file_name.rsplit_once('.')?.1;
    if FILM_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(extension))
    {
        return Some(FileKind::Film);
    }
    if SUBTITLE_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(extension))
    {
        return Some(FileKind::Subtitle);
    }
    if IMAGE_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(extension))
    {
        return Some(FileKind::Image);
    }
    None
}

/// Whether a directory is worth descending into.
pub(crate) fn is_worth_walking(name: &str) -> bool {
    !name.starts_with('.')
        && !SKIPPED_DIRECTORIES
            .iter()
            .any(|skipped| skipped.eq_ignore_ascii_case(name))
}

/// Whether a path might hold something a scan would want to read.
///
/// The watcher asks this of every event it is handed, and it has to answer for
/// paths that no longer exist, since a deletion is exactly the case where the
/// file cannot be inspected. So the answer comes from the name alone, and it
/// errs towards yes: a path with no extension could be a folder full of films,
/// and the cost of being wrong is a rescan that finds nothing to do.
pub(crate) fn might_matter(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if name.starts_with('.') {
        return false;
    }
    classify(name).is_some() || !name.contains('.')
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{FileKind, classify, is_worth_walking, might_matter};

    #[test]
    fn recognises_films_and_subtitles() {
        assert_eq!(classify("Heat.1995.mkv"), Some(FileKind::Film));
        assert_eq!(classify("Heat.1995.MP4"), Some(FileKind::Film));
        assert_eq!(classify("Heat.1995.en.srt"), Some(FileKind::Subtitle));
        assert_eq!(classify("Heat.1995.nfo"), None);
        assert_eq!(classify("Heat"), None);
    }

    #[test]
    fn recognises_the_pictures_a_cover_could_be() {
        assert_eq!(classify("poster.jpg"), Some(FileKind::Image));
        assert_eq!(classify("Heat.1995.PNG"), Some(FileKind::Image));
        assert_eq!(classify("cover.webp"), Some(FileKind::Image));
        assert_eq!(classify("fanart.bmp"), None);
    }

    #[test]
    fn leaves_the_operating_systems_own_files_alone() {
        assert_eq!(classify("._Heat.1995.mkv"), None);
        assert_eq!(classify(".DS_Store"), None);
    }

    #[test]
    fn does_not_walk_into_a_recycle_bin() {
        assert!(is_worth_walking("Films"));
        assert!(is_worth_walking("Nineteen Nineties"));
        assert!(!is_worth_walking("$RECYCLE.BIN"));
        assert!(!is_worth_walking("System Volume Information"));
        assert!(!is_worth_walking(".git"));
    }

    #[test]
    fn a_watched_path_that_might_be_a_folder_counts() {
        assert!(might_matter(Path::new("/films/Heat.1995.mkv")));
        assert!(might_matter(Path::new("/films/Heat.1995.srt")));
        // A folder being renamed or deleted, which cannot be inspected.
        assert!(might_matter(Path::new("/films/Nineteen Nineties")));

        assert!(!might_matter(Path::new("/films/Heat.1995.nfo")));
        assert!(!might_matter(Path::new("/films/.DS_Store")));
    }
}
