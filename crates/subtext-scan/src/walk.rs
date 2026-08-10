//! Finding out what is in a folder.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use walkdir::{DirEntry, WalkDir};

use crate::media::{FileKind, classify, is_worth_walking};

/// A file as the walk found it.
///
/// Size and modification time are read here, in the one pass that already has
/// the directory entry in hand, since asking the filesystem again later would
/// double the cost of the cheapest part of a scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundFile {
    pub path: PathBuf,
    /// The name on its own, which is all the pairing has to go on.
    pub file_name: String,
    pub size_bytes: u64,
    /// Milliseconds since the epoch.
    pub modified_at: i64,
}

/// What a folder turned out to hold.
#[derive(Clone, Debug, Default)]
pub struct Discovery {
    pub films: Vec<FoundFile>,
    pub subtitles: Vec<FoundFile>,
    /// Every file the walk looked at, including the ones it had no use for.
    pub files_seen: usize,
    /// Places the walk was not allowed into, or that vanished underneath it.
    pub unreadable: Vec<PathBuf>,
}

/// Walks a folder and everything under it.
///
/// This never fails. A folder the user cannot read, or one that is removed
/// while the walk is in progress, is recorded and stepped over: a library of a
/// thousand films should not go unindexed because one directory is locked.
///
/// Symbolic links are not followed. A link into a parent directory would send
/// the walk round in circles, and a link to a film elsewhere would index the
/// same file twice under two paths.
#[must_use]
pub fn discover(root: &Path) -> Discovery {
    let mut discovery = Discovery::default();

    let walk = WalkDir::new(root)
        .follow_links(false)
        // So that two films whose names tie are paired the same way on every
        // machine and in every run.
        .sort_by_file_name()
        .into_iter()
        .filter_entry(worth_visiting);

    for entry in walk {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                if let Some(path) = error.path() {
                    discovery.unreadable.push(path.to_path_buf());
                }
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        discovery.files_seen += 1;

        let Some(file_name) = entry.file_name().to_str() else {
            continue;
        };
        let Some(kind) = classify(file_name) else {
            continue;
        };
        let Ok(metadata) = entry.metadata() else {
            discovery.unreadable.push(entry.path().to_path_buf());
            continue;
        };

        let found = FoundFile {
            path: entry.path().to_path_buf(),
            file_name: file_name.to_owned(),
            size_bytes: metadata.len(),
            modified_at: metadata.modified().ok().map_or(0, millis_since_epoch),
        };
        match kind {
            FileKind::Film => discovery.films.push(found),
            FileKind::Subtitle => discovery.subtitles.push(found),
        }
    }

    discovery
}

/// The root itself always passes, so that a folder whose own name looks like
/// one to skip is still walked when someone has explicitly asked for it.
fn worth_visiting(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    entry.file_name().to_str().is_none_or(is_worth_walking)
}

/// A file time as the database stores it.
///
/// A time the platform cannot express as milliseconds since the epoch reads as
/// zero, which makes the file look older than everything else and so always
/// worth reading again. That is the safe direction to be wrong in.
pub(crate) fn millis_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::time::{Duration, UNIX_EPOCH};

    use super::millis_since_epoch;

    #[test]
    fn a_file_time_becomes_milliseconds() {
        let time = UNIX_EPOCH + Duration::from_millis(1_700_000_000_500);
        assert_eq!(millis_since_epoch(time), 1_700_000_000_500);

        // Before the epoch, which a clock set wrongly can produce.
        assert_eq!(millis_since_epoch(UNIX_EPOCH - Duration::from_secs(1)), 0);
    }
}
