//! The shapes that cross the boundary between Rust and the front end.
//!
//! Deliberately separate from the types in the crates underneath. Those are
//! written for the work they do; these are written for what a screen needs, and
//! they are what `tauri-specta` turns into TypeScript. Keeping them apart means
//! the schema can change without the front end changing, and the other way
//! round.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use subtext_align::{Alignment, Confidence, Landing, Misfit};
use subtext_container::{SubtitleCodec, audio_codec_name, video_codec_name};
use subtext_core::{Correction, Cue, CuePosition, Timestamp, shelf_of};
use subtext_index::{
    AudioDetails, FilmRecord, PlaybackPosition, TrackMatch, TrackOrigin, TrackRecord, VideoDetails,
    WatchedFolder,
};
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
    /// The row this film sits on, read from where it is filed on disk.
    ///
    /// Worked out here rather than on the other side, so that the front end
    /// groups by a value it was given instead of taking paths apart with a
    /// separator it would have to guess at.
    pub(crate) shelf: ShelfView,
    pub(crate) title: String,
    pub(crate) year: Option<u16>,
    /// When the film was first recorded, which is as near as this gets to when
    /// somebody acquired it. A file's own timestamps are not used: copying a
    /// library onto a new drive rewrites those and would date every film to the
    /// afternoon of the copy.
    pub(crate) added_at: Millis,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) poster_path: Option<String>,
    pub(crate) accent: Option<AccentView>,
    /// The file is not where it was. The film is kept anyway.
    pub(crate) missing: bool,
    /// What the file turned out to be, once a scan has looked at it.
    ///
    /// One group rather than a dozen loose fields, because none of it means
    /// anything on its own and all of it arrives together. Absent for a film
    /// recorded before this build described such things, until the next scan.
    pub(crate) details: Option<MediaView>,
    pub(crate) tracks: Vec<TrackView>,
    /// The track this film is watched with, where somebody has said which.
    ///
    /// Null covers two different things, and the flag beside it is what tells
    /// them apart: nobody has chosen, in which case the front end picks one by
    /// its rule, or subtitles have been turned off and none is wanted.
    pub(crate) chosen_track_id: Option<Id>,
    pub(crate) subtitles_off: bool,
    pub(crate) position: Option<PositionView>,
}

impl FilmView {
    /// The folder is the watched folder this film was found in, which is what
    /// its shelf is measured from. Nothing but a row that has lost its folder
    /// arrives without one, and such a film is shelved under the directory it
    /// sits in rather than disappearing from the screen.
    pub(crate) fn of(
        film: FilmRecord,
        folder: Option<&Path>,
        tracks: Vec<TrackRecord>,
        audio: Vec<AudioDetails>,
        position: Option<PlaybackPosition>,
    ) -> Self {
        let details = MediaView::of(&film, audio);
        let shelf = ShelfView::of(&film.path, folder);

        Self {
            id: Id::of(film.id),
            folder_id: Id::of(film.folder_id),
            path: film.path.display().to_string(),
            shelf,
            title: film.title,
            year: film.year,
            added_at: Millis::of(film.added_at),
            duration_ms: film.duration.map(Timestamp::millis),
            poster_path: film.poster_path.map(|path| path.display().to_string()),
            accent: film.accent.as_deref().and_then(AccentView::parse),
            missing: film.missing_since.is_some(),
            details,
            tracks: tracks.into_iter().map(TrackView::of).collect(),
            chosen_track_id: film.choice.track_id().map(Id::of),
            subtitles_off: film.choice.is_off(),
            position: position.map(PositionView::of),
        }
    }
}

/// The row a film sits on, which is the folder somebody filed it in.
///
/// The name is the heading and the path is what sits beside it, since two
/// watched folders can each hold a directory called the same thing and the name
/// alone would not say which row is which.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShelfView {
    pub(crate) name: String,
    pub(crate) path: String,
}

impl ShelfView {
    fn of(film: &Path, folder: Option<&Path>) -> Self {
        let folder = folder.unwrap_or_else(|| film.parent().unwrap_or_else(|| Path::new("")));
        let shelf = shelf_of(film, folder);
        Self {
            name: shelf.name,
            path: shelf.path.display().to_string(),
        }
    }
}

