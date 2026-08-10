//! A film on disk.

use std::path::PathBuf;

use crate::pairing::ParsedName;

/// A film file, with what its name says about it.
///
/// The path is the identity. Subtext never copies or moves anything, so a film
/// is the file where it already is, and the title is only what the file name
/// could be read as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Film {
    pub path: PathBuf,
    pub title: String,
    pub year: Option<u16>,
}

impl Film {
    /// Reads a film from its path. Touches the filesystem in no way.
    #[must_use]
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let parsed = ParsedName::from_file_name(&file_name);

        Self {
            title: parsed.title,
            year: parsed.year,
            path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Film;

    #[test]
    fn reads_the_title_and_year_from_the_file_name() {
        let film = Film::from_path("/films/drama/The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
        assert_eq!(film.title, "The Matrix");
        assert_eq!(film.year, Some(1999));
        assert!(
            film.path
                .ends_with("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv")
        );
    }

    #[test]
    fn a_path_with_no_file_name_is_not_a_problem() {
        let film = Film::from_path("");
        assert!(film.title.is_empty());
        assert!(film.year.is_none());
    }
}
