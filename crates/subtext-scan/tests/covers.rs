//! Where a scan decides each film's cover comes from.
//!
//! The sources come in a fixed order: a cover somebody picked, the artwork
//! inside the film, the picture beside it, the picture a media manager's
//! sidecar names, the picture serving the folder above, and then nothing, which
//! leaves a frame from the film itself for the front end to take. What is
//! asserted here is which of them wins in a real folder, that a film keeps the
//! answer once it has one, and that a choice is never taken away by a scan.

// The fixture stops a test outright when the library does not hold what the
// test is about to ask it for.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use subtext_container::fixture::Entry;
use subtext_core::{Cover, CoverSource};

use crate::common::{ARTWORK, Fixture};

fn entries() -> Vec<Entry> {
    vec![Entry::video(1), Entry::audio(2)]
}

/// Where a film's cover comes from, as the row records it.
fn cover_of(library: &Fixture, relative: &str) -> Option<Cover> {
    library
        .database()
        .films()
        .by_id(library.film_id(relative))
        .unwrap()
        .expect("the film to be in the library")
        .cover
}

/// The same, as a path alone, for a test about which file won rather than
/// about what kind of claim it had.
fn cover_path_of(library: &Fixture, relative: &str) -> Option<PathBuf> {
    cover_of(library, relative).map(|cover| cover.path)
}

/// A sidecar of the shape a media manager writes, naming one picture.
fn sidecar(names: &str) -> Vec<u8> {
    let mut text = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<movie>\n");
    text.push_str("  <title>Heat</title>\n  <year>1995</year>\n");
    let _ = writeln!(text, "  <thumb aspect=\"poster\">{names}</thumb>");
    text.push_str("</movie>\n");
    text.into_bytes()
}

#[test]
fn a_film_carrying_its_own_artwork_is_its_own_cover() {
    let library = Fixture::new();
    let path = library.matroska_carrying("Heat.1995.mkv", entries(), "cover.jpg");

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat.1995.mkv"),
        Some(Cover::new(path, CoverSource::InFile))
    );
}

#[test]
fn a_picture_beside_a_film_is_its_cover() {
    let library = Fixture::new();
    library.matroska("Heat.1995.mkv", entries());
    let image = library.write("Heat.1995.jpg", ARTWORK);

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat.1995.mkv"),
        Some(Cover::new(image, CoverSource::Beside))
    );
}

#[test]
fn a_film_with_neither_leaves_the_frame_to_be_taken() {
    let library = Fixture::new();
    library.matroska("Heat.1995.mkv", entries());
    // A picture in the folder that names no film and stands beside two of them.
    library.matroska("Ronin.1998.mkv", entries());
    library.write("folder.jpg", ARTWORK);

    library.scan();
    assert_eq!(cover_of(&library, "Heat.1995.mkv"), None);
    assert_eq!(cover_of(&library, "Ronin.1998.mkv"), None);
}

/// The order, in one folder, with both answers available to the same film.
#[test]
fn what_is_inside_the_film_comes_before_what_is_beside_it() {
    let library = Fixture::new();
    let path = library.matroska_carrying("Heat (1995)/Heat.mkv", entries(), "cover.jpg");
    library.write("Heat (1995)/poster.jpg", ARTWORK);

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat (1995)/Heat.mkv"),
        Some(Cover::new(path, CoverSource::InFile))
    );
}

/// The reason the answer is written down rather than worked out afresh: a film
/// that has not changed is never opened again, so nothing else remembers that
/// its artwork is inside it.
#[test]
fn a_rescan_leaves_a_films_own_artwork_where_it_is() {
    let library = Fixture::new();
    let path = library.matroska_carrying("Heat.1995.mkv", entries(), "cover.jpg");

    library.scan();
    let outcome = library.scan();

    assert_eq!(
        outcome.films_probed, 0,
        "an unchanged film is not opened again"
    );
    assert_eq!(
        cover_of(&library, "Heat.1995.mkv"),
        Some(Cover::new(path, CoverSource::InFile))
    );
}