/// What a film's file is, as the sheet describes it.
///
/// Everything inside was read once, when the film was scanned, and is held on
/// the row from then on. A fact the file did not state is absent rather than
/// nought, so a screen can leave out what it does not know instead of printing
/// a number nobody wrote.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaView {
    /// What the file is, which is known for every film since it comes from the
    /// name rather than from anything inside.
    pub(crate) container: String,
    #[specta(type = Number)]
    pub(crate) size_bytes: u64,
    /// Bits a second across the whole film, from its size and its running time.
    ///
    /// An average, and labelled as one wherever it is shown: no encode of
    /// anything worth watching holds one rate from beginning to end.
    pub(crate) average_bitrate: Option<u32>,
    pub(crate) video: Option<VideoView>,
    pub(crate) audio: Vec<AudioView>,
}

impl MediaView {
    /// What a film's row says about its file, or nothing where it has not been
    /// looked at.
    fn of(film: &FilmRecord, audio: Vec<AudioDetails>) -> Option<Self> {
        Some(Self {
            container: film.container.clone()?,
            size_bytes: film.size_bytes,
            average_bitrate: bitrate_of(film.size_bytes, film.duration),
            video: film.video.as_ref().map(VideoView::of),
            audio: audio.into_iter().map(AudioView::of).collect(),
        })
    }
}

/// A film's picture.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VideoView {
    /// What the codec is called, and what the file wrote where nothing here has
    /// a name for it. Showing the identifier is more use than the word unknown.
    pub(crate) codec: String,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) bit_depth: Option<u8>,
    #[specta(type = Option<Number>)]
    pub(crate) frame_rate: Option<f64>,
}

impl VideoView {
    fn of(video: &VideoDetails) -> Self {
        Self {
            codec: named(video_codec_name(&video.codec), &video.codec),
            width: video.width,
            height: video.height,
            bit_depth: video.bit_depth,
            frame_rate: video.frame_rate,
        }
    }
}

/// One of a film's sound tracks.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioView {
    pub(crate) codec: String,
    /// What the channel count amounts to, said the way somebody would say it.
    pub(crate) layout: Option<String>,
    pub(crate) language: Option<String>,
    /// The track the film suggests, which is the one that will be heard.
    pub(crate) default: bool,
}

impl AudioView {
    fn of(audio: AudioDetails) -> Self {
        Self {
            codec: named(audio_codec_name(&audio.codec), &audio.codec),
            layout: audio.channels.map(layout_of),
            language: audio.language,
            default: audio.default,
        }
    }
}

/// A codec's name, or the identifier the file wrote where there is no name.
fn named(name: Option<&'static str>, codec_id: &str) -> String {
    name.map_or_else(|| codec_id.to_owned(), ToOwned::to_owned)
}

/// What a number of channels is called.
///
/// Only the counts that mean one arrangement are named. Six is written as five
/// point one because that is what six channels are in every film anybody owns,
/// and anything this does not name is given as a count rather than as a guess
/// at where the speakers are.
fn layout_of(channels: u8) -> String {
    match channels {
        1 => "Mono".to_owned(),
        2 => "Stereo".to_owned(),
        6 => "5.1".to_owned(),
        8 => "7.1".to_owned(),
        other => format!("{other} channels"),
    }
}

/// Bits a second, from how large a film is and how long it runs.
///
/// Both halves are needed, and a film nobody has watched whose container never
/// stated a running time has only one of them. That is a fact the sheet leaves
/// out rather than a nought to print.
fn bitrate_of(size_bytes: u64, duration: Option<Timestamp>) -> Option<u32> {
    let millis = u64::from(duration?.millis());
    if millis == 0 {
        return None;
    }
    u32::try_from(size_bytes.saturating_mul(8_000) / millis).ok()
}

