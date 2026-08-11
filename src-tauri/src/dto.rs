//! The shapes that cross the boundary between Rust and the front end.
//!
//! Deliberately separate from the types in the crates underneath. Those are
//! written for the work they do; these are written for what a screen needs, and
//! they are what `tauri-specta` turns into TypeScript. Keeping them apart means
//! the schema can change without the front end changing, and the other way
//! round.

use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use subtext_core::Timestamp;
use subtext_index::{FilmRecord, PlaybackPosition, TrackMatch, TrackRecord, WatchedFolder};
use subtext_scan::{ScanOutcome, ScanProgress, ScanStage};
use tauri_specta::Event;

/// Something that went wrong, said in a way a person can act on.
///
/// Errors do not cross the boundary as types. A front end cannot do anything
/// useful with a variant name, and every failure here comes down to a sentence
/// to put on the screen.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub(crate) struct Failure {
    pub(crate) message: String,
}

impl Failure {
    pub(crate) fn of(error: impl core::fmt::Display) -> Self {
        Self {
            message: error.to_string(),
        }
    }

    pub(crate) fn saying(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

/// A row number on its way to the front end.
///
/// SQLite hands out sixty-four bit identifiers, and JavaScript cannot hold
/// every value one of those can. It goes across as a plain number rather than
/// as a bigint, because a library would have to reach nine thousand million
/// films before an identifier stopped being exact, and a bigint would make
/// every comparison and every URL on every screen more awkward for a limit
/// nobody will meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub(crate) struct Id(#[specta(type = Number)] i64);

impl Id {
    pub(crate) fn of(id: i64) -> Self {
        Self(id)
    }

    pub(crate) fn get(self) -> i64 {
        self.0
    }
}

/// A moment, in milliseconds since the epoch. Wide for the same reason as
/// [`Id`], and narrowed for the same one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub(crate) struct Millis(#[specta(type = Number)] i64);

impl Millis {
    fn of(millis: i64) -> Self {
        Self(millis)
    }
}

/// The result of every command.
pub(crate) type Answer<T> = Result<T, Failure>;

/// A folder the library is a view onto.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FolderView {
    pub(crate) id: Id,
    pub(crate) path: String,
    pub(crate) added_at: Millis,
    pub(crate) films: u32,
    /// Whether changes to it are being noticed as they happen.
    pub(crate) watching: bool,
}

impl FolderView {
    pub(crate) fn of(folder: &WatchedFolder, films: usize, watching: bool) -> Self {
        Self {
            id: Id::of(folder.id),
            path: folder.path.display().to_string(),
            added_at: Millis::of(folder.added_at),
            films: u32::try_from(films).unwrap_or(u32::MAX),
            watching,
        }
    }
}

/// A film and everything the library screen needs to draw it.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FilmView {
    pub(crate) id: Id,
    pub(crate) folder_id: Id,
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) year: Option<u16>,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) poster_path: Option<String>,
    pub(crate) accent: Option<AccentView>,
    /// The file is not where it was. The film is kept anyway.
    pub(crate) missing: bool,
    pub(crate) tracks: Vec<TrackView>,
    pub(crate) position: Option<PositionView>,
}

impl FilmView {
    pub(crate) fn of(
        film: FilmRecord,
        tracks: Vec<TrackRecord>,
        position: Option<PlaybackPosition>,
    ) -> Self {
        Self {
            id: Id::of(film.id),
            folder_id: Id::of(film.folder_id),
            path: film.path.display().to_string(),
            title: film.title,
            year: film.year,
            duration_ms: film.duration.map(Timestamp::millis),
            poster_path: film.poster_path.map(|path| path.display().to_string()),
            accent: film.accent.as_deref().and_then(AccentView::parse),
            missing: film.missing_since.is_some(),
            tracks: tracks.into_iter().map(TrackView::of).collect(),
            position: position.map(PositionView::of),
        }
    }
}

/// A subtitle file paired with a film.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackView {
    pub(crate) id: Id,
    pub(crate) path: String,
    pub(crate) language: Option<String>,
    pub(crate) forced: bool,
    pub(crate) hearing_impaired: bool,
    pub(crate) match_kind: MatchKindView,
    pub(crate) cue_count: u32,
}

