//! Turning what was dropped on the window into folders to watch.
//!
//! The library is a view onto folders, not a list of imported files, so a drop
//! has to become folders before it means anything. A dropped folder is itself;
//! a dropped film is the folder it already lives in, which is also the folder
//! its subtitles live in.

use std::path::{Path, PathBuf};

/// The distinct folders a set of dropped paths belong to.
pub(crate) fn folders_of(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for path in paths {
        if let Some(folder) = folder_of(path)
            && !folders.contains(&folder)
        {
            folders.push(folder);
        }
    }

    // Dropping a folder together with one inside it would have the same film
    // found under two watched folders. A film belongs to one folder, so the two
    // would take it off each other on every scan for as long as both were
    // watched. The outer one covers the inner one anyway.
    folders
        .iter()
        .filter(|folder| !folders.iter().any(|other| is_within(folder, other)))
        .cloned()
        .collect()
}

/// The folder a dropped path stands for, where there is one.
///
/// A path that is gone by the time this runs, which a drop from an archive
/// tool can produce, has no folder worth watching and is stepped over.
fn folder_of(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    path.parent()
        .filter(|parent| parent.is_dir())
        .map(Path::to_path_buf)
}

/// Whether one folder sits inside another, which a folder does not do to
/// itself.
fn is_within(folder: &Path, other: &Path) -> bool {
    folder != other && folder.starts_with(other)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::PathBuf;

    use super::folders_of;

    /// A directory tree standing in for what someone might drag in.
    fn films() -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let films = directory.path().join("films");
        std::fs::create_dir_all(films.join("Nineteen Nineties")).unwrap();
        std::fs::write(films.join("Heat.1995.mkv"), b"").unwrap();
        std::fs::write(films.join("Heat.1995.srt"), b"").unwrap();
        std::fs::write(films.join("Nineteen Nineties/Ronin.1998.mkv"), b"").unwrap();
        (directory, films)
    }

    #[test]
    fn a_dropped_folder_is_the_folder_itself() {
        let (_directory, films) = films();

        assert_eq!(folders_of(std::slice::from_ref(&films)), vec![films]);
    }

    #[test]
    fn dropped_files_become_the_folder_they_live_in() {
        let (_directory, films) = films();

        let folders = folders_of(&[films.join("Heat.1995.mkv"), films.join("Heat.1995.srt")]);

        assert_eq!(folders, vec![films]);
    }

    #[test]
    fn a_folder_dropped_with_its_own_parent_is_covered_by_the_parent() {
        let (_directory, films) = films();

        let folders = folders_of(&[films.join("Nineteen Nineties"), films.clone()]);

        assert_eq!(folders, vec![films]);
    }

    #[test]
    fn separate_folders_are_all_kept() {
        let (directory, films) = films();
        let archive = directory.path().join("archive");
        std::fs::create_dir_all(&archive).unwrap();

        let folders = folders_of(&[films.join("Heat.1995.mkv"), archive.clone()]);

        assert_eq!(folders, vec![films, archive]);
    }

    #[test]
    fn something_that_is_no_longer_there_is_stepped_over() {
        let folders = folders_of(&[PathBuf::from("/nowhere/at/all/Heat.1995.mkv")]);

        assert!(folders.is_empty());
    }
}
