//! Working out how far a subtitle track is out of step with its film.
//!
//! A sidecar subtitle is paired to a film by the likeness of the two names,
//! which says nothing about whether the timings inside were written against
//! these frames. Where they were not, somebody can find the difference by ear,
//! nudging the track and watching a scene until it sits right. That number is
//! not a matter of taste. It is written plainly in the film: the dialogue is in
//! the audio, the cues claim to describe that dialogue, and the difference
//! between the two is arithmetic.
//!
//! This crate is the arithmetic. Two runs of bits go in, one saying where the
//! cues fall and one saying where somebody is actually talking, and a
//! [`Correction`](subtext_core::Correction) comes out with a [`Confidence`]
//! attached to it. It reads no files, decodes no audio, starts no threads and
//! knows nothing about where either signal came from, which is what lets it be
//! tested on signals made up on the spot.
//!
//! What it will not do is decide. An answer applied to a film nobody has watched
//! yet is wrong for the whole film if it is wrong at all, with no reason to
//! suspect the tool rather than the file, so the confidence is reported and the
//! caller holds the threshold.
//!
//! ```
//! use subtext_align::{Signal, align};
//! use subtext_core::{Correction, Cue, Timestamp};
//!
//! // Dialogue comes in exchanges rather than at a fixed spacing, and it is the
//! // unevenness that makes one moment in a film tell itself apart from another.
//! let mut cues: Vec<Cue> = Vec::new();
//! let mut at_ms = 30_000;
//! for at in 0..400 {
//!     cues.push(Cue {
//!         index: at + 1,
//!         start: Timestamp::from_millis(at_ms),
//!         end: Timestamp::from_millis(at_ms + 1_400),
//!         text: "line".to_owned(),
//!         position: None,
//!     });
//!     at_ms += 2_000 + (at * 977) % 4_100;
//! }
//!
//! // A film whose dialogue falls two and a half seconds after the file says.
//! let late = Correction::of_offset(2_500);
//! let spoken: Vec<Cue> = cues
//!     .iter()
//!     .map(|cue| Cue {
//!         start: late.apply(cue.start),
//!         end: late.apply(cue.end),
//!         ..cue.clone()
//!     })
//!     .collect();
//!
//! let found = align(&cues, &Signal::from_cues(&spoken));
//! assert_eq!(found.correction().offset_ms(), 2_500);
//! assert!(found.confidence().score() > 0.25);
//! ```

mod align;
mod correlate;
mod landing;
mod rate;
mod signal;

pub use crate::align::{Alignment, Confidence, align};
pub use crate::landing::{Landing, TOLERANCE_MS, landing_of};
pub use crate::rate::RATES;
pub use crate::signal::{BIN_MS, Signal};
