//! A folder of films and subtitles on disk, and a library pointed at it.
//!
//! Real files in a real directory, because the whole crate is about what is on
//! a filesystem: modification times, nested directories, files that disappear.
//! None of that can be tested against a pretend one.

#![allow(clippy::unwrap_used, dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use subtext_index::{Database, WatchedFolder};
use subtext_scan::{ScanOutcome, Scanner, Silent};
use tempfile::TempDir;

pub(crate) struct Fixture {
    pub(crate) scanner: Arc<Scanner>,
    pub(crate) folder: WatchedFolder,
    pub(crate) films: PathBuf,
    pub(crate) root: PathBuf,
    // Held so that the directory lasts as long as the test.
    _directory: TempDir,
}

impl Fixture {
    /// A watched folder with nothing in it yet.
    pub(crate) fn new() -> Self {
        let directory = TempDir::new().unwrap();
        let root = directory.path().to_path_buf();
        let films = root.join("films");
        std::fs::create_dir_all(&films).unwrap();

        let database = Database::open(root.join("library.db")).unwrap();
        let scanner = Arc::new(Scanner::new(database));
        let folder = scanner.add_folder(&films).unwrap();

        Self {
            scanner,
            folder,
            films,
            root,
            _directory: directory,
        }
    }

    pub(crate) fn database(&self) -> &Database {
        self.scanner.database()
    }

    pub(crate) fn scan(&self) -> ScanOutcome {
        self.scanner.scan(&self.folder, &Silent).unwrap()
    }

    /// A path inside the watched folder, written with forward slashes whatever
    /// the platform separates its own paths with.
    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.films.clone(), |path, part| path.join(part))
    }

    /// Writes a file standing in for a film. Nothing reads its contents.
    pub(crate) fn film(&self, relative: &str) -> PathBuf {
        self.write(relative, b"not really a film")
    }

    /// Writes a subtitle file with one line of dialogue every ten seconds.
    pub(crate) fn subtitle(&self, relative: &str, lines: &[&str]) -> PathBuf {
        self.write(relative, srt(lines).as_bytes())
    }

    pub(crate) fn write(&self, relative: &str, contents: &[u8]) -> PathBuf {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, contents).unwrap();
        path
    }

    pub(crate) fn remove(&self, relative: &str) {
        std::fs::remove_file(self.path(relative)).unwrap();
    }

    /// The identifier of the film at a path, which the assertions need in
    /// order to ask what was recorded about it.
    pub(crate) fn film_id(&self, relative: &str) -> i64 {
        self.database()
            .films()
            .by_path(&self.path(relative))
            .unwrap()
            .unwrap_or_else(|| panic!("{relative} should be in the library"))
            .id
    }

    /// The lines recorded for a film, in playback order.
    pub(crate) fn dialogue(&self, relative: &str) -> Vec<String> {
        let tracks = self
            .database()
            .tracks()
            .for_film(self.film_id(relative))
            .unwrap();
        tracks
            .iter()
            .flat_map(|track| self.database().tracks().cues(track.id).unwrap())
            .map(|cue| cue.text)
            .collect()
    }
}

/// An SRT file with one cue every ten seconds.
pub(crate) fn srt(lines: &[&str]) -> String {
    use core::fmt::Write;

    let mut text = String::new();
    for (at, line) in lines.iter().enumerate() {
        let start = at * 10;
        let _ = writeln!(
            text,
            "{}\n{} --> {}\n{line}\n",
            at + 1,
            timing(start),
            timing(start + 4)
        );
    }
    text
}

fn timing(seconds: usize) -> String {
    format!("00:{:02}:{:02},000", seconds / 60, seconds % 60)
}

/// The names of the files in a list of paths, sorted, for readable assertions.
pub(crate) fn names(paths: &[PathBuf]) -> Vec<String> {
    let mut names: Vec<String> = paths
        .iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .collect();
    names.sort();
    names
}

pub(crate) fn exists(path: &Path) -> bool {
    path.try_exists().unwrap_or(false)
}
