//! The rows, as the rest of the application sees them.

use std::path::{Path, PathBuf};

use subtext_core::{Correction, Cover, Cue, MatchKind, SubtitleLabel, Timestamp};

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
    /// The image the poster was drawn from and the claim it has on this film,
    /// or nothing at all, in which case a frame from the film is all there is.
    pub cover: Option<Cover>,
    /// The colour pair taken from the poster, as the front end wrote it.
    pub accent: Option<String>,
    /// What the film's file is, which is also what says whether the rest of
    /// these facts have ever been read. Nothing for a film recorded before this
    /// build looked inside for them, until the next scan opens it.
    pub container: Option<String>,
    /// The picture, where the container said anything about it.
    pub video: Option<VideoDetails>,
    /// When the file stopped being there, or `None` while it is present.
    pub missing_since: Option<i64>,
    /// Which of the film's subtitle tracks it is watched with.
    pub choice: TrackChoice,
    pub added_at: i64,
}

impl FilmRecord {
    #[must_use]
    pub fn is_missing(&self) -> bool {
        self.missing_since.is_some()
    }
}

/// A film's picture, as the container described it.
///
/// Every field but the codec is optional, because a header states what it
/// states. A file that did not say how deep its colour is has not said, and
/// filling in the usual answer would be putting a fact on the screen that
/// nothing read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VideoDetails {
    /// The identifier the file wrote, which is named where anybody has a name
    /// for it and shown as it stands where nobody does.
    pub codec: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bit_depth: Option<u8>,
    pub frame_rate: Option<f64>,
}

/// One of a film's sound tracks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioDetails {
    /// The number the container knows the track by.
    pub stream_number: u64,
    pub codec: String,
    /// How many channels, from which the layout is named.
    pub channels: Option<u8>,
    pub language: Option<String>,
    /// The track the film suggests, which is the one that will be heard.
    pub default: bool,
}

/// What a scan found out about a film's file, on its way to being recorded.
///
/// The container is the one fact always known, since it comes from the name of
/// the file rather than from anything inside it, and it is what says that a
/// film has been looked at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaDetails {
    pub container: String,
    /// How long the film runs, where the container said. The player finds this
    /// out too, the first time anybody watches, and either answer describes the
    /// same file.
    pub duration: Option<Timestamp>,
    pub video: Option<VideoDetails>,
    pub audio: Vec<AudioDetails>,
}

/// Which subtitle track a film is watched with.
///
/// Three answers rather than two, because "show none of them" and "nobody has
/// said" are different things. The first is a decision and is kept; the second
/// leaves the track to be picked by a rule from what the pairing found, which
/// is what almost every film in a library does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackChoice {
    /// Nobody has chosen, so the rule decides.
    #[default]
    Unset,
    /// This track, whatever the rule would have said.
    Track(i64),
    /// None of them.
    Off,
}

impl TrackChoice {
    /// The two columns the choice is stored as.
    ///
    /// Turning subtitles off forgets which track was being read. It is one
    /// decision either way, and remembering the track behind an off switch
    /// would mean a fourth state that only the database could tell apart.
    #[must_use]
    pub(crate) fn columns(self) -> (Option<i64>, bool) {
        match self {
            Self::Unset => (None, false),
            Self::Track(id) => (Some(id), false),
            Self::Off => (None, true),
        }
    }

    /// Reads back what [`Self::columns`] wrote.
    #[must_use]
    pub(crate) fn from_columns(track_id: Option<i64>, off: bool) -> Self {
        match (track_id, off) {
            (_, true) => Self::Off,
            (Some(id), false) => Self::Track(id),
            (None, false) => Self::Unset,
        }
    }

    /// The track that was chosen, where one was.
    #[must_use]
    pub fn track_id(self) -> Option<i64> {
        match self {
            Self::Track(id) => Some(id),
            Self::Unset | Self::Off => None,
        }
    }

