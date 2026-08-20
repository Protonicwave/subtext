//! Which picture on the disk is a film's cover.
//!
//! Plex, Jellyfin and Kodi have taught people to keep a film's artwork next to
//! it, either named after the film or under one of a handful of fixed names in
//! the film's own folder. Both layouts are somebody's decision about which
//! image belongs to which film, which is why they are read before a frame is
//! ever taken from the picture.
//!
//! Two more places carry the same kind of decision. A media manager writes a
//! small file beside each film naming the artwork it settled on, which is that
//! tool's answer rather than the reader's and so a step further away. And a box
//! set keeps one image in the folder its films sit in, which stands for all of
//! them and so says the least about any one of them. Both are found here, and
//! the ordering on [`CoverSource`] is where each of them sits.
//!
//! The names are reduced the way the subtitle pairing reduces them, so a cover
//! called `Heat.1995.jpg` finds `Heat.1995.1080p.BluRay.x264-GROUP.mkv` for the
//! same reason its subtitle does. What is added on top is the suffix the media
//! managers write, since `Heat.1995-poster.jpg` is the same claim about the
//! same film.
//!
//! Everything here is a question about names except one, and that one is kept
//! apart on purpose: reading a sidecar means opening a file, so a sidecar is
//! only ever named here and is opened by [`decide`], for the one film in the
//! folder that has found nothing better.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use subtext_core::{Cover, CoverSource, ParsedName};

use crate::nfo;
use crate::walk::FoundFile;

/// The names a cover is given when it is named after its folder rather than
/// after the film, in the layout where each film has a folder of its own.
const FIXED_NAMES: &[&str] = &["cover", "poster", "folder", "default"];

/// The name a media manager gives a sidecar when it names it after the folder.
const SIDECAR_NAMES: &[&str] = &["movie"];

/// What a media manager puts on the end of an image named after the film.
const SUFFIXES: &[&str] = &["poster", "cover", "folder", "thumb", "art"];

/// The separators one of those suffixes is joined on with.
const JOINERS: &[char] = &['-', '.', '_', ' '];

/// What the files on the disk claim about one film, worked out from names
/// alone.
///
/// Three of the six sources in [`CoverSource`] can be settled without opening
/// anything, and these are those three. The sidecar is the odd one out and is
/// carried here as the file to read rather than as an answer, because whether
/// it has an answer in it is not known until it is read and reading it is the
/// one part of this that costs anything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OnDisk {
    /// An image in the film's own folder.
    pub beside: Option<Cover>,
    /// The sidecar another tool wrote about this film, which names a picture
    /// rather than being one.
    pub sidecar: Option<PathBuf>,
    /// An image in the folder above, which serves every film filed under it.
    pub above: Option<Cover>,
}

/// What the pictures and sidecars on the disk claim, for each film in the order
/// the films were given.
///
/// The names are the films' parsed names, which the pairing has already worked
/// out, so a scan reduces each film's name once rather than twice. The pictures
/// and the sidecars are reduced once each here, for the same reason: a library
/// of a few thousand films holds several thousand images, and reducing every
/// image's name once for every film would be the whole cost of a scan.
///
/// Nothing is opened. Every answer here comes from a name and from where the
/// file sits.
#[must_use]
pub fn on_disk(
    films: &[FoundFile],
    names: &[ParsedName],
    images: &[FoundFile],
    sidecars: &[FoundFile],
) -> Vec<OnDisk> {
    let films_in_folder = films_per_folder(films);
    let pictures = by_folder(images, FIXED_NAMES);
    let written = by_folder(sidecars, SIDECAR_NAMES);

    films
        .iter()
        .zip(names)
        .map(|(film, name)| {
            let Some(folder) = film.path.parent() else {
                return OnDisk::default();
            };
            let alone = films_in_folder.get(folder).copied().unwrap_or_default() == 1;

            OnDisk {
                beside: best(pictures.get(folder), name, alone)
                    .map(|image| Cover::new(image, CoverSource::Beside)),
                sidecar: best(written.get(folder), name, alone).map(Path::to_path_buf),
                above: above(folder, name, &pictures, &films_in_folder),
            }
        })
        .collect()
}

