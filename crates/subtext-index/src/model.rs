//! The rows, as the rest of the application sees them.

use std::path::{Path, PathBuf};

use subtext_core::{MatchKind, SubtitleLabel, Timestamp};

/// A folder the library watches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchedFolder {
    pub id: i64,
    pub path: PathBuf,
    /// Milliseconds since the epoch.
    pub added_at: i64,
}

/// What recording a file came to.
///
/// The `changed` flag is what makes an incremental rescan worth doing: a file
/// whose row is already right writes nothing and reports so, and the scanner
/// can then skip reading and reparsing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stored {
    pub id: i64,
    pub changed: bool,
}

/// A film as it was found on disk.
#[derive(Clone, Debug)]
pub struct NewFilm<'a> {
    pub folder_id: i64,
    pub path: &'a Path,
    pub title: &'a str,
    pub year: Option<u16>,
    pub size_bytes: u64,
    /// Modification time in milliseconds since the epoch.
    pub modified_at: i64,
}

/// A film in the library.
#[derive(Clone, Debug)]
pub struct FilmRecord {
    pub id: i64,
    pub folder_id: i64,
    pub path: PathBuf,
    pub title: String,
    pub year: Option<u16>,
    pub size_bytes: u64,
    pub modified_at: i64,
    /// How long the film runs, once the player has opened it and found out.
    pub duration: Option<Timestamp>,
    pub poster_path: Option<PathBuf>,
    /// The colour pair taken from the poster, as the front end wrote it.
    pub accent: Option<String>,
    /// When the file stopped being there, or `None` while it is present.
    pub missing_since: Option<i64>,
    pub added_at: i64,
}

impl FilmRecord {
    #[must_use]
    pub fn is_missing(&self) -> bool {
        self.missing_since.is_some()
    }
}

/// What a rescan compares against before it reads a file again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub id: i64,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_at: i64,
    pub missing: bool,
}

impl Fingerprint {
    /// Whether the file on disk is the one this row was written from.
    #[must_use]
    pub fn matches(&self, size_bytes: u64, modified_at: i64) -> bool {
        !self.missing && self.size_bytes == size_bytes && self.modified_at == modified_at
    }
}

/// A subtitle file the library already knows about, as a rescan sees it.
///
/// Separate from [`Fingerprint`] because a subtitle file raises a question a
/// film does not: not only whether it needs reading again, but whether it still
/// belongs to the film it was paired with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackPairing {
    pub id: i64,
    pub film_id: i64,
    pub path: PathBuf,
    pub match_kind: TrackMatch,
    pub size_bytes: u64,
    pub modified_at: i64,
}

impl TrackPairing {
    /// Whether the file on disk is the one this row was written from.
    #[must_use]
    pub fn matches(&self, size_bytes: u64, modified_at: i64) -> bool {
        self.size_bytes == size_bytes && self.modified_at == modified_at
    }
}

/// Where a subtitle track came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackMatch {
    /// The two names reduced to exactly the same thing.
    #[default]
    Exact,
    /// One name is the beginning of the other.
    Approximate,
    /// Someone attached it themselves, which no rescan may overrule.
    ByHand,
}

impl TrackMatch {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Approximate => "approximate",
            Self::ByHand => "by-hand",
        }
    }

    /// Reads back what [`Self::as_str`] wrote.
    ///
    /// Anything else is read as approximate, which is the one that asks to be
    /// checked, so an unknown value shows up in the import sheet rather than
    /// passing itself off as certain.
    #[must_use]
    pub fn from_stored(text: &str) -> Self {
        match text {
            "exact" => Self::Exact,
            "by-hand" => Self::ByHand,
            _ => Self::Approximate,
        }
    }
}

impl From<MatchKind> for TrackMatch {
    fn from(kind: MatchKind) -> Self {
        match kind {
            MatchKind::Exact => Self::Exact,
            MatchKind::Approximate => Self::Approximate,
        }
    }
}