/// A subtitle track of a film, whether beside it or inside it.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackView {
    pub(crate) id: Id,
    pub(crate) path: String,
    pub(crate) origin: TrackOriginView,
    /// The number the container knows the track by, which is what tells two
    /// tracks of one film apart when they say the same thing about themselves.
    /// Zero for a subtitle file.
    pub(crate) stream_number: u32,
    /// Whether the dialogue in this track can be read.
    pub(crate) form: TrackFormView,
    pub(crate) language: Option<String>,
    pub(crate) forced: bool,
    pub(crate) hearing_impaired: bool,
    pub(crate) match_kind: MatchKindView,
    pub(crate) cue_count: u32,
    /// What the timings have been put through to match the film.
    ///
    /// Carried so that the sync control opens on the value in force rather than
    /// on nothing, and so that the player knows what the cues it was handed
    /// have already had done to them.
    pub(crate) correction: CorrectionView,
}

impl TrackView {
    pub(crate) fn of(track: TrackRecord) -> Self {
        Self {
            id: Id::of(track.id),
            path: track.path.display().to_string(),
            origin: TrackOriginView::of(track.origin),
            stream_number: u32::try_from(track.stream_number).unwrap_or_default(),
            form: TrackFormView::of(&track.codec),
            language: track.language,
            forced: track.forced,
            hearing_impaired: track.hearing_impaired,
            match_kind: MatchKindView::of(track.match_kind),
            cue_count: u32::try_from(track.cue_count).unwrap_or(u32::MAX),
            correction: CorrectionView::of(track.correction),
        }
    }
}

/// How a track's timings relate to the film it is paired with.
///
/// One shape rather than two loose numbers, because neither means anything
/// without the other and both travel together in each direction: out on the
/// track, and back in when somebody has settled on a value.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorrectionView {
    /// How far the whole track is moved, in milliseconds. Negative is earlier.
    pub(crate) offset_ms: i32,
    /// What the track is stretched by, for a file written against a release at
    /// a different framerate. One for almost every track.
    #[specta(type = Number)]
    pub(crate) rate: f64,
}

impl CorrectionView {
    fn of(correction: Correction) -> Self {
        Self {
            offset_ms: correction.offset_ms(),
            rate: correction.rate(),
        }
    }

    /// The correction as the core understands it, with anything out of range
    /// brought back into it.
    pub(crate) fn wanted(self) -> Correction {
        Correction::new(self.offset_ms, self.rate)
    }
}

/// How many of a track's lines arrive when somebody starts talking.
///
/// The one figure in an alignment that was not read off the correlation that
/// produced the answer, which is what makes it worth showing. It is quoted
/// twice, once for the file as it stands and once for what a measurement would
/// make of it, because neither number means much alone: no film puts every line
/// on a voice, and what a person wants to know is whether this would be better.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LandingView {
    /// The share of lines landing on the start of an utterance, none to all.
    #[specta(type = Number)]
    pub(crate) fraction: f32,
    /// How far the middle line falls from the nearest one, in milliseconds.
    pub(crate) median_ms: u32,
    /// How many lines were measured. Nought where there was nothing to measure,
    /// which is what tells an empty answer from a bad one.
    pub(crate) examined: u32,
}

impl LandingView {
    fn of(landing: Landing) -> Self {
        Self {
            fraction: landing.fraction(),
            median_ms: landing.median_ms(),
            examined: landing.examined(),
        }
    }
}

/// What a measurement was made against.
///
/// There are two things in a film that say where its dialogue falls, and they
/// are not equally good. The soundtrack is always there and has to be decoded
/// and read, which takes seconds and is only ever an estimate of where somebody
/// is talking. Another text subtitle track of the same film is authored timings
/// on both sides, needs no decoder, and is accurate to the bin, but most films
/// do not carry one.
///
/// Which of the two answered is worth saying, because it is the difference
/// between a measurement to the millisecond and one to the fraction of a second,
/// and somebody reading the sentence should know which they have been handed.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(
    tag = "against",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ReferenceView {
    /// The talking in the film's own soundtrack.
    Speech,
    /// Another text subtitle track of the same film.
    Track {
        /// What language it is in, where the file says.
        language: Option<String>,
        /// Whether it came from inside the film rather than from beside it. A
        /// track inside the film was muxed against these exact frames, which is
        /// why it is preferred and why it is worth naming.
        inside: bool,
    },
}

