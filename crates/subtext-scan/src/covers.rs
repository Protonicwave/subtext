//! Which picture beside a film is that film's cover.
//!
//! Plex, Jellyfin and Kodi have taught people to keep a film's artwork next to
//! it, either named after the film or under one of a handful of fixed names in
//! the film's own folder. Both layouts are somebody's decision about which
//! image belongs to which film, which is why they are read before a frame is
//! ever taken from the picture.
//!
//! The names are reduced the way the subtitle pairing reduces them, so a cover
//! called `Heat.1995.jpg` finds `Heat.1995.1080p.BluRay.x264-GROUP.mkv` for the
//! same reason its subtitle does. What is added on top is the suffix the media
//! managers write, since `Heat.1995-poster.jpg` is the same claim about the
//! same film.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use subtext_core::ParsedName;

use crate::walk::FoundFile;

/// The names a cover is given when it is named after its folder rather than
/// after the film, in the layout where each film has a folder of its own.
const FIXED_NAMES: &[&str] = &["cover", "poster", "folder", "default"];

/// What a media manager puts on the end of an image named after the film.
const SUFFIXES: &[&str] = &["poster", "cover", "folder", "thumb", "art"];

/// The separators one of those suffixes is joined on with.
const JOINERS: &[char] = &['-', '.', '_', ' '];

/// How good a claim an image has on a film. Lower is better.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Claim {
    /// The image is named after the film.
    Named,
    /// The image carries one of the fixed names, in a folder holding one film.
    Fixed,
}

/// Where one film's cover comes from, which is the only place that is decided.
///
/// Three answers, tried in order. The artwork the film carries inside it wins,
/// then the picture somebody put beside it, and then nothing, which leaves a
/// frame taken from the film itself as the only answer left. The first two were
/// chosen by a person and the third is a guess, which is the whole reason for
/// the order.
///
/// Whether a film carries its own artwork is known only when the film was
/// opened during this scan. A film that has not changed is not opened again, so
/// what the row already said stands: it was read from the same file and the
/// file is still the same one.
pub(crate) fn decide(
    film: &Path,
    opened: Option<bool>,
    recorded: Option<&Path>,
    beside: Option<&Path>,
) -> Option<PathBuf> {
    let carries_its_own = opened.unwrap_or_else(|| recorded == Some(film));

    if carries_its_own {
        return Some(film.to_path_buf());
    }
    beside.map(Path::to_path_buf)
}

/// The cover beside each film, in the order the films were given.
///
/// The names are the films' parsed names, which the pairing has already worked
/// out, so a scan reduces each film's name once rather than twice.
pub(crate) fn beside(
    films: &[FoundFile],
    names: &[ParsedName],
    images: &[FoundFile],
) -> Vec<Option<PathBuf>> {
    let films_in_folder = films_per_folder(films);

    films
        .iter()
        .zip(names)
        .map(|(film, name)| best_for(film, name, images, &films_in_folder))
        .collect()
}

/// Every image that could be this film's cover, of which the best is kept.
///
/// Kept rather than returned, because which one wins is a question about the
/// whole set: a fixed name only counts where the folder holds one film, and an
/// image named after the film outranks one named after the folder wherever both
/// are there.
fn best_for(
    film: &FoundFile,
    name: &ParsedName,
    images: &[FoundFile],
    films_in_folder: &HashMap<&Path, usize>,
) -> Option<PathBuf> {
    let folder = film.path.parent()?;
    let alone = films_in_folder.get(folder).copied().unwrap_or_default() == 1;

    images
        .iter()
        .filter(|image| image.path.parent() == Some(folder))
        .filter_map(|image| claim_of(image, name, alone).map(|claim| (claim, image)))
        // The walk is sorted by name, so two images with equal claim always
        // come to the same answer rather than to whichever the filesystem
        // happened to hand over first.
        .min_by_key(|(claim, _)| *claim)
        .map(|(_, image)| image.path.clone())
}

/// What claim one image has on one film.
fn claim_of(image: &FoundFile, film: &ParsedName, alone: bool) -> Option<Claim> {
    if is_fixed_name(&image.file_name) {
        return alone.then_some(Claim::Fixed);
    }

    let named = ParsedName::from_file_name(&without_suffix(&image.file_name));
    let matches = !film.key.is_empty() && named.key == film.key && named.year_agrees_with(film);

    matches.then_some(Claim::Named)
}