/// Where one film's cover comes from, which is the only place that is decided.
///
/// The candidates are compared by the claim each has, so which one wins is the
/// ordering on [`CoverSource`] rather than the shape of the code here. What is
/// decided elsewhere is what the candidates are: this weighs them. The one
/// break in that is where the weighing stops, since the tier below a picture
/// beside the film is the tier that costs a file to be opened, and there is no
/// sense in reading one to answer a question already answered.
///
/// A cover somebody picked is not weighed at all. It is kept whatever a scan
/// has found, because the one thing a chosen cover means is that no scan is to
/// have another opinion about it.
///
/// Whether a film carries its own artwork is known only when the film was
/// opened during this scan. A film that has not changed is not opened again, so
/// what the row already said stands: it was read from the same file and the
/// file is still the same one.
///
/// This is the one part of deciding a cover that reads a file, and it does so
/// under two conditions. Nothing that outranks a sidecar has answered, so a
/// library with its artwork beside its films never opens one. And the row does
/// not already say the cover came from a sidecar, so a library that has been
/// through a media manager opens each of them once and never again. What is
/// left is a film with no cover at all, which is read again on every scan, and
/// that is the film worth asking about again: it is how a sidecar that gains a
/// picture later is ever noticed, and it costs one bounded read.
pub(crate) fn decide(
    film: &Path,
    opened: Option<bool>,
    recorded: Option<&Cover>,
    found: &OnDisk,
) -> Option<Cover> {
    if recorded.is_some_and(|cover| cover.source.is_chosen()) {
        return recorded.cloned();
    }

    let carries_its_own =
        opened.unwrap_or_else(|| recorded.is_some_and(|cover| cover.source == CoverSource::InFile));
    let inside = carries_its_own.then(|| Cover::new(film, CoverSource::InFile));

    if let Some(better) = [inside, found.beside.clone()]
        .into_iter()
        .flatten()
        .min_by_key(|cover| cover.source)
    {
        return Some(better);
    }

    let named = match recorded {
        Some(cover) if cover.source == CoverSource::Sidecar => Some(cover.clone()),
        _ => found
            .sidecar
            .as_deref()
            .and_then(nfo::thumb)
            .map(|picture| Cover::new(picture, CoverSource::Sidecar)),
    };

    [named, found.above.clone()]
        .into_iter()
        .flatten()
        .min_by_key(|cover| cover.source)
}

/// The image in the folder above that serves this film.
///
/// The condition on a fixed name is the same one it has in a film's own folder,
/// read one level up: `poster.jpg` says something about the folder it is in, so
/// it belongs to the films filed under that folder only where the folder holds
/// no films of its own. Where it does, the image is theirs and saying it also
/// belongs to their neighbours' folders would be guessing.
fn above<'a>(
    folder: &Path,
    name: &ParsedName,
    pictures: &HashMap<&'a Path, Held<'a>>,
    films_in_folder: &HashMap<&Path, usize>,
) -> Option<Cover> {
    let above = folder.parent()?;
    let bare = films_in_folder.get(above).copied().unwrap_or_default() == 0;

    best(pictures.get(above), name, bare).map(|image| Cover::new(image, CoverSource::FolderAbove))
}

/// The files in one folder that could belong to a film, sorted into the two
/// claims a name can make.
///
/// Sorted once for the folder rather than tested once for every film in it.
/// A folder of a thousand films holds several thousand pictures, and asking of
/// each picture whether it is this film's is the same question asked a million
/// times: what a film wants to know is whether anything reduced to its own
/// name, which is a lookup.
#[derive(Default)]
struct Held<'a> {
    /// Files named after a film, kept under the name they reduce to and in the
    /// order the walk found them.
    named: HashMap<String, Vec<Candidate<'a>>>,
    /// Files named after the folder rather than after any film, which belong to
    /// a film only where the folder is one it has to itself. The name of one of
    /// these says nothing about a film, so nothing is kept but the path.
    fixed: Vec<&'a Path>,
}

/// One file named after a film, with its name read once.
struct Candidate<'a> {
    path: &'a Path,
    /// The name with any media manager suffix taken off it, kept for the year,
    /// which is what tells two films of the same name apart.
    name: ParsedName,
}

/// The files in each folder, sorted by the claim their names make.
///
/// A file whose name reduces to nothing is dropped here rather than kept and
/// passed over later, since it would otherwise be claimed by every film whose
/// name reduces to nothing as well.
fn by_folder<'a>(files: &'a [FoundFile], fixed: &[&str]) -> HashMap<&'a Path, Held<'a>> {
    let mut folders: HashMap<&Path, Held<'a>> = HashMap::new();
    for file in files {
        let Some(folder) = file.path.parent() else {
            continue;
        };
        let held = folders.entry(folder).or_default();

        if fixed
            .iter()
            .any(|known| known.eq_ignore_ascii_case(stem_of(&file.file_name)))
        {
            held.fixed.push(&file.path);
            continue;
        }

        let name = ParsedName::from_file_name(&without_suffix(&file.file_name));
        if name.key.is_empty() {
            continue;
        }
        held.named
            .entry(name.key.clone())
            .or_default()
            .push(Candidate {
                path: &file.path,
                name,
            });
    }
    folders
}

