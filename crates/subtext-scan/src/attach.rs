//! Attaching a subtitle file to a film by hand.
//!
//! What the amber row in the import sheet does. Pairing by filename gets most
//! of a library right and cannot get all of it right, so there has to be a way
//! to say "this one, actually", and that answer has to outlast every later
//! scan.

use std::path::Path;

use subtext_core::{ParseWarning, ParsedName, parse_srt};
use subtext_index::{Database, NewTrack, TrackMatch};

use crate::error::{Error, Result};
use crate::media::{self, FileKind};
use crate::walk::millis_since_epoch;

/// A subtitle file, now belonging to a film because somebody said so.
#[derive(Clone, Debug)]
pub struct Attached {
    pub track_id: i64,
    pub film_id: i64,
    pub cues: usize,
    /// What the parser had to work around, which the sheet shows for a file
    /// somebody chose themselves and might want to reconsider.
    pub warnings: Vec<ParseWarning>,
}

/// Reads a subtitle file and gives it to a film.
///
/// The file is read here and now rather than left for the next scan, because
/// the person doing it is looking at the screen and expects the transcript to
/// exist afterwards. It may sit outside every watched folder: someone whose
/// subtitles live in their downloads folder should not have to move files
/// around to use them.
pub fn attach_subtitle(database: &Database, film_id: i64, path: &Path) -> Result<Attached> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| refused(path, "it has no name this platform can read"))?;

    if media::classify(name) != Some(FileKind::Subtitle) {
        return Err(refused(path, "only SRT subtitle files can be attached"));
    }
    if database.films().by_id(film_id)?.is_none() {
        return Err(refused(path, "that film is no longer in the library"));
    }

    let metadata = std::fs::metadata(path).map_err(|error| refused(path, &error.to_string()))?;
    let bytes = std::fs::read(path).map_err(|error| refused(path, &error.to_string()))?;

    let outcome = parse_srt(&bytes);
    let encoding = outcome.track.encoding();
    let cues = outcome.track.into_cues();

    let tracks = database.tracks();
    let stored = tracks.upsert(&NewTrack {
        film_id,
        path,
        label: ParsedName::from_file_name(name).label,
        match_kind: TrackMatch::ByHand,
        encoding,
        size_bytes: metadata.len(),
        modified_at: metadata.modified().ok().map_or(0, millis_since_epoch),
    })?;

    // Writing a track leaves a pairing already made by hand pointing where it
    // pointed, which is what a scan should do and the opposite of what this is:
    // here somebody is saying where the file belongs, so it is moved.
    tracks.attach(stored.id, film_id)?;
    tracks.replace_cues(stored.id, &cues)?;

    Ok(Attached {
        track_id: stored.id,
        film_id,
        cues: cues.len(),
        warnings: outcome.warnings,
    })
}

fn refused(path: &Path, reason: &str) -> Error {
    Error::Attach {
        path: path.to_path_buf(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    // A test whose fixture will not build has nothing to assert, so it stops
    // rather than carrying on against a library that is not there.
    #![allow(clippy::unwrap_used, clippy::panic)]

    use std::path::{Path, PathBuf};

    use subtext_index::{Database, NewFilm, TrackMatch};

    use super::attach_subtitle;
    use crate::error::Error;

    const SUBTITLE: &[u8] = b"1\n00:00:01,000 --> 00:00:03,000\nWhat do you want?\n\n\
                              2\n00:00:04,000 --> 00:00:06,500\nNothing you can give me.\n";

    struct Library {
        _directory: tempfile::TempDir,
        database: Database,
        film_id: i64,
        root: PathBuf,
    }

    fn library() -> Library {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let database = Database::open(root.join("library.db")).unwrap();

        let folder = database.folders().add(&root).unwrap();
        let film_path = root.join("Heat.1995.mkv");
        std::fs::write(&film_path, b"not really a film").unwrap();
        let film = database
            .films()
            .upsert(&NewFilm {
                folder_id: folder.id,
                path: &film_path,
                title: "Heat",
                year: Some(1995),
                size_bytes: 17,
                modified_at: 1,
            })
            .unwrap();

        Library {
            _directory: directory,
            database,
            film_id: film.id,
            root,
        }
    }

    fn write_subtitle(library: &Library, name: &str) -> PathBuf {
        let path = library.root.join(name);
        std::fs::write(&path, SUBTITLE).unwrap();
        path
    }

    #[test]
    fn reads_the_file_and_gives_it_to_the_film() {
        let library = library();
        let path = write_subtitle(&library, "something else entirely.srt");

        let attached = attach_subtitle(&library.database, library.film_id, &path).unwrap();

        assert_eq!(attached.cues, 2);
        let tracks = library.database.tracks().for_film(library.film_id).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].match_kind, TrackMatch::ByHand);
        assert_eq!(tracks[0].cue_count, 2);

        let cues = library.database.tracks().cues(attached.track_id).unwrap();
        assert_eq!(cues[0].text, "What do you want?");
    }

    #[test]
    fn takes_a_subtitle_away_from_the_film_it_was_paired_with() {
        let library = library();
        let other = library
            .database
            .films()
            .upsert(&NewFilm {
                folder_id: 1,
                path: Path::new("/films/Collateral.2004.mkv"),
                title: "Collateral",
                year: Some(2004),
                size_bytes: 1,
                modified_at: 1,
            })
            .unwrap();
        let path = write_subtitle(&library, "Heat.1995.srt");

        attach_subtitle(&library.database, other.id, &path).unwrap();
        attach_subtitle(&library.database, library.film_id, &path).unwrap();

        assert!(
            library
                .database
                .tracks()
                .for_film(other.id)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            library
                .database
                .tracks()
                .for_film(library.film_id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn refuses_a_file_that_is_not_a_subtitle() {
        let library = library();
        let path = library.root.join("Heat.1995.mkv");

        let refused = attach_subtitle(&library.database, library.film_id, &path);

        assert!(matches!(refused, Err(Error::Attach { .. })));
    }

    #[test]
    fn refuses_a_film_that_is_not_there() {
        let library = library();
        let path = write_subtitle(&library, "Heat.1995.srt");

        let refused = attach_subtitle(&library.database, 9_999, &path);

        assert!(matches!(refused, Err(Error::Attach { .. })));
    }

    #[test]
    fn says_which_file_could_not_be_read() {
        let library = library();
        let missing = library.root.join("not there.srt");

        let refused = attach_subtitle(&library.database, library.film_id, &missing);

        let Err(error) = refused else {
            panic!("a file that is not there should not attach");
        };
        assert!(error.to_string().contains("not there.srt"));
    }
}