/// What the film itself said about the answer it was handed.
///
/// The parts rather than the number they come to. A single figure is a verdict,
/// and a verdict is exactly what somebody cannot check: told that a measurement
/// is nine tenths sure, there is nothing to do but believe it or not. Told that
/// the film was measured in twenty stretches, that eighteen of them agreed, and
/// that the middle one sits forty milliseconds from the answer, somebody can
/// weigh it, and can watch a minute of the film and see whether it is so.
///
/// The score comes too, because it is the number the bar was cleared on and
/// leaving it out would mean the front end could not say what was decided, only
/// what was seen.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GroundsView {
    /// How many stretches of the film agreed with the answer.
    pub(crate) agreed: u32,
    /// How many stretches it was measured in, which is what the count above is
    /// out of. Nought where there was nothing to measure.
    pub(crate) across: u32,
    /// How far the middle stretch sits from the answer, in milliseconds.
    pub(crate) spread_ms: u32,
    /// The parts as one number, which is what an answer is accepted on.
    #[specta(type = Number)]
    pub(crate) score: f32,
}

impl GroundsView {
    fn of(confidence: Confidence) -> Self {
        Self {
            agreed: confidence.agreed(),
            across: confidence.across(),
            // A stretch nothing could be measured in reports a distance of
            // everything, which is not a figure to send to a screen.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            spread_ms: if confidence.across() == 0 {
                0
            } else {
                confidence.spread_ms().round() as u32
            },
            score: confidence.score(),
        }
    }
}

/// How an attempt to line a subtitle track up with its film ended.
///
/// One shape covering every ending, so that the front end matches on this and
/// has no case of its own to invent. Only the first of them writes anything.
/// The rest leave the track exactly as it was, with the nudge keys still where
/// they were, which is the point: an answer nobody can check is worse than no
/// answer, and each of these says plainly which it is.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(
    tag = "outcome",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum AlignmentView {
    /// The track has been moved to where the dialogue in the film actually is.
    Aligned {
        correction: CorrectionView,
        /// What was in force beforehand, so that it can be put back.
        previous: CorrectionView,
        /// What the film said about the answer: how many of its stretches
        /// agreed, out of how many, and how closely.
        grounds: GroundsView,
        /// How the lines land now.
        landing: LandingView,
        /// How they landed before, which is what makes the figure above mean
        /// something.
        as_written: LandingView,
        /// What the track was measured against, since the two answers are not
        /// of the same quality and the figures above read differently for each.
        reference: ReferenceView,
    },
    /// The track carries too few lines to be correlated with anything.
    ///
    /// What a forced or signs-only track looks like. Settled from the count on
    /// the track's own row, before any audio is read.
    TooFewCues {
        cues: u32,
        /// How many it would have taken.
        wanted: u32,
    },
    /// The film has no soundtrack, so there is nothing to measure against.
    NoAudio,
    /// The soundtrack is in a codec this build has no decoder for.
    ///
    /// The same boundary the README states about video, in the same terms:
    /// nothing is broken and trying again will not help.
    Unsupported {
        /// What the codec is called, where it is one anybody has a name for.
        codec: Option<String>,
    },
    /// A measurement was made and is not worth acting on.
    ///
    /// What a subtitle belonging to a different film produces. The correction
    /// it would have written comes back with it, since somebody who wants it
    /// can still type it in, but nothing has been written.
    Uncertain {
        correction: CorrectionView,
        #[specta(type = Number)]
        confidence: f32,
        /// How sure it would have had to be.
        #[specta(type = Number)]
        wanted: f32,
        landing: LandingView,
        as_written: LandingView,
    },
    /// A measurement was made, believed, and would not have helped.
    ///
    /// The correlation can be perfectly clear about a lag and still be answering
    /// the wrong question, since the confidence beside it is read off the same
    /// peak and cannot see past it. This is the outcome where the answer was put
    /// to the film itself and did not put more lines on the talking than the
    /// file already does, or did not put enough of them there to be worth
    /// writing over somebody's library.
    NoBetter {
        correction: CorrectionView,
        landing: LandingView,
        as_written: LandingView,
        /// The share of lines a measurement has to land to be written.
        #[specta(type = Number)]
        wanted: f32,
    },
    /// The film has time cut into it, so no one shift describes all of it.
    ///
    /// What a recording off a broadcast carries. Every part of the film knows
    /// where its own lines are and the parts disagree with each other, because a
    /// break moves everything after it and nothing before it. The lines are all
    /// there, which is why this is worth saying rather than simply refusing:
    /// somebody who knows their file has breaks in it knows what they are looking
    /// at.
    Breaks {
        /// Roughly where the film jumps, in milliseconds. Roughly because it is
        /// read off stretches of film several minutes long.
        at_ms: u32,
        /// How the lines land as the file stands, which is the evidence for
        /// leaving it alone.
        as_written: LandingView,
    },
    /// The subtitle describes a different cut of this film.
    ///
    /// Somewhere in the film nobody says any of the lines being looked for, and it
    /// goes on that way. This is not a timing problem and no correction answers
    /// it: bending a theatrical subtitle onto an extended cut produces something
    /// that looks aligned and is wrong in every added scene. No correction is
    /// offered, deliberately, since a number here would only invite that.
    DifferentCut {
        /// Roughly where it stopped matching, in milliseconds.
        from_ms: u32,
        as_written: LandingView,
    },
    /// The film could not be opened, or nothing in it could be made sense of.
    Unreadable { message: String },
    /// Somebody asked for it to stop, and it stopped.
    Stopped,
}