impl TrackView {
    pub(crate) fn of(track: TrackRecord) -> Self {
        Self {
            id: Id::of(track.id),
            path: track.path.display().to_string(),
            language: track.language,
            forced: track.forced,
            hearing_impaired: track.hearing_impaired,
            match_kind: MatchKindView::of(track.match_kind),
            cue_count: u32::try_from(track.cue_count).unwrap_or(u32::MAX),
        }
    }
}

/// How sure the pairing between a film and a subtitle file is.
#[derive(Clone, Copy, Debug, Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MatchKindView {
    Exact,
    Approximate,
    ByHand,
}

impl MatchKindView {
    fn of(kind: TrackMatch) -> Self {
        match kind {
            TrackMatch::Exact => Self::Exact,
            TrackMatch::Approximate => Self::Approximate,
            TrackMatch::ByHand => Self::ByHand,
        }
    }
}

/// The colour pair taken from a film's own frame.
///
/// Kept in one column as two hex triples separated by a space, because nothing
/// ever queries one colour without the other and a second column would only be
/// a second thing to keep in step.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccentView {
    /// The colour the film's glow and its accent are drawn in.
    pub(crate) primary: String,
    /// The second colour of the pair, which the ambient wash uses.
    pub(crate) pair: String,
}

impl AccentView {
    /// Reads back what [`Self::stored`] wrote, or nothing if the column holds
    /// something else. Anything unreadable leaves the film on the default
    /// accent, which is a duller library and not a broken one.
    fn parse(stored: &str) -> Option<Self> {
        let (primary, pair) = stored.split_once(' ')?;
        (is_hex(primary) && is_hex(pair)).then(|| Self {
            primary: primary.to_owned(),
            pair: pair.to_owned(),
        })
    }

    /// The pair as one column, refusing anything that is not a hex triple.
    ///
    /// These two strings are written into CSS custom properties, so this is the
    /// point at which a value that came from the front end stops being trusted.
    pub(crate) fn stored(&self) -> Result<String, Failure> {
        if !is_hex(&self.primary) || !is_hex(&self.pair) {
            return Err(Failure::saying("that is not a pair of colours"));
        }
        Ok(format!("{} {}", self.primary, self.pair))
    }
}

/// Whether a string is a colour of the form `#rrggbb`.
fn is_hex(colour: &str) -> bool {
    let Some(digits) = colour.strip_prefix('#') else {
        return false;
    };
    digits.len() == 6 && digits.bytes().all(|digit| digit.is_ascii_hexdigit())
}

/// A film with no frame captured from it yet.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PosterWanted {
    pub(crate) id: Id,
    pub(crate) path: String,
}

/// Where a film was left.
#[derive(Clone, Copy, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PositionView {
    pub(crate) position_ms: u32,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) finished: bool,
    pub(crate) updated_at: Millis,
    /// How far through, from zero to one, where the running time is known.
    #[specta(type = Option<Number>)]
    pub(crate) progress: Option<f32>,
}

impl PositionView {
    fn of(position: PlaybackPosition) -> Self {
        Self {
            position_ms: position.position.millis(),
            duration_ms: position.duration.map(Timestamp::millis),
            finished: position.finished,
            updated_at: Millis::of(position.updated_at),
            progress: position.progress(),
        }
    }
}

/// What a scan found, once it has finished.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanSummary {
    pub(crate) folder_id: Id,
    pub(crate) films_found: u32,
    pub(crate) subtitles_found: u32,
    pub(crate) films_paired: u32,
    pub(crate) subtitles_read: u32,
    pub(crate) cues_indexed: u32,
    pub(crate) films_missing: u32,
    /// Subtitle files belonging to no film, which the import sheet offers to
    /// attach by hand.
    pub(crate) unpaired_subtitles: Vec<String>,
    /// Films with no subtitle, which get no transcript and no search.
    pub(crate) films_without_subtitles: Vec<String>,
    /// Files that could not be read at all.
    pub(crate) unreadable: Vec<String>,
    /// Files the parser had to work around, and what was wrong with each.
    pub(crate) warnings: Vec<FileWarnings>,
}