/// A choice is a statement about this film that no scan is entitled to
/// revisit, whatever it finds in the folder afterwards.
#[test]
fn a_cover_somebody_picked_is_left_alone_by_every_later_scan() {
    let library = Fixture::new();
    library.matroska_carrying("Heat.1995.mkv", entries(), "cover.jpg");
    library.scan();

    let picked = library.write("Chosen.png", ARTWORK);
    let chosen = Cover::new(picked, CoverSource::Chosen);
    library
        .database()
        .films()
        .set_covers(&[(library.film_id("Heat.1995.mkv"), Some(&chosen))])
        .unwrap();

    // A picture named after the film, added after the choice was made, which
    // is exactly what would have won had nobody chosen.
    library.write("Heat.1995.jpg", ARTWORK);
    library.scan();

    assert_eq!(cover_of(&library, "Heat.1995.mkv"), Some(chosen));
}

#[test]
fn a_picture_added_later_becomes_the_cover_of_a_film_that_had_none() {
    let library = Fixture::new();
    library.matroska("Heat.1995.mkv", entries());

    library.scan();
    assert_eq!(cover_of(&library, "Heat.1995.mkv"), None);

    let image = library.write("Heat.1995.png", ARTWORK);
    library.scan();
    assert_eq!(
        cover_of(&library, "Heat.1995.mkv"),
        Some(Cover::new(image, CoverSource::Beside))
    );

    // And taking it away again leaves the film where it started.
    library.remove("Heat.1995.png");
    library.scan();
    assert_eq!(cover_of(&library, "Heat.1995.mkv"), None);
}

#[test]
fn a_picture_is_not_a_film_and_not_a_subtitle() {
    let library = Fixture::new();
    library.matroska("Heat.1995.mkv", entries());
    library.write("Heat.1995.jpg", ARTWORK);
    library.subtitle("Heat.1995.srt", &["the action is the juice"]);

    let outcome = library.scan();
    assert_eq!(outcome.films_found, 1);
    assert_eq!(outcome.subtitles_found, 1);
    // Seen by the walk, since it had to be looked at to be ruled out.
    assert_eq!(outcome.files_seen, 3);
    assert!(outcome.unpaired_subtitles.is_empty());
}

#[test]
fn a_film_that_is_not_matroska_carries_no_artwork_and_is_no_trouble() {
    let library = Fixture::new();
    library.film("Heat.1995.mp4");
    let image = library.write("Heat.1995.jpg", ARTWORK);

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat.1995.mp4"),
        Some(Cover::new(image, CoverSource::Beside))
    );
}

#[test]
fn the_cover_a_film_keeps_is_a_path_the_application_can_use() {
    let library = Fixture::new();
    library.matroska_carrying("Heat.1995.mkv", entries(), "cover.jpg");

    library.scan();
    let cover = cover_path_of(&library, "Heat.1995.mkv").expect("a cover");

    // The film's own file, which is what says the image is inside it.
    assert_eq!(cover, library.path("Heat.1995.mkv"));
    assert_eq!(
        subtext_container::cover_image(Path::new(&cover))
            .unwrap()
            .as_deref(),
        Some(ARTWORK)
    );
}

/// The layout Kodi and Jellyfin leave behind, where the artwork is kept out of
/// the film's own folder and only the sidecar says where it went.
#[test]
fn a_picture_a_sidecar_names_is_the_cover_of_the_film_it_sits_beside() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    let image = library.write("Heat (1995)/artwork/heat-poster.jpg", ARTWORK);
    library.write("Heat (1995)/movie.nfo", &sidecar("artwork/heat-poster.jpg"));

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat (1995)/Heat.mkv"),
        Some(Cover::new(image, CoverSource::Sidecar))
    );
}

/// The order again, on the two claims that sit either side of a sidecar.
#[test]
fn a_picture_beside_the_film_comes_before_one_a_sidecar_names() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    let beside = library.write("Heat (1995)/poster.jpg", ARTWORK);
    library.write("Heat (1995)/artwork/heat-poster.jpg", ARTWORK);
    library.write("Heat (1995)/movie.nfo", &sidecar("artwork/heat-poster.jpg"));

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat (1995)/Heat.mkv"),
        Some(Cover::new(beside, CoverSource::Beside))
    );
}

