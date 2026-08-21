//! The artwork a media manager named beside a film.
//!
//! Kodi, Jellyfin and the tools that write for them leave a small XML file next
//! to each film recording what they worked out about it, and one element of it
//! names the picture they settled on. That name is somebody's earlier decision
//! about this film, sitting unread on their own disk, which is the only reason
//! this module exists: reading it costs one bounded read and covers a library
//! that has already been through one of those tools.
//!
//! What is read is one element out of one file, and nothing else about the
//! sidecar is understood or acted on. No XML crate is added for it. This
//! workspace names every feature it compiles and a whole parser for one element
//! is a poor trade, so what follows is a scan for that element and nothing more.
//!
//! What it understands: the `thumb` element wherever it appears, its text with
//! the five named XML entities decoded, a path given relative to the sidecar's
//! own folder or as an absolute one. What it does not: attributes, namespaces,
//! CDATA sections, character references written as numbers, and comments, any
//! of which make the value unreadable rather than wrong. A sidecar it cannot
//! make sense of is no answer, which leaves the film exactly where it was.

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::media::is_image;

/// The most of a sidecar that is ever read, and the size above which one is
/// not read at all.
///
/// A film's own sidecar is a few kilobytes. Anything past this is not the file
/// this is looking for, and reading it would put an unbounded amount of
/// somebody else's data through a scan that has no use for it.
const CAP: usize = 64 * 1024;

/// How many named pictures are looked for before the file is given up on.
///
/// A media manager writes the film's own artwork first and then one of these
/// elements for every actor in the cast, so a file can carry dozens of them.
/// Each candidate costs a look at the disk, and an answer that is not in the
/// first few is not the answer to this question.
const CANDIDATES: usize = 8;

/// The five entities XML defines by name, which is what a path in one of these
/// files can carry.
const ENTITIES: &[(&str, char)] = &[
    ("&amp;", '&'),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&quot;", '"'),
    ("&apos;", '\''),
];

/// The picture a sidecar names, where it names one that is there.
///
/// Nothing is fetched. A value naming a server is a picture on somebody else's
/// machine, and this application makes no network request, so it is passed over
/// the way a value naming a file that has been deleted is.
pub(crate) fn thumb(sidecar: &Path) -> Option<PathBuf> {
    let folder = sidecar.parent()?;
    let text = read_bounded(sidecar)?;

    named(&text)
        .take(CANDIDATES)
        .find_map(|named| resolve(folder, &named))
}

/// The whole of a sidecar, where the whole of it is small enough to be worth
/// having.
///
/// One byte past the cap is read on purpose, so that a file exactly at the cap
/// is read and one above it is told apart from it without asking the filesystem
/// a second question.
fn read_bounded(sidecar: &Path) -> Option<String> {
    let file = File::open(sidecar).ok()?;
    let mut bytes = Vec::new();
    file.take(CAP as u64 + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() > CAP {
        return None;
    }

    // These files are written by tools on three operating systems and are not
    // reliably UTF-8. A path is almost always plain ASCII, so a byte that is
    // not part of one is replaced rather than being allowed to end the read:
    // the worst it can do is name a file that is not there.
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Every picture the file names, in the order it names them.
fn named(text: &str) -> impl Iterator<Item = String> {
    let mut rest = text;

    core::iter::from_fn(move || {
        loop {
            let at = rest.find("<thumb")?;
            rest = &rest[at + "<thumb".len()..];

            // `<thumbnail>` is a different element and is not this one, so the
            // name has to end where it is expected to end.
            let ends = rest
                .chars()
                .next()
                .is_some_and(|next| next == '>' || next == '/' || next.is_whitespace());
            let close = rest.find('>')?;
            let attributes = &rest[..close];
            rest = &rest[close + 1..];

            // An element that closes itself carries no text, and one whose name
            // only began this way was somebody else's.
            if !ends || attributes.ends_with('/') {
                continue;
            }

            let end = rest.find("</thumb")?;
            let value = decode(rest[..end].trim());
            rest = &rest[end + "</thumb".len()..];
            if !value.is_empty() {
                return Some(value);
            }
        }
    })
}

/// The text of an element with its entities put back.
///
/// Almost every path comes through this unchanged. The one that does not is a
/// path with an ampersand in it, which a well behaved writer has to escape.
fn decode(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }

    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        decoded.push_str(&rest[..at]);
        rest = &rest[at..];
        // Anything that is not one of the five is left as it was written. A
        // numeric reference in a file name is rare enough that guessing at it
        // would be the larger risk, and the path it produces does not exist.
        if let Some((entity, character)) =
            ENTITIES.iter().find(|(entity, _)| rest.starts_with(entity))
        {
            decoded.push(*character);
            rest = &rest[entity.len()..];
        } else {
            decoded.push('&');
            rest = &rest[1..];
        }
    }
    decoded.push_str(rest);
    decoded
}