impl ScanSummary {
    pub(crate) fn of(outcome: &ScanOutcome) -> Self {
        Self {
            folder_id: Id::of(outcome.folder_id),
            films_found: count(outcome.films_found),
            subtitles_found: count(outcome.subtitles_found),
            films_paired: count(outcome.films_paired),
            subtitles_read: count(outcome.subtitles_read),
            cues_indexed: count(outcome.cues_indexed),
            films_missing: count(outcome.films_missing),
            unpaired_subtitles: paths(&outcome.unpaired_subtitles),
            films_without_subtitles: paths(&outcome.films_without_subtitles),
            unreadable: paths(&outcome.unreadable),
            warnings: outcome
                .warnings
                .iter()
                .map(|warned| FileWarnings {
                    path: warned.path.display().to_string(),
                    warnings: warned.warnings.iter().map(ToString::to_string).collect(),
                })
                .collect(),
        }
    }
}

/// What the parser had to work around in one file, as sentences.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub(crate) struct FileWarnings {
    pub(crate) path: String,
    pub(crate) warnings: Vec<String>,
}

/// Every folder a scan covered, once all of them have finished.
#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
pub(crate) struct ScanFinished(pub Vec<ScanSummary>);

/// A scan that stopped part way through, and why.
///
/// Whatever it had already written is still there: batches are committed
/// whole, so this means some files went unread, not that any were read badly.
#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
pub(crate) struct ScanFailed(pub Failure);

/// How far along a scan is.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgressed {
    pub(crate) folder_id: Id,
    pub(crate) stage: StageView,
    pub(crate) files_seen: u32,
    pub(crate) films_found: u32,
    pub(crate) subtitles_found: u32,
    pub(crate) films_paired: u32,
    pub(crate) subtitles_to_read: u32,
    pub(crate) subtitles_read: u32,
    pub(crate) cues_indexed: u32,
    /// How much of the reading is done, from zero to one.
    #[specta(type = Number)]
    pub(crate) fraction_read: f32,
}

impl ScanProgressed {
    pub(crate) fn of(progress: &ScanProgress) -> Self {
        Self {
            folder_id: Id::of(progress.folder_id),
            stage: StageView::of(progress.stage),
            files_seen: count(progress.files_seen),
            films_found: count(progress.films_found),
            subtitles_found: count(progress.subtitles_found),
            films_paired: count(progress.films_paired),
            subtitles_to_read: count(progress.subtitles_to_read),
            subtitles_read: count(progress.subtitles_read),
            cues_indexed: count(progress.cues_indexed),
            fraction_read: progress.fraction_read(),
        }
    }
}

/// Which step of a scan is running.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum StageView {
    Discovering,
    Pairing,
    Indexing,
    Finished,
}

impl StageView {
    fn of(stage: ScanStage) -> Self {
        match stage {
            ScanStage::Discovering => Self::Discovering,
            ScanStage::Pairing => Self::Pairing,
            ScanStage::Indexing => Self::Indexing,
            ScanStage::Finished => Self::Finished,
        }
    }
}

/// A count on its way to a front end that has no use for more than four
/// thousand million of anything.
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn paths(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    // A test that cannot get at the value it is about to check has nothing to
    // say, so it stops rather than passing quietly.
    #![allow(clippy::expect_used)]

    use super::AccentView;

    fn accent(primary: &str, pair: &str) -> AccentView {
        AccentView {
            primary: primary.to_owned(),
            pair: pair.to_owned(),
        }
    }

    #[test]
    fn an_accent_survives_a_round_trip() {
        let stored = accent("#e8a33d", "#2b6c7a")
            .stored()
            .expect("a pair of hex colours should be storable");
        assert_eq!(stored, "#e8a33d #2b6c7a");

        let read = AccentView::parse(&stored).expect("what was written should read back");
        assert_eq!(read.primary, "#e8a33d");
        assert_eq!(read.pair, "#2b6c7a");
    }

    #[test]
    fn anything_that_is_not_a_colour_is_refused() {
        // These end up in a CSS custom property, so the interesting case is not
        // a typo but a value chosen to close the declaration and open another.
        for rubbish in [
            "red",
            "#e8a33",
            "#e8a33dd",
            "e8a33d",
            "#ggghhh",
            "#e8a33d;--colour-bg:#fff",
        ] {
            assert!(accent(rubbish, "#2b6c7a").stored().is_err());
            assert!(accent("#2b6c7a", rubbish).stored().is_err());
        }
    }

    #[test]
    fn a_column_holding_something_else_leaves_the_film_on_the_default() {
        for stored in ["", "#e8a33d", "#e8a33d #2b6c7a #fff", "red green"] {
            assert!(AccentView::parse(stored).is_none());
        }
    }
}
