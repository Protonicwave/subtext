//! Which shelf a film sits on.
//!
//! People already sorted their films, and the sort they chose means something to
//! them. Reading it costs a path split, and it needs no tagging interface and no
//! genre list that would be wrong for half a library.
//!
//! One level below the watched folder and no further. A film buried three
//! directories down belongs to the first of them, because the shelf is the
//! heading somebody wrote and the rest is how they filed it underneath.

use std::path::{Component, Path, PathBuf};

/// A row on the library screen, named for a folder on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shelf {
    /// What the folder is called, which is what the heading reads.
    pub name: String,
    /// The folder itself, shown beside the heading so it is clear which is meant
    /// when two watched folders hold a directory of the same name.
    pub path: PathBuf,
}

/// The shelf a film belongs on, given the watched folder it was found in.
///
/// A film sitting directly in a watched folder is gathered under that folder's
/// own name, so nothing is ever shelfless. So is a film whose path does not lie
/// under the folder at all, which should not happen and is not worth losing a
/// film over if it ever does.
#[must_use]
pub fn shelf_of(film: &Path, folder: &Path) -> Shelf {
    let Ok(under) = film.strip_prefix(folder) else {
        return folder_itself(folder);
    };

    // The file name is not a shelf, so only what it sits inside is considered.
    let directories = under.parent().unwrap_or_else(|| Path::new(""));

    match directories.components().next() {
        Some(Component::Normal(first)) => Shelf {
            name: first.to_string_lossy().into_owned(),
            path: folder.join(first),
        },
        _ => folder_itself(folder),
    }
}

/// The watched folder as its own shelf.
///
/// Named by its last part, or by the whole path where it has no last part,
/// which is what a drive root looks like on Windows and the filesystem root
/// everywhere else.
fn folder_itself(folder: &Path) -> Shelf {
    let name = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );

    Shelf {
        name,
        path: folder.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Shelf, shelf_of};

    fn shelf(film: &str, folder: &str) -> Shelf {
        shelf_of(Path::new(film), Path::new(folder))
    }

    #[test]
    fn a_film_in_a_subdirectory_is_shelved_under_it() {
        let found = shelf("/films/Crime/Heat (1995).mkv", "/films");
        assert_eq!(found.name, "Crime");
        assert_eq!(found.path, PathBuf::from("/films/Crime"));
    }

    #[test]
    fn a_film_in_the_watched_folder_itself_is_gathered_under_its_name() {
        let found = shelf("/films/Heat (1995).mkv", "/films");
        assert_eq!(found.name, "films");
        assert_eq!(found.path, PathBuf::from("/films"));
    }

    /// Only the first directory is the shelf. A season inside a director inside
    /// a watched folder is still that director's row.
    #[test]
    fn a_film_further_down_belongs_to_the_first_directory_above_it() {
        let found = shelf(
            "/films/Wong Kar-wai/Remasters/2160p/Chungking.mkv",
            "/films",
        );
        assert_eq!(found.name, "Wong Kar-wai");
        assert_eq!(found.path, PathBuf::from("/films/Wong Kar-wai"));
    }

    /// The name on its own does not say which folder is meant, which is the
    /// reason the path travels with it.
    #[test]
    fn a_directory_named_after_its_parent_keeps_them_apart_by_path() {
        let found = shelf("/films/films/Heat (1995).mkv", "/films");
        assert_eq!(found.name, "films");
        assert_eq!(found.path, PathBuf::from("/films/films"));
    }

    #[test]
    fn a_watched_folder_inside_another_is_measured_from_itself() {
        let found = shelf("/films/Crime/Noir/Heat (1995).mkv", "/films/Crime");
        assert_eq!(found.name, "Noir");
        assert_eq!(found.path, PathBuf::from("/films/Crime/Noir"));
    }

    #[test]
    fn unusual_characters_in_a_directory_name_are_carried_through_untouched() {
        let found = shelf("/films/Amélie & Co. [1080p] #2/Le Fabuleux.mkv", "/films");
        assert_eq!(found.name, "Amélie & Co. [1080p] #2");
    }

    /// A watched folder recorded with a separator on the end is the same folder,
    /// and it would be a poor reason for every film in it to lose its shelf.
    #[test]
    fn a_separator_on_the_end_of_the_watched_folder_changes_nothing() {
        let found = shelf("/films/Crime/Heat (1995).mkv", "/films/");
        assert_eq!(found.name, "Crime");
    }

    /// Should not happen: a film is found by walking its folder. If it ever
    /// does, the film keeps a shelf rather than falling off the screen.
    #[test]
    fn a_film_that_is_not_under_the_folder_falls_back_to_the_folder() {
        let found = shelf("/elsewhere/Heat (1995).mkv", "/films");
        assert_eq!(found.name, "films");
        assert_eq!(found.path, PathBuf::from("/films"));
    }

    #[test]
    fn a_folder_with_no_last_part_is_named_by_the_whole_path() {
        let found = shelf("/Heat (1995).mkv", "/");
        assert_eq!(found.name, "/");
        assert_eq!(found.path, PathBuf::from("/"));
    }
}