impl AlignmentView {
    pub(crate) fn aligned(
        found: &Alignment,
        previous: Correction,
        reference: ReferenceView,
    ) -> Self {
        Self::Aligned {
            correction: CorrectionView::of(found.correction()),
            previous: CorrectionView::of(previous),
            grounds: GroundsView::of(found.confidence()),
            landing: LandingView::of(found.landing()),
            as_written: LandingView::of(found.as_written()),
            reference,
        }
    }

    pub(crate) fn uncertain(found: &Alignment, confidence: f32, wanted: f32) -> Self {
        Self::Uncertain {
            correction: CorrectionView::of(found.correction()),
            confidence,
            wanted,
            landing: LandingView::of(found.landing()),
            as_written: LandingView::of(found.as_written()),
        }
    }

    /// A film one correction cannot describe, said as the shape it is.
    pub(crate) fn misfit(found: &Alignment, misfit: Misfit) -> Self {
        let as_written = LandingView::of(found.as_written());
        match misfit {
            Misfit::Breaks { at_ms } => Self::Breaks { at_ms, as_written },
            Misfit::DifferentCut { from_ms } => Self::DifferentCut {
                from_ms,
                as_written,
            },
        }
    }

    pub(crate) fn no_better(found: &Alignment, wanted: f32) -> Self {
        Self::NoBetter {
            correction: CorrectionView::of(found.correction()),
            landing: LandingView::of(found.landing()),
            as_written: LandingView::of(found.as_written()),
            wanted,
        }
    }
}

/// Whether a track is a file beside the film or a stream inside it.
#[derive(Clone, Copy, Debug, Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TrackOriginView {
    Sidecar,
    Stream,
}

impl TrackOriginView {
    fn of(origin: TrackOrigin) -> Self {
        match origin {
            TrackOrigin::Sidecar => Self::Sidecar,
            TrackOrigin::Stream => Self::Stream,
        }
    }
}

/// What a track is made of, which is what decides whether it can be read.
///
/// The codec itself does not cross the boundary. Everything a screen has to say
/// about a track comes down to these three cases, and sending the name instead
/// would mean the front end keeping its own list of which codecs are which.
#[derive(Clone, Copy, Debug, Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TrackFormView {
    /// Dialogue, which is what the player draws and steps between.
    Text,
    /// Images of dialogue, as Blu-ray discs and DVDs carry. Naming them is as
    /// far as this goes: reading them would mean optical character
    /// recognition, a large dependency and an accuracy figure nothing else
    /// here has to apologise for.
    Pictures,
    /// Something this build has no name for.
    Unrecognised,
}

