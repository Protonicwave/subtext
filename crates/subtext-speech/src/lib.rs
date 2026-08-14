//! Reading where the speech falls in a film.
//!
//! A subtitle track claims to describe the dialogue of a film. Whether it
//! actually does, and how far out it is if it nearly does, can only be settled
//! against the film itself, and the only part of a film that says where the
//! dialogue is is the audio. This crate reads it and answers one question, bin
//! by bin: is somebody talking here.
//!
//! That is the whole of it. Nothing here identifies a word, a speaker or a
//! language, and nothing it produces is played, saved or shown. The audio is
//! decoded to be measured and thrown away as it goes, which is why a three hour
//! film costs the same memory as a three minute one.
//!
//! The answer is a [`Signal`], which is the type [`subtext_align`] correlates.
//! Handing one of these and a track's cues to that crate gives the offset and
//! the rate the track needs to sit over the film.
//!
//! # What cannot be read
//!
//! Symphonia reads AAC, MP3, FLAC, Vorbis, PCM and ALAC. It does not read AC-3,
//! `E-AC-3`, DTS or `TrueHD`, which is what a disc rip usually carries. Those films
//! come back as a [`Refusal`] naming the codec rather than as an empty signal,
//! because a film nobody can measure and a film with nobody talking in it look
//! the same afterwards and mean opposite things.
//!
//! ```
//! use subtext_speech::{Refusal, fixture::Film, speech_of};
//!
//! let film = Film::new(4_000).speaking(1_000, 3_000);
//! let dir = tempfile::tempdir().unwrap();
//!
//! let path = dir.path().join("a film.mkv");
//! std::fs::write(&path, film.matroska()).unwrap();
//! let speech = speech_of(&path).unwrap();
//! assert!(speech.active() > 0);
//!
//! // The same film, in a codec this build has no decoder for.
//! let path = dir.path().join("a disc rip.mkv");
//! std::fs::write(&path, film.claiming("A_AC3").matroska()).unwrap();
//! assert_eq!(
//!     speech_of(&path),
//!     Err(Refusal::Codec { name: Some("AC-3".to_owned()) }),
//! );
//! ```

use core::fmt;
use std::path::Path;

pub use subtext_align::{BIN_MS, Signal};

mod band;
mod codec;
pub mod fixture;
mod floor;
mod read;

/// Why a film's speech could not be read.
///
/// Not an error, and deliberately not one. Nothing has gone wrong when a film
/// has no soundtrack, or carries it in a codec this build does not read: the
/// answer to the question is that there is no answer, and what a caller needs is
/// something plain to say to somebody rather than a failure to report and log. A
/// person told that their film's audio is DTS knows that trying again will not
/// help and that nothing is broken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The film has no soundtrack at all.
    NoAudio,
    /// The soundtrack is in a codec this build has no decoder for.
    Codec {
        /// What the codec is called, where it is one anybody has a name for.
        name: Option<String>,
    },
    /// The file could not be opened, or nothing in it could be made sense of.
    Unreadable(String),
    /// The caller asked for the reading to stop before it had finished.
    Stopped,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAudio => write!(f, "the film has no soundtrack"),
            Self::Codec { name: Some(name) } => {
                write!(f, "the soundtrack is {name}, which cannot be read")
            }
            Self::Codec { name: None } => {
                write!(f, "the soundtrack is in a codec that cannot be read")
            }
            Self::Unreadable(why) => write!(f, "the film could not be read: {why}"),
            Self::Stopped => write!(f, "the reading was stopped"),
        }
    }
}

/// Whether to carry on reading a film.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    Continue,
    Stop,
}

/// Where reports of how far through a film the reading has got go.
///
/// Reading a long film is the slow part of aligning a subtitle, slow enough that
/// somebody watching has to be told it is happening and be able to change their
/// mind. Reports arrive from whichever run of a split read produced them, so
/// this has to be usable from several threads at once.
pub trait Progress: Send + Sync {
    /// How much of the film has been read, between nothing and all of it.
    ///
    /// Returning [`Reading::Stop`] ends the read, and the film comes back as
    /// [`Refusal::Stopped`].
    fn read(&self, fraction: f32) -> Reading;
}

impl<F: Fn(f32) -> Reading + Send + Sync> Progress for F {
    fn read(&self, fraction: f32) -> Reading {
        self(fraction)
    }
}

/// A listener for callers with nothing to show and nothing to stop.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unwatched;

impl Progress for Unwatched {
    fn read(&self, _fraction: f32) -> Reading {
        Reading::Continue
    }
}

/// Where somebody is talking in a film.
///
/// # Errors
///
/// Returns a [`Refusal`] naming what stopped it, which is a film with no
/// soundtrack, a codec this build cannot read, or a file that will not open.
pub fn speech_of(path: &Path) -> Result<Signal, Refusal> {
    speech_of_with(path, &Unwatched)
}

/// The same, saying how far it has got and stopping when asked.
///
/// # Errors
///
/// As [`speech_of`], and [`Refusal::Stopped`] where `progress` asked it to stop.
pub fn speech_of_with(path: &Path, progress: &dyn Progress) -> Result<Signal, Refusal> {
    let energy = read::energy(path, progress)?;
    Ok(Signal::from_bins(floor::speech(&energy.levels())))
}