/// A subtitle file as it was found on disk.
#[derive(Clone, Debug)]
pub struct NewTrack<'a> {
    pub film_id: i64,
    pub path: &'a Path,
    /// What the file name said the track was.
    pub label: SubtitleLabel,
    pub match_kind: TrackMatch,
    /// The encoding the file turned out to be in.
    pub encoding: &'a str,
    pub size_bytes: u64,
    pub modified_at: i64,
}

/// A subtitle track in the library.
#[derive(Clone, Debug)]
pub struct TrackRecord {
    pub id: i64,
    pub film_id: i64,
    pub path: PathBuf,
    /// The two letter language code, where the file name gave one.
    pub language: Option<String>,
    pub forced: bool,
    pub hearing_impaired: bool,
    pub match_kind: TrackMatch,
    pub encoding: String,
    pub cue_count: usize,
    pub size_bytes: u64,
    pub modified_at: i64,
}

/// Where a film was left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackPosition {
    pub film_id: i64,
    pub position: Timestamp,
    pub duration: Option<Timestamp>,
    /// Watched to the end, so the library offers it as finished rather than as
    /// something to carry on with.
    pub finished: bool,
    pub updated_at: i64,
}

impl PlaybackPosition {
    /// How far through the film this is, from zero to one, where the running
    /// time is known.
    #[must_use]
    // The division is done at f64 and the result is a fraction between zero and
    // one, so narrowing it costs nothing anyone can see.
    #[allow(clippy::cast_possible_truncation)]
    pub fn progress(&self) -> Option<f32> {
        let duration = self.duration?.millis();
        (duration > 0).then(|| {
            (f64::from(self.position.millis()) / f64::from(duration)).clamp(0.0, 1.0) as f32
        })
    }
}

/// A film with somewhere to carry on from.
#[derive(Clone, Debug)]
pub struct Resumable {
    pub film: FilmRecord,
    pub position: PlaybackPosition,
}

#[cfg(test)]
mod tests {
    use subtext_core::{MatchKind, Timestamp};

    use super::{Fingerprint, PlaybackPosition, TrackMatch};

    #[test]
    fn a_match_kind_survives_a_round_trip() {
        for kind in [
            TrackMatch::Exact,
            TrackMatch::Approximate,
            TrackMatch::ByHand,
        ] {
            assert_eq!(TrackMatch::from_stored(kind.as_str()), kind);
        }
        assert_eq!(TrackMatch::from_stored("nonsense"), TrackMatch::Approximate);
        assert_eq!(TrackMatch::from(MatchKind::Exact), TrackMatch::Exact);
    }

    #[test]
    fn a_fingerprint_matches_only_the_file_it_came_from() {
        let fingerprint = Fingerprint {
            id: 1,
            path: "/films/heat.mkv".into(),
            size_bytes: 4_000,
            modified_at: 1_700_000_000_000,
            missing: false,
        };

        assert!(fingerprint.matches(4_000, 1_700_000_000_000));
        assert!(!fingerprint.matches(4_001, 1_700_000_000_000));
        assert!(!fingerprint.matches(4_000, 1_700_000_000_001));

        // A file that came back after being marked missing is read again, since
        // what came back may not be what went away.
        let returned = Fingerprint {
            missing: true,
            ..fingerprint
        };
        assert!(!returned.matches(4_000, 1_700_000_000_000));
    }

    #[test]
    fn progress_needs_a_running_time() {
        let mut position = PlaybackPosition {
            film_id: 1,
            position: Timestamp::from_seconds(30),
            duration: None,
            finished: false,
            updated_at: 0,
        };
        assert!(position.progress().is_none());

        position.duration = Some(Timestamp::from_seconds(120));
        assert!((position.progress().unwrap_or_default() - 0.25).abs() < f32::EPSILON);

        // A position past the end, which happens when a file is replaced by a
        // shorter cut of the same film.
        position.position = Timestamp::from_seconds(600);
        assert!((position.progress().unwrap_or_default() - 1.0).abs() < f32::EPSILON);
    }
}
