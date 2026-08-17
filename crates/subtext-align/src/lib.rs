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
//! The answer is arrived at by asking the film about it. A correlation over the
//! whole film narrows the field to a few explanations; then the film is cut into
//! stretches and each is asked what each explanation still leaves it needing,
//! and a line through those answers is what chooses between them. A stretch and
//! a shift trade against each other, so one peak cannot separate them and a line
//! across two dozen points can: its slope is the stretch and its intercept the
//! shift. The [`Confidence`] that comes back is the shape of that line, in
//! parts, and not the height of any peak.
//!
//! The answer comes with a second figure beside it, and the two say different
//! things. The confidence says how much of the film agrees about where its lines
//! belong. The [`Landing`] figure asks whether they then arrive when somebody
//! speaks: with this correction applied, how many lines land on the start of an
//! utterance, against how many do already? A film can agree with itself about an
//! answer that is wrong, and nothing but this can tell the two apart.
//!
//! Both figures are measured against speech, so what goes into a cue signal is
//! the lines somebody speaks in. A cue captioning a door slamming marks a moment
//! of no speech, and a track full of them would be measured against a reading
//! that was never going to find any. Those cues are left out of the signal here
//! and out of nothing else: what is drawn and what is stored are the file's own
//! cues.
//!
//! There are also files no correction fits, and there are two of them. A film
//! recorded off a broadcast has time inserted into it, so its lines are all
//! there and one number cannot place them all. A subtitle for a different cut
//! describes scenes this film does not have, and no number places those at all.
//! Both score well, because most of either film really does line up, so both are
//! looked for in the stretches of the film rather than in the answer, and each is
//! named as what it is: see [`Misfit`].
//!
//! What it will not do is decide. An answer applied to a film nobody has watched
//! yet is wrong for the whole film if it is wrong at all, with no reason to
//! suspect the tool rather than the file, so both figures are reported and the
//! caller holds the bar.
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
//!
//! // And the lines land where the film talks, which they did not before.
//! assert!(found.landing().fraction() > found.as_written().fraction());
//! ```

mod align;
mod correlate;
mod landing;
mod local;
mod misfit;
mod rate;
mod signal;
mod spoken;

pub use crate::align::{Alignment, Confidence, align};
pub use crate::landing::{LATE_TOLERANCE_MS, LEAD_TOLERANCE_MS, Landing, landing_of};
pub use crate::misfit::Misfit;
pub use crate::rate::RATES;
pub use crate::signal::{BIN_MS, Signal};