/// Whether an image is named after the folder it is in rather than after a film.
fn is_fixed_name(file_name: &str) -> bool {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);

    FIXED_NAMES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(stem))
}

/// The same name with the media manager's suffix taken off.
///
/// The extension is put back, since the name reader takes a name apart from the
/// end and a stem ending in a year would otherwise lose the year to it.
fn without_suffix(file_name: &str) -> String {
    let Some((stem, extension)) = file_name.rsplit_once('.') else {
        return file_name.to_owned();
    };

    for suffix in SUFFIXES {
        for joiner in JOINERS {
            let tail_length = suffix.len() + joiner.len_utf8();
            let Some(head) = stem.len().checked_sub(tail_length).map(|at| &stem[..at]) else {
                continue;
            };
            let tail = &stem[head.len()..];
            if tail.starts_with(*joiner) && tail[joiner.len_utf8()..].eq_ignore_ascii_case(suffix) {
                return format!("{head}.{extension}");
            }
        }
    }

    file_name.to_owned()
}

fn films_per_folder(films: &[FoundFile]) -> HashMap<&Path, usize> {
    let mut counts = HashMap::new();
    for film in films {
        if let Some(folder) = film.path.parent() {
            *counts.entry(folder).or_insert(0) += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use subtext_core::ParsedName;

    use super::{beside, decide, without_suffix};
    use crate::walk::FoundFile;

    fn file(path: &str) -> FoundFile {
        let path = PathBuf::from(path);
        FoundFile {
            file_name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned(),
            path,
            size_bytes: 0,
            modified_at: 0,
        }
    }

    /// The covers found for a set of films, as paths, so a test reads as the
    /// folder it describes.
    fn covers(films: &[&str], images: &[&str]) -> Vec<Option<String>> {
        let films: Vec<FoundFile> = films.iter().copied().map(file).collect();
        let images: Vec<FoundFile> = images.iter().copied().map(file).collect();
        let names: Vec<ParsedName> = films
            .iter()
            .map(|film| ParsedName::from_file_name(&film.file_name))
            .collect();

        beside(&films, &names, &images)
            .into_iter()
            .map(|found| found.map(|path| path.display().to_string()))
            .collect()
    }

    /// Every level of the order, on one film that has all three answers
    /// available to it.
    #[test]
    fn the_artwork_a_film_carries_comes_before_anything_beside_it() {
        let film = Path::new("/films/Heat.1995.mkv");
        let beside = Path::new("/films/Heat.1995.jpg");

        // Carries its own, so the picture beside it is not reached.
        assert_eq!(
            decide(film, Some(true), None, Some(beside)),
            Some(film.to_path_buf())
        );
        // Carries none, so the picture beside it is the cover.
        assert_eq!(
            decide(film, Some(false), None, Some(beside)),
            Some(beside.to_path_buf())
        );
        // Neither, which leaves a frame from the film as the only answer.
        assert_eq!(decide(film, Some(false), None, None), None);
    }

    /// The film that was not opened this time, which is almost every film in
    /// almost every scan.
    #[test]
    fn a_film_nobody_opened_keeps_what_was_recorded_about_it() {
        let film = Path::new("/films/Heat.1995.mkv");
        let beside = Path::new("/films/Heat.1995.jpg");

        // The row says the artwork is inside the film, and the film has not
        // changed since it was read, so it still is.
        assert_eq!(
            decide(film, None, Some(film), Some(beside)),
            Some(film.to_path_buf())
        );

        // The row says there was nothing, and a picture has appeared beside it
        // since, which is a change the scan is entitled to act on.
        assert_eq!(
            decide(film, None, None, Some(beside)),
            Some(beside.to_path_buf())
        );

        // The picture that was beside it has gone, and nothing takes its place.
        assert_eq!(decide(film, None, Some(beside), None), None);
    }

    #[test]
    fn an_image_named_after_the_film_is_its_cover() {
        assert_eq!(
            covers(
                &["/films/Heat.1995.1080p.BluRay.x264-GROUP.mkv"],
                &["/films/Heat.1995.jpg"]
            ),
            [Some("/films/Heat.1995.jpg".to_owned())]
        );
    }

    #[test]
    fn the_suffix_a_media_manager_writes_is_taken_off_first() {
        for name in [
            "Heat.1995-poster.jpg",
            "Heat.1995.poster.jpg",
            "Heat.1995_thumb.png",
            "Heat.1995 cover.webp",
        ] {
            let path = format!("/films/{name}");
            assert_eq!(
                covers(&["/films/Heat.1995.mkv"], &[&path]),
                [Some(path.clone())],
                "{name}"
            );
        }
    }

    #[test]
    fn a_fixed_name_serves_the_film_it_shares_a_folder_with() {
        assert_eq!(
            covers(
                &["/films/Heat (1995)/Heat.mkv"],
                &["/films/Heat (1995)/poster.jpg"]
            ),
            [Some("/films/Heat (1995)/poster.jpg".to_owned())]
        );
    }

    /// The case a fixed name cannot answer: several films in one folder, where
    /// `folder.jpg` says something about the folder and nothing about which
    /// film it belongs to.
    #[test]
    fn a_fixed_name_in_a_folder_of_films_belongs_to_none_of_them() {
        assert_eq!(
            covers(
                &["/films/Heat.1995.mkv", "/films/Ronin.1998.mkv"],
                &["/films/folder.jpg"]
            ),
            [None, None]
        );
    }

    #[test]
    fn a_name_that_matches_wins_over_the_folders_own_picture() {
        assert_eq!(
            covers(
                &["/films/Heat (1995)/Heat.1995.mkv"],
                &[
                    "/films/Heat (1995)/cover.jpg",
                    "/films/Heat (1995)/Heat.1995.png"
                ]
            ),
            [Some("/films/Heat (1995)/Heat.1995.png".to_owned())]
        );
    }

    #[test]
    fn an_image_in_another_folder_is_somebody_elses() {
        assert_eq!(
            covers(
                &["/films/Nineties/Heat.1995.mkv"],
                &["/films/Eighties/Heat.1995.jpg", "/films/poster.jpg"]
            ),
            [None]
        );
    }

    #[test]
    fn the_wrong_film_in_the_same_folder_keeps_its_own_cover() {
        assert_eq!(
            covers(
                &["/films/Heat.1995.mkv", "/films/Ronin.1998.mkv"],
                &["/films/Ronin.1998.jpg"]
            ),
            [None, Some("/films/Ronin.1998.jpg".to_owned())]
        );
    }

    #[test]
    fn a_year_that_disagrees_is_a_different_film() {
        assert_eq!(
            covers(&["/films/Dune.2021.mkv"], &["/films/Dune.1984.jpg"]),
            [None]
        );
    }

    #[test]
    fn a_film_with_no_picture_beside_it_has_no_cover() {
        assert_eq!(covers(&["/films/Heat.1995.mkv"], &[]), [None]);
        assert_eq!(
            covers(&["/films/Heat.1995.mkv"], &["/films/backdrop.jpg"]),
            [None]
        );
    }

    #[test]
    fn a_name_with_nothing_in_it_matches_nothing() {
        // A film whose name reduces to nothing would otherwise be paired with
        // every image whose name does the same.
        assert_eq!(covers(&["/films/1080p.mkv"], &["/films/----.jpg"]), [None]);
    }

    #[test]
    fn a_suffix_is_only_taken_off_the_end() {
        assert_eq!(
            without_suffix("The.Poster.Boys.2019.mkv"),
            "The.Poster.Boys.2019.mkv"
        );
        assert_eq!(without_suffix("Heat.1995-poster.jpg"), "Heat.1995.jpg");
        // Nothing but a suffix, which leaves a name with no film in it rather
        // than an empty string.
        assert_eq!(without_suffix("poster.jpg"), "poster.jpg");
        assert_eq!(without_suffix("cover"), "cover");
    }

    #[test]
    fn nothing_pairs_with_nothing() {
        assert!(covers(&[], &["/films/poster.jpg"]).is_empty());
        assert_eq!(covers(&["/films/Heat.1995.mkv"], &[]), [None]);
    }

    #[test]
    fn a_film_at_the_root_of_a_drive_is_no_trouble() {
        let found = covers(&["/Heat.1995.mkv"], &["/Heat.1995.jpg"]);
        assert_eq!(found, [Some("/Heat.1995.jpg".to_owned())]);
    }
}