    #[must_use]
    pub fn is_off(self) -> bool {
        matches!(self, Self::Off)
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

/// Where a subtitle track was found.
///
/// Two kinds, and almost everything treats them the same way once they are in
/// the table. What differs is how they are identified, how they are read, and
/// what happens when the file they came from changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackOrigin {
    /// A subtitle file sitting beside the film, paired to it by name.
    #[default]
    Sidecar,
    /// A track inside the film's own container, muxed by whoever made the
    /// encode and therefore timed against those exact frames.
    Stream,
}

impl TrackOrigin {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sidecar => "sidecar",
            Self::Stream => "stream",
        }
    }

    /// Reads back what [`Self::as_str`] wrote.
    ///
    /// Anything else reads as a sidecar, which is what every row written before
    /// there was such a column is.
    #[must_use]
    pub fn from_stored(text: &str) -> Self {
        match text {
            "stream" => Self::Stream,
            _ => Self::Sidecar,
        }
    }

    #[must_use]
    pub fn is_stream(self) -> bool {
        matches!(self, Self::Stream)
    }
}

/// A subtitle track as it was found, whether beside the film or inside it.
#[derive(Clone, Debug)]
pub struct NewTrack<'a> {
    pub film_id: i64,
    /// The subtitle file, or the film itself for a track carried inside it.
    pub path: &'a Path,
    pub origin: TrackOrigin,
    /// The number the container knows the track by, and zero for a file.
    pub stream_number: u64,
    /// What the track is written as, under the name the container reader gives
    /// it. Text rather than a type of its own, because knowing what those names
    /// mean belongs to whatever reads the tracks and not to the table they are
    /// stored in.
    pub codec: &'a str,
    /// What the file name, or the container's own header, said the track was.
    pub label: SubtitleLabel,
    pub match_kind: TrackMatch,
    /// The encoding the file turned out to be in.
    pub encoding: &'a str,
    pub size_bytes: u64,
    pub modified_at: i64,
}

/// One track found inside a film, and the dialogue read out of it.
///
/// The dialogue is empty for a track of pictures, which is recorded so that it
/// can be named rather than read.
pub type StreamEntry<'a> = (NewTrack<'a>, &'a [Cue]);

/// One film, and everything that was found inside it.
pub type FilmStreams<'a> = (i64, Vec<StreamEntry<'a>>);

/// A subtitle track in the library.
#[derive(Clone, Debug)]
pub struct TrackRecord {
    pub id: i64,
    pub film_id: i64,
    pub path: PathBuf,
    pub origin: TrackOrigin,
    /// The number the container knows the track by, and zero for a file.
    pub stream_number: u64,
    /// What the track is written as, which decides whether its dialogue can be
    /// read at all.
    pub codec: String,
    /// The two letter language code, where the file name or the container
    /// header gave one.
    pub language: Option<String>,
    pub forced: bool,
    pub hearing_impaired: bool,
    pub match_kind: TrackMatch,
    /// How the timings inside the file line up with the film, which is a
    /// separate question from whether the file belongs to it at all.
    pub correction: Correction,
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

    use super::{Fingerprint, PlaybackPosition, TrackChoice, TrackMatch};

    #[test]
    fn a_choice_survives_a_round_trip() {
        for choice in [TrackChoice::Unset, TrackChoice::Track(4), TrackChoice::Off] {
            let (track_id, off) = choice.columns();
            assert_eq!(TrackChoice::from_columns(track_id, off), choice);
        }

        // The track a film was watched with before subtitles were turned off is
        // not somewhere the rest of the application can see it, so a row that
        // holds both reads as off rather than as either answer by chance.
        assert_eq!(TrackChoice::from_columns(Some(4), true), TrackChoice::Off);

        assert_eq!(TrackChoice::Track(4).track_id(), Some(4));
        assert_eq!(TrackChoice::Off.track_id(), None);
        assert!(TrackChoice::Off.is_off());
        assert!(!TrackChoice::Unset.is_off());
    }

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
