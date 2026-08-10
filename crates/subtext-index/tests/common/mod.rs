//! A database on disk for the integration tests to work against.
//!
//! A file rather than an in-memory database, because write-ahead logging, the
//! busy timeout and the file locking are the parts most worth testing, and none
//! of them exist in memory.

#![allow(clippy::unwrap_used, dead_code)]

use std::path::PathBuf;

use subtext_core::{Cue, SubtitleLabel, Timestamp};
use subtext_index::{Database, NewFilm, NewTrack, TrackMatch};
use tempfile::TempDir;

pub(crate) struct Library {
    pub(crate) database: Database,
    pub(crate) root: PathBuf,
    // Held so that the directory lasts as long as the database in it.
    _directory: TempDir,
}

impl Library {
    pub(crate) fn new() -> Self {
        let directory = TempDir::new().unwrap();
        let root = directory.path().to_path_buf();
        let database = Database::open(root.join("library.db")).unwrap();
        Self {
            database,
            root,
            _directory: directory,
        }
    }

    /// Opens the same file again, as a restart of the application would.
    pub(crate) fn reopen(&self) -> Database {
        Database::open(self.root.join("library.db")).unwrap()
    }

    pub(crate) fn watch(&self) -> i64 {
        self.database
            .folders()
            .add(&self.root.join("films"))
            .unwrap()
            .id
    }

    /// Records a film, and returns its identifier.
    pub(crate) fn add_film(&self, folder_id: i64, name: &str) -> i64 {
        self.database
            .films()
            .upsert(&NewFilm {
                folder_id,
                path: &self.root.join(format!("{name}.mkv")),
                title: name,
                year: Some(1_999),
                size_bytes: 4_000,
                modified_at: 1_700_000_000_000,
            })
            .unwrap()
            .id
    }

    /// Records a subtitle file for a film, and returns its identifier.
    pub(crate) fn add_track(&self, film_id: i64, name: &str) -> i64 {
        self.database
            .tracks()
            .upsert(&NewTrack {
                film_id,
                path: &self.root.join(format!("{name}.srt")),
                label: SubtitleLabel {
                    language: Some("en"),
                    forced: false,
                    hearing_impaired: false,
                },
                match_kind: TrackMatch::Exact,
                encoding: "UTF-8",
                size_bytes: 60_000,
                modified_at: 1_700_000_000_000,
            })
            .unwrap()
            .id
    }

    /// A film with one track carrying the given lines, one every ten seconds.
    pub(crate) fn add_film_with_dialogue(&self, folder_id: i64, name: &str, lines: &[&str]) -> i64 {
        let film_id = self.add_film(folder_id, name);
        let track_id = self.add_track(film_id, name);
        self.database
            .tracks()
            .replace_cues(track_id, &cues(lines))
            .unwrap();
        film_id
    }
}

/// Turns lines into cues ten seconds apart.
pub(crate) fn cues(lines: &[&str]) -> Vec<Cue> {
    lines
        .iter()
        .enumerate()
        .map(|(at, line)| {
            let start = u32::try_from(at).unwrap() * 10_000;
            Cue {
                index: u32::try_from(at).unwrap() + 1,
                start: Timestamp::from_millis(start),
                end: Timestamp::from_millis(start + 4_000),
                text: (*line).to_owned(),
                position: None,
            }
        })
        .collect()
}