impl TrackFormView {
    fn of(codec: &str) -> Self {
        let codec = SubtitleCodec::from_stored(codec);
        if codec.is_text() {
            return Self::Text;
        }
        if codec.is_bitmap() {
            return Self::Pictures;
        }
        Self::Unrecognised
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

/// One line of dialogue, as the player wants it.
///
/// Deliberately flat and deliberately small. A film of five thousand cues
/// crosses the boundary in one go and is then searched sixty times a second, so
/// every field here is a number or a string the renderer uses directly, with
/// nothing to unpack on the other side.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CueView {
    /// Position in the track, counted from one in playback order.
    pub(crate) index: u32,
    pub(crate) start_ms: u32,
    pub(crate) end_ms: u32,
    /// The dialogue as plain text. A cue of several lines keeps its breaks.
    pub(crate) text: String,
    /// Where the file asked for the cue to be drawn, if it asked at all.
    pub(crate) position: Option<CuePositionView>,
}

impl CueView {
    pub(crate) fn of(cue: Cue) -> Self {
        Self {
            index: cue.index,
            start_ms: cue.start.millis(),
            end_ms: cue.end.millis(),
            text: cue.text,
            position: cue.position.map(CuePositionView::of),
        }
    }
}

/// One of the nine places a cue can ask to be drawn.
#[derive(Clone, Copy, Debug, Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CuePositionView {
    TopLeft,
    TopCentre,
    TopRight,
    MiddleLeft,
    MiddleCentre,
    MiddleRight,
    BottomLeft,
    BottomCentre,
    BottomRight,
}

impl CuePositionView {
    fn of(position: CuePosition) -> Self {
        match position {
            CuePosition::TopLeft => Self::TopLeft,
            CuePosition::TopCentre => Self::TopCentre,
            CuePosition::TopRight => Self::TopRight,
            CuePosition::MiddleLeft => Self::MiddleLeft,
            CuePosition::MiddleCentre => Self::MiddleCentre,
            CuePosition::MiddleRight => Self::MiddleRight,
            CuePosition::BottomLeft => Self::BottomLeft,
            CuePosition::BottomCentre => Self::BottomCentre,
            CuePosition::BottomRight => Self::BottomRight,
        }
    }
}

/// One preference, as it is stored.
///
/// Both halves are text, and what the text means belongs to the settings
/// screen. Sending them across as they are keeps a control that is added there
/// from being a change here as well, and it is why adding one is a change to
/// one file rather than to a command, a type and a screen.
#[derive(Clone, Debug, Serialize, Type)]
pub(crate) struct PreferenceView {
    pub(crate) key: String,
    pub(crate) value: String,
}

/// A film with no poster drawn for it yet.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PosterWanted {
    pub(crate) id: Id,
    pub(crate) path: String,
    /// Whether there is an image on the disk to draw it from, either inside the
    /// film or beside it. Where there is not, a frame has to be taken from the
    /// picture, which means opening the film and decoding it.
    pub(crate) cover: bool,
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
    /// Films with no subtitle, which the sheet marks as such.
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
    pub(crate) films_to_read: u32,
    pub(crate) films_read: u32,
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
            films_to_read: count(progress.films_to_read),
            films_read: count(progress.films_read),
            cues_indexed: count(progress.cues_indexed),
            fraction_read: progress.fraction_read(),
        }
    }
}

/// How far along an alignment is.
///
/// Reading the film is nearly all of the time an alignment takes, and it is the
/// part that can be stopped, so it is the part that reports itself. The
/// correlation is under a second and says only that it has started.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AlignProgressed {
    pub(crate) track_id: Id,
    pub(crate) stage: AlignStageView,
    /// How much of the film has been read, from zero to one.
    #[specta(type = Number)]
    pub(crate) fraction: f32,
}

/// Which step of an alignment is running.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AlignStageView {
    /// Decoding the film's audio to find where the talking is.
    Reading,
    /// Matching that against where the subtitle says the talking is.
    Correlating,
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