/// The best of the files in one folder for one film, of which there may be
/// none.
///
/// A file named after the film outranks one named after the folder wherever
/// both are there, since the first was named for this film and the second for
/// whatever the folder happens to hold. Where two files make the same claim the
/// first the walk found wins, and the walk is sorted by name, so the answer is
/// the same on every machine and in every run.
fn best<'a>(held: Option<&'a Held<'a>>, film: &ParsedName, alone: bool) -> Option<&'a Path> {
    let held = held?;

    if !film.key.is_empty()
        && let Some(named) = held.named.get(&film.key)
        && let Some(matched) = named
            .iter()
            .find(|candidate| candidate.name.year_agrees_with(film))
    {
        return Some(matched.path);
    }

    alone.then(|| held.fixed.first().copied()).flatten()
}

/// A file name with its extension taken off.
fn stem_of(file_name: &str) -> &str {
    file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem)
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
    #![allow(clippy::unwrap_used)]

    use std::path::{Path, PathBuf};

    use subtext_core::{Cover, CoverSource, ParsedName};
    use tempfile::TempDir;

    use super::{OnDisk, decide, on_disk, without_suffix};
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

    fn found(paths: &[&str]) -> Vec<FoundFile> {
        paths.iter().copied().map(file).collect()
    }

    fn names(films: &[FoundFile]) -> Vec<ParsedName> {
        films
            .iter()
            .map(|film| ParsedName::from_file_name(&film.file_name))
            .collect()
    }

    /// What the disk claims about a set of films, as paths, so a test reads as
    /// the folder it describes.
    fn claims(films: &[&str], images: &[&str], sidecars: &[&str]) -> Vec<OnDisk> {
        let films = found(films);
        on_disk(&films, &names(&films), &found(images), &found(sidecars))
    }

    /// The picture found beside each film, which is the claim most libraries
    /// are answered by.
    fn covers(films: &[&str], images: &[&str]) -> Vec<Option<String>> {
        claims(films, images, &[])
            .into_iter()
            .map(|found| found.beside.map(|cover| cover.path.display().to_string()))
            .collect()
    }

    /// The image in the folder above each film.
    fn above(films: &[&str], images: &[&str]) -> Vec<Option<String>> {
        claims(films, images, &[])
            .into_iter()
            .map(|found| found.above.map(|cover| cover.path.display().to_string()))
            .collect()
    }

    /// The sidecar chosen for each film, which is named here and read nowhere.
    fn sidecars(films: &[&str], sidecars: &[&str]) -> Vec<Option<String>> {
        claims(films, &[], sidecars)
            .into_iter()
            .map(|found| found.sidecar.map(|path| path.display().to_string()))
            .collect()
    }

    /// A cover taken from the row, as a scan reads one back.
    fn recorded(path: &str, source: CoverSource) -> Cover {
        Cover::new(path, source)
    }

    /// A picture beside a film, as the only claim on it.
    fn beside(path: &str) -> OnDisk {
        OnDisk {
            beside: Some(recorded(path, CoverSource::Beside)),
            ..OnDisk::default()
        }
    }

    /// Every level of the order, on one film that has all of them available
    /// to it.
    #[test]
    fn the_artwork_a_film_carries_comes_before_anything_beside_it() {
        let film = Path::new("/films/Heat.1995.mkv");
        let found = beside("/films/Heat.1995.jpg");

        // Carries its own, so the picture beside it is outranked.
        assert_eq!(
            decide(film, Some(true), None, &found),
            Some(Cover::new(film, CoverSource::InFile))
        );
        // Carries none, so the picture beside it is the cover.
        assert_eq!(
            decide(film, Some(false), None, &found),
            found.beside.clone()
        );
        // Neither, which leaves a frame from the film as the only answer.
        assert_eq!(decide(film, Some(false), None, &OnDisk::default()), None);
    }

    /// The two weaker claims, which are only reached when the two stronger ones
    /// have nothing to say.
    #[test]
    fn a_picture_beside_the_film_outranks_one_that_serves_the_whole_folder() {
        let film = Path::new("/box/Heat (1995)/Heat.mkv");
        let up = recorded("/box/poster.jpg", CoverSource::FolderAbove);

        let found = OnDisk {
            above: Some(up.clone()),
            ..beside("/box/Heat (1995)/Heat.jpg")
        };
        assert_eq!(
            decide(film, Some(false), None, &found),
            found.beside.clone()
        );

        let found = OnDisk {
            above: Some(up.clone()),
            ..OnDisk::default()
        };
        assert_eq!(decide(film, Some(false), None, &found), Some(up.clone()));

        // Artwork inside the film outranks it as well.
        assert_eq!(
            decide(film, Some(true), None, &found),
            Some(Cover::new(film, CoverSource::InFile))
        );
    }

    /// The film that was not opened this time, which is almost every film in
    /// almost every scan.
    #[test]
    fn a_film_nobody_opened_keeps_what_was_recorded_about_it() {
        let film = Path::new("/films/Heat.1995.mkv");
        let inside = recorded("/films/Heat.1995.mkv", CoverSource::InFile);
        let found = beside("/films/Heat.1995.jpg");

        // The row says the artwork is inside the film, and the film has not
        // changed since it was read, so it still is.
        assert_eq!(
            decide(film, None, Some(&inside), &found),
            Some(inside.clone())
        );

        // The row says there was nothing, and a picture has appeared beside it
        // since, which is a change the scan is entitled to act on.
        assert_eq!(decide(film, None, None, &found), found.beside.clone());

        // The picture that was beside it has gone, and nothing takes its place.
        let gone = found.beside.clone().unwrap();
        assert_eq!(decide(film, None, Some(&gone), &OnDisk::default()), None);
    }

    /// The whole point of recording the source: a scan may find whatever it
    /// likes and none of it displaces a choice.
    #[test]
    fn a_cover_somebody_picked_survives_whatever_a_scan_finds() {
        let film = Path::new("/films/Heat.1995.mkv");
        let chosen = recorded("/pictures/heat.png", CoverSource::Chosen);
        let found = beside("/films/Heat.1995.jpg");

        // A picture beside the film, artwork inside it, and both at once.
        assert_eq!(
            decide(film, Some(false), Some(&chosen), &found),
            Some(chosen.clone())
        );
        assert_eq!(
            decide(film, Some(true), Some(&chosen), &OnDisk::default()),
            Some(chosen.clone())
        );
        assert_eq!(
            decide(film, Some(true), Some(&chosen), &found),
            Some(chosen.clone())
        );
        // And a scan that finds nothing at all leaves it alone as well, which
        // is the case an image somebody moved out of the folder produces.
        assert_eq!(
            decide(film, None, Some(&chosen), &OnDisk::default()),
            Some(chosen)
        );
    }

    /// The one claim that costs a file to be opened, and the two conditions
    /// under which it is.
    #[test]
    fn a_sidecar_is_read_only_where_nothing_better_has_answered() {
        let directory = TempDir::new().unwrap();
        let picture = directory.path().join("poster.jpg");
        std::fs::write(&picture, "not really a picture").unwrap();
        let sidecar = directory.path().join("movie.nfo");
        std::fs::write(&sidecar, "<movie><thumb>poster.jpg</thumb></movie>").unwrap();

        let film = Path::new("/films/Heat.1995.mkv");
        let named = Cover::new(picture, CoverSource::Sidecar);
        let found = OnDisk {
            sidecar: Some(sidecar),
            ..OnDisk::default()
        };

        assert_eq!(decide(film, Some(false), None, &found), Some(named.clone()));

        // Something better answered, so it is not read at all. The picture it
        // names is there, so a read would have found it.
        let inside = decide(film, Some(true), None, &found);
        assert_eq!(inside, Some(Cover::new(film, CoverSource::InFile)));
        let alongside = OnDisk {
            beside: Some(recorded("/films/Heat.1995.jpg", CoverSource::Beside)),
            ..found.clone()
        };
        assert_eq!(
            decide(film, Some(false), None, &alongside),
            alongside.beside.clone()
        );

        // A sidecar outranks an image serving the whole folder above.
        let with_above = OnDisk {
            above: Some(recorded("/box/poster.jpg", CoverSource::FolderAbove)),
            ..found.clone()
        };
        assert_eq!(decide(film, Some(false), None, &with_above), Some(named));
    }

    /// A library that has been through a media manager is read once and then
    /// left alone, which is what keeps a rescan of it cheap.
    #[test]
    fn a_sidecar_the_row_already_answers_for_is_not_read_again() {
        let film = Path::new("/films/Heat.1995.mkv");
        let already = recorded("/films/artwork/heat.jpg", CoverSource::Sidecar);
        // A path that is not there at all, so a read would give nothing back.
        let found = OnDisk {
            sidecar: Some(PathBuf::from("/films/movie.nfo")),
            ..OnDisk::default()
        };

        assert_eq!(decide(film, None, Some(&already), &found), Some(already));

        // The row says nothing was found, so the sidecar is asked again. It is
        // not there, which is the same answer as before.
        assert_eq!(decide(film, None, None, &found), None);
    }

    /// A picture in the film's own folder is that claim and says so, since
    /// which claim it is decides what a later scan may do with it.
    #[test]
    fn a_picture_found_beside_a_film_says_that_is_where_it_came_from() {
        assert_eq!(
            claims(&["/films/Heat.1995.mkv"], &["/films/Heat.1995.jpg"], &[])[0].beside,
            Some(Cover::new("/films/Heat.1995.jpg", CoverSource::Beside))
        );
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
                &["/films/Eighties/Heat.1995.jpg"]
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

    /// The box set layout: one folder, one image, several films filed under it
    /// in folders of their own.
    #[test]
    fn one_image_serves_every_film_filed_under_the_folder_it_is_in() {
        assert_eq!(
            above(
                &[
                    "/box/Alien (1979)/Alien.1979.mkv",
                    "/box/Aliens (1986)/Aliens.1986.mkv"
                ],
                &["/box/poster.jpg"]
            ),
            [
                Some("/box/poster.jpg".to_owned()),
                Some("/box/poster.jpg".to_owned())
            ]
        );
    }

    #[test]
    fn a_folder_above_with_one_film_under_it_is_no_different() {
        assert_eq!(
            above(&["/box/Alien (1979)/Alien.1979.mkv"], &["/box/folder.jpg"]),
            [Some("/box/folder.jpg".to_owned())]
        );
    }

    /// A folder that holds films of its own, where the image in it is theirs
    /// rather than their neighbours'.
    #[test]
    fn a_fixed_name_in_a_folder_that_holds_films_stays_with_them() {
        let films = ["/films/Heat.1995.mkv", "/films/Ronin (1998)/Ronin.1998.mkv"];

        // One film loose in the folder, so the image is beside that one.
        assert_eq!(
            covers(&films, &["/films/poster.jpg"]),
            [Some("/films/poster.jpg".to_owned()), None]
        );
        assert_eq!(above(&films, &["/films/poster.jpg"]), [None, None]);
    }

    /// A picture one folder up named after the film, which is a claim on that
    /// film wherever the folder holds anything else.
    #[test]
    fn a_name_that_matches_reaches_up_a_folder() {
        assert_eq!(
            above(
                &["/films/Heat (1995)/Heat.1995.mkv"],
                &["/films/Heat.1995.jpg"]
            ),
            [Some("/films/Heat.1995.jpg".to_owned())]
        );
    }

    #[test]
    fn the_sidecar_a_film_is_read_from_is_chosen_the_way_a_picture_is() {
        // Named after the film.
        assert_eq!(
            sidecars(&["/films/Heat.1995.mkv"], &["/films/Heat.1995.nfo"]),
            [Some("/films/Heat.1995.nfo".to_owned())]
        );
        // Named after the folder, in a folder this film has to itself.
        assert_eq!(
            sidecars(
                &["/films/Heat (1995)/Heat.mkv"],
                &["/films/Heat (1995)/movie.nfo"]
            ),
            [Some("/films/Heat (1995)/movie.nfo".to_owned())]
        );
        // Named after the folder, in a folder of several films, where it says
        // nothing about which of them it is for.
        assert_eq!(
            sidecars(
                &["/films/Heat.1995.mkv", "/films/Ronin.1998.mkv"],
                &["/films/movie.nfo"]
            ),
            [None, None]
        );
        // Somebody else's.
        assert_eq!(
            sidecars(&["/films/Heat.1995.mkv"], &["/films/Ronin.1998.nfo"]),
            [None]
        );
        assert_eq!(sidecars(&["/films/Heat.1995.mkv"], &[]), [None]);
    }

    /// Only one sidecar is ever named for a film, so only one is ever opened
    /// for it.
    #[test]
    fn a_film_with_two_sidecars_is_only_ever_read_from_one() {
        assert_eq!(
            sidecars(
                &["/films/Heat (1995)/Heat.1995.mkv"],
                &[
                    "/films/Heat (1995)/Heat.1995.nfo",
                    "/films/Heat (1995)/movie.nfo"
                ]
            ),
            [Some("/films/Heat (1995)/Heat.1995.nfo".to_owned())]
        );
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
        assert_eq!(
            covers(&["/Heat.1995.mkv"], &["/Heat.1995.jpg"]),
            [Some("/Heat.1995.jpg".to_owned())]
        );
        // Nothing above the root to hold a picture that serves it.
        assert_eq!(above(&["/Heat.1995.mkv"], &["/Heat.1995.jpg"]), [None]);
    }
}