/// A sidecar naming a picture on a server, which is every actor portrait a
/// media manager writes and the one thing this application will never ask for.
#[test]
fn a_sidecar_naming_a_picture_that_is_not_on_the_disk_covers_nothing() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    library.write(
        "Heat (1995)/movie.nfo",
        &sidecar("https://example.invalid/heat.jpg"),
    );

    library.scan();
    assert_eq!(cover_of(&library, "Heat (1995)/Heat.mkv"), None);
}

/// The box set layout, where one image sits in the folder the films are filed
/// under and stands for all of them.
#[test]
fn one_picture_covers_every_film_filed_under_the_folder_it_is_in() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    library.matroska("Ronin (1998)/Ronin.mkv", entries());
    let image = library.write("poster.jpg", ARTWORK);

    library.scan();
    let expected = Some(Cover::new(image, CoverSource::FolderAbove));
    assert_eq!(cover_of(&library, "Heat (1995)/Heat.mkv"), expected);
    assert_eq!(cover_of(&library, "Ronin (1998)/Ronin.mkv"), expected);
}

/// The claim it loses to, since a picture in the film's own folder was put
/// there for that film and this one was not.
#[test]
fn a_picture_in_the_films_own_folder_comes_before_one_serving_the_folder_above() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    let own = library.write("Heat (1995)/poster.jpg", ARTWORK);
    library.write("poster.jpg", ARTWORK);

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat (1995)/Heat.mkv"),
        Some(Cover::new(own, CoverSource::Beside))
    );
}

/// A folder holding films of its own, where the picture in it belongs to those
/// films rather than to the folders beside them.
#[test]
fn a_picture_among_films_does_not_reach_down_into_their_neighbours_folders() {
    let library = Fixture::new();
    library.matroska("Heat.1995.mkv", entries());
    library.matroska("Ronin (1998)/Ronin.mkv", entries());
    let image = library.write("poster.jpg", ARTWORK);

    library.scan();
    assert_eq!(
        cover_of(&library, "Heat.1995.mkv"),
        Some(Cover::new(image, CoverSource::Beside))
    );
    assert_eq!(cover_of(&library, "Ronin (1998)/Ronin.mkv"), None);
}

/// A sidecar is not a film and not a subtitle, and the only thing a scan does
/// with one is read a name out of it.
#[test]
fn a_sidecar_is_seen_and_indexed_as_nothing() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    library.write("Heat (1995)/artwork/heat-poster.jpg", ARTWORK);
    library.write("Heat (1995)/movie.nfo", &sidecar("artwork/heat-poster.jpg"));

    let outcome = library.scan();
    assert_eq!(outcome.films_found, 1);
    assert_eq!(outcome.subtitles_found, 0);
    // Seen by the walk, since it had to be looked at to be ruled out.
    assert_eq!(outcome.files_seen, 3);
}

/// What keeps a dressed library cheap to rescan: the answer read out of a
/// sidecar goes on the row, and the row is what every later scan reads.
///
/// The picture is taken away between the two scans, which is what proves it. A
/// scan that opened the sidecar again would find the name in it naming nothing
/// and would leave the film with no cover, so the cover surviving is the
/// sidecar going unread.
#[test]
fn a_rescan_does_not_open_a_sidecar_the_row_already_answers_for() {
    let library = Fixture::new();
    library.matroska("Heat (1995)/Heat.mkv", entries());
    let image = library.write("Heat (1995)/artwork/heat-poster.jpg", ARTWORK);
    library.write("Heat (1995)/movie.nfo", &sidecar("artwork/heat-poster.jpg"));

    library.scan();
    let expected = Some(Cover::new(image, CoverSource::Sidecar));
    assert_eq!(cover_of(&library, "Heat (1995)/Heat.mkv"), expected);

    library.remove("Heat (1995)/artwork/heat-poster.jpg");
    library.scan();
    assert_eq!(cover_of(&library, "Heat (1995)/Heat.mkv"), expected);
}