/// The file a named picture turns out to be, where it is a picture and it is
/// there.
fn resolve(folder: &Path, named: &str) -> Option<PathBuf> {
    if named.contains("://") {
        return None;
    }

    let named = Path::new(named);
    let path = if named.is_absolute() {
        named.to_path_buf()
    } else {
        folder.join(named)
    };

    let file_name = path.file_name()?.to_str()?;
    if !is_image(file_name) {
        return None;
    }

    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::fmt::Write as _;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::{CAP, decode, named, thumb};

    /// A folder with a sidecar in it and whatever pictures the case needs.
    struct Folder {
        directory: TempDir,
    }

    impl Folder {
        fn new() -> Self {
            Self {
                directory: TempDir::new().unwrap(),
            }
        }

        fn at(&self, relative: &str) -> PathBuf {
            self.directory.path().join(relative)
        }

        fn write(&self, relative: &str, contents: &str) -> PathBuf {
            let path = self.at(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, contents).unwrap();
            path
        }

        /// The sidecar, with the pictures it names put on the disk first.
        fn sidecar(&self, contents: &str, pictures: &[&str]) -> PathBuf {
            for picture in pictures {
                self.write(picture, "not really a picture");
            }
            self.write("movie.nfo", contents)
        }

        fn thumb(&self, contents: &str, pictures: &[&str]) -> Option<PathBuf> {
            thumb(&self.sidecar(contents, pictures))
        }
    }

    /// What one of these files looks like, cut down to the part that is read.
    fn movie(body: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n\
             <movie>\n  <title>Heat</title>\n  <year>1995</year>\n{body}</movie>\n"
        )
    }

    #[test]
    fn a_well_formed_sidecar_names_the_picture_beside_it() {
        let folder = Folder::new();
        let found = folder.thumb(&movie("  <thumb>poster.jpg</thumb>\n"), &["poster.jpg"]);

        assert_eq!(found, Some(folder.at("poster.jpg")));
    }

    #[test]
    fn the_attributes_a_media_manager_writes_are_stepped_over() {
        let folder = Folder::new();
        let found = folder.thumb(
            &movie("  <thumb aspect=\"poster\" preview=\"\">poster.jpg</thumb>\n"),
            &["poster.jpg"],
        );

        assert_eq!(found, Some(folder.at("poster.jpg")));
    }

    #[test]
    fn a_sidecar_with_no_picture_in_it_is_no_answer() {
        let folder = Folder::new();

        assert_eq!(folder.thumb(&movie(""), &["poster.jpg"]), None);
        // An element that closes itself, which is what a tool writes when it
        // knows of the field and has nothing to put in it.
        assert_eq!(folder.thumb(&movie("  <thumb />\n"), &["poster.jpg"]), None);
        assert_eq!(
            folder.thumb(&movie("  <thumb></thumb>\n"), &["poster.jpg"]),
            None
        );
        // A different element that begins the same way.
        assert_eq!(
            folder.thumb(
                &movie("  <thumbnail>poster.jpg</thumbnail>\n"),
                &["poster.jpg"]
            ),
            None
        );
    }

    /// The one that matters most, since a film's own artwork is written first
    /// and every actor in the cast is written after it.
    #[test]
    fn the_first_picture_that_is_actually_there_is_the_answer() {
        let folder = Folder::new();
        let found = folder.thumb(
            &movie(
                "  <thumb aspect=\"poster\">poster.jpg</thumb>\n  \
                 <thumb aspect=\"poster\">poster-2.jpg</thumb>\n",
            ),
            &["poster.jpg", "poster-2.jpg"],
        );

        assert_eq!(found, Some(folder.at("poster.jpg")));
    }

    /// A cast list names its portraits with the same element, and every one of
    /// them is on a server this application will never ask.
    #[test]
    fn a_picture_on_a_server_is_passed_over_for_one_on_the_disk() {
        let folder = Folder::new();
        let found = folder.thumb(
            &movie(
                "  <actor>\n    <name>Al Pacino</name>\n    \
                 <thumb>https://example.invalid/pacino.jpg</thumb>\n  </actor>\n  \
                 <thumb>poster.jpg</thumb>\n",
            ),
            &["poster.jpg"],
        );

        assert_eq!(found, Some(folder.at("poster.jpg")));
    }

    #[test]
    fn a_picture_in_another_folder_is_found_from_the_sidecar_it_was_named_in() {
        let folder = Folder::new();
        let found = folder.thumb(
            &movie("  <thumb>art/poster.jpg</thumb>\n"),
            &["art/poster.jpg"],
        );

        assert_eq!(found, Some(folder.at("art").join("poster.jpg")));
    }

    #[test]
    fn a_picture_named_in_full_is_taken_as_it_was_written() {
        let folder = Folder::new();
        let picture = folder.write("elsewhere/poster.jpg", "not really a picture");
        let body = format!("  <thumb>{}</thumb>\n", picture.display());

        assert_eq!(folder.thumb(&movie(&body), &[]), Some(picture));
    }

    #[test]
    fn a_picture_that_is_not_there_is_no_answer() {
        let folder = Folder::new();

        assert_eq!(
            folder.thumb(&movie("  <thumb>poster.jpg</thumb>\n"), &[]),
            None
        );
        // There, and not a picture this application could ever draw.
        assert_eq!(
            folder.thumb(&movie("  <thumb>poster.bmp</thumb>\n"), &["poster.bmp"]),
            None
        );
    }

    /// The other kind of file that carries this extension: a release note, in
    /// ASCII art, with no markup in it anywhere.
    #[test]
    fn a_sidecar_that_is_not_xml_at_all_is_no_answer() {
        let folder = Folder::new();
        let notes = "  .-----------------------.\n  |  GROUP presents       |\n  \
                     |  Heat.1995.1080p      |\n  '-----------------------'\n";

        assert_eq!(folder.thumb(notes, &["poster.jpg"]), None);
    }

    #[test]
    fn a_sidecar_above_the_cap_is_not_read() {
        let folder = Folder::new();
        let padding = " ".repeat(CAP);
        let body = format!("  <thumb>poster.jpg</thumb>\n{padding}");

        assert_eq!(folder.thumb(&movie(&body), &["poster.jpg"]), None);
    }

    #[test]
    fn a_sidecar_that_is_not_there_is_no_answer() {
        let folder = Folder::new();

        assert_eq!(thumb(&folder.at("movie.nfo")), None);
        // A folder rather than a file, which is what a name with no extension
        // on it can turn out to be.
        std::fs::create_dir_all(folder.at("empty.nfo")).unwrap();
        assert_eq!(thumb(&folder.at("empty.nfo")), None);
        assert_eq!(thumb(Path::new("movie.nfo")), None);
    }

    /// A file that stops in the middle of the element being read, which is what
    /// a tool interrupted while writing leaves behind.
    #[test]
    fn a_sidecar_that_stops_part_way_through_is_no_answer() {
        let folder = Folder::new();

        assert_eq!(
            folder.thumb("<movie>\n  <thumb>poster.jpg", &["poster.jpg"]),
            None
        );
        assert_eq!(folder.thumb("<movie>\n  <thumb", &["poster.jpg"]), None);
    }

    #[test]
    fn only_the_first_few_pictures_are_looked_for() {
        let mut body = String::new();
        for at in 0..40 {
            let _ = writeln!(body, "  <thumb>missing-{at}.jpg</thumb>");
        }
        body.push_str("  <thumb>poster.jpg</thumb>\n");

        let folder = Folder::new();
        assert_eq!(folder.thumb(&movie(&body), &["poster.jpg"]), None);
    }

    #[test]
    fn the_entities_a_path_can_carry_are_put_back() {
        assert_eq!(decode("poster.jpg"), "poster.jpg");
        assert_eq!(
            decode("Dungeons &amp; Dragons.jpg"),
            "Dungeons & Dragons.jpg"
        );
        assert_eq!(decode("&lt;&gt;&quot;&apos;"), "<>\"'");
        // Written as a number, which is not understood and is left alone.
        assert_eq!(decode("a&#38;b"), "a&#38;b");
        assert_eq!(decode("&"), "&");
        assert_eq!(decode("&amp"), "&amp");
    }

    #[test]
    fn every_picture_a_file_names_is_read_in_order() {
        let text = "<thumb>a.jpg</thumb><thumb aspect=\"x\">b.jpg</thumb><thumb/><thumb>c.jpg";
        let found: Vec<String> = named(text).collect();

        assert_eq!(found, ["a.jpg", "b.jpg"]);
        assert_eq!(named("").count(), 0);
    }
}
