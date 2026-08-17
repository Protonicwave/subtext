//! Putting a subtitle track where the dialogue in its film actually is.
//!
//! Coordination and nothing else. Finding something that says where the dialogue
//! falls, reading the authored cues, asking `subtext-align` what would explain
//! one in terms of the other, and writing the answer down are four things that
//! happen in an order, with no arithmetic of their own. The order is the only
//! judgement here, and it is to refuse cheaply: a track nothing could be made of
//! is turned away before a byte of audio is read.
//!
//! There are two things in a film that say where its dialogue falls, and the
//! cheaper one is also the better one. Where the film carries another text
//! subtitle track, that track was written against these frames, so the
//! comparison is authored timings on both sides: no decoder, no detector, and an
//! answer accurate to the bin in a tenth of a second. Where it does not, the
//! soundtrack is read, which takes seconds and produces an estimate of where
//! somebody is talking. The exact path is tried first and the audio is the
//! fallback, not the default.
//!
//! What is not coordination is deciding. Whether a measurement is worth writing
//! to somebody's library is a judgement about the product rather than a property
//! of the correlation, so the engine reports its figures and this weighs them.
//! There are two of those judgements and they ask different questions. The
//! threshold asks how much of the film agrees about the answer. The bar asks
//! whether the answer then puts the lines where the dialogue is, which is the
//! one question no measurement can mark its own work on: a film can agree with
//! itself about an answer that is wrong, and does whenever a subtitle belongs to
//! a different cut of the same picture.

use subtext_align::{Landing, Signal};
use subtext_container::SubtitleCodec;
use subtext_core::Cue;
use subtext_index::{Database, TrackOrigin, TrackRecord};
use subtext_speech::{Progress, Refusal};

use crate::dto::{AlignmentView, Answer, Failure, ReferenceView};

/// How sure the measurement has to be before a correction is written.
///
/// Nudging by ear has a floor under it: somebody watching can hear that a
/// change made it worse and press the other key. An answer arrived at by
/// measurement has no such floor, because it is applied to a film nobody has
/// watched yet, and if it is wrong it is wrong for the whole film with no
/// reason to suspect the tool rather than the file. So the number is set where
/// a wrong answer is refused rather than where the most files are helped.
///
/// Set from a run over a real library: nine films, a hundred and sixty one
/// manufactured cases. Every one of the eight cases pairing one film's subtitle
/// with another film's audio scored exactly nought, because the confidence is
/// now three figures multiplied together and a wrong pairing fails all three at
/// once. The lowest correct answer the bar below would let through scored 0.26,
/// and the middle one 0.49. So there is a great deal of room and nothing in it,
/// and this sits a factor of two and a half under the lowest correct answer with
/// nothing at all beneath it.
///
/// It is deliberately not the number that decides the hard cases. A subtitle for
/// a different cut of the same film correlates strongly, because most of the
/// film really does line up, and refusing it is the bar's work rather than this
/// one's. Pushing this up until it caught those would put it at 0.30, a
/// hundredth above the strongest of them, and would refuse a whole film in the
/// run whose answers were all correct and all scored 0.27. A number placed on
/// the edge of one run's observations is how the trouble in this part began.
///
/// Erring low would be worse than erring high. Refusing a file that could have
/// been helped leaves somebody exactly where they were, with the keys they
/// already had and a sentence saying why. Accepting one that could not leaves
/// them watching a film that is wrong from beginning to end.
const THRESHOLD: f32 = 0.10;

/// How sure a measurement against another subtitle track has to be.
///
/// A separate number from [`THRESHOLD`] rather than the same one, because the
/// two are read off correlations of different kinds and the whole scale moves
/// between them. Against a speech reading, one side is an estimate: music and
/// effects mark bins nobody spoke in, whispers and lines under a loud mix go
/// unmarked, and stretches of the film disagree about the answer for reasons
/// that have nothing to do with the answer. Against another subtitle track
/// nothing is estimated and every stretch of the film agrees exactly.
///
/// The corpus measures both ends of this regime. On the one film in the run
/// carrying a text track of its own, all seventeen correct answers scored
/// exactly one, another film's subtitle scored nought, and a subtitle for a
/// different cut of the same film scored 0.625. That last figure is the share of
/// the film a single correction explains, and on this path it is read straight:
/// with nothing estimated on either side, a film that agrees with an answer
/// about five eighths of itself is a film the answer does not describe.
///
/// Four fifths. It sits under every correct answer by a fifth of the scale and
/// well above the one wrong one, and unlike the audio path this number rather
/// than the bar is what refuses a file with breaks in it, because here it can.
///
/// Erring high costs less here than anywhere else in this file. A reference that
/// does not settle the question is not reported as a refusal at all: the film
/// itself is still there to be asked, and it is asked. So the price of turning
/// away a reference that would have been right is a soundtrack read that could
/// have been skipped, which is seconds of somebody's afternoon rather than a
/// film that plays wrongly from beginning to end.
const REFERENCE_THRESHOLD: f32 = 0.80;

/// How much of a track has to land on the dialogue before anything is written.
///
/// The confidence above is read off the same peak the correction is, so a
/// measurement that is confidently wrong has nothing left to catch it. This is
/// what catches it: with the correction applied, the share of lines that arrive
/// within a quarter of a second of somebody starting to speak. It is arrived at
/// independently of the estimator, so the trade a rate and an offset make
/// against each other cannot flatter it.
///
/// Two conditions rather than one, because the figure is worth more as a
/// comparison than as a level. A measurement that would put fewer lines on the
/// talking than the file already does is refused whatever it scores, since
/// making a film worse is the one outcome there is no argument for. And a
/// measurement that clears that and still lands under this bar is refused as
/// well, because both readings being poor means the pairing is wrong rather than
/// the timing.
///
/// Set from a run over a real library of nine films and a hundred and sixty one
/// manufactured cases, and it is this number rather than the threshold that
/// decides the hard cases. Correct answers landed between 63 and 76 per cent of
/// a track, with a middle of 70. A subtitle for a different cut of the same
/// film, which is time cut into the middle of a picture and no single correction
/// can answer, landed between 42 and 55. A subtitle for another film altogether
/// landed between 21 and 29, which is what chance gives.
///
/// So this sits three points above the highest of the wrong answers and five
/// below the lowest of the right ones. At that value nothing in the run was
/// written wrongly, and nothing anywhere in the swept range from 55 to 60 per
/// cent changes a single verdict, so it is not balanced on an observation.
///
/// It costs something and the cost is worth stating. Two of the nine films have
/// subtitles that land on 53 per cent of their own talking untouched, so no
/// correction to either could ever clear this and neither film can be helped.
/// That is the price of leaving this number to catch a file with breaks in it,
/// which is a job it is doing on someone else's behalf. Once such a file is
/// refused on its own account, this can come down to where it belongs, which is
/// somewhere near the two fifths that chance and a correct answer sit either
/// side of.
///
/// It errs the same way the threshold does, and for the same reason. Refusing a
/// file that could have been helped leaves somebody where they were, with the
/// keys they already had and a sentence saying why.
///
/// One number rather than two, unlike the threshold above. Where the reference
/// is another subtitle track this asks how many lines arrive when the reference
/// says somebody speaks, which is a stricter question than the one it asks of a
/// speech reading and which a right answer clears by further still: the same run
/// put a correct answer against a film's own track at 81 per cent. A floor both
/// regimes are well clear of does not need to be set twice.
const BAR: f32 = 0.58;

/// How many lines a track needs before it is worth measuring at all.
///
/// A film's dialogue runs to several hundred lines at the least and usually to
/// a couple of thousand. A forced or signs-only track runs to a few dozen: it
/// captions the signs and the foreign sentences in a film somebody otherwise
/// understands, and there is not enough of it to tell one lag from another. A
/// hundred sits between the two with room on both sides, and the check is
/// against the count on the track's own row, so a track like that is turned
/// away in a millisecond rather than after a film has been decoded.
const FEWEST_CUES: usize = 100;

/// Lines a subtitle track up with its film, and writes the answer if it is
/// worth writing.
///
/// # Errors
///
/// Only where the library itself cannot be read. A film that cannot be measured
/// is not a failure: it comes back as one of the outcomes, with what stopped it
/// named, because there is something to say to somebody about every one of them
/// and nothing to log.
pub(crate) fn run(
    database: &Database,
    track_id: i64,
    progress: &dyn Progress,
) -> Answer<AlignmentView> {
    let track = database
        .tracks()
        .by_id(track_id)
        .map_err(Failure::of)?
        .ok_or_else(|| Failure::saying("that subtitle is no longer in the library"))?;

    // First, because it costs one row that has already been read. Everything
    // below this opens a file.
    if track.cue_count < FEWEST_CUES {
        return Ok(AlignmentView::TooFewCues {
            cues: u32::try_from(track.cue_count).unwrap_or(u32::MAX),
            wanted: u32::try_from(FEWEST_CUES).unwrap_or(u32::MAX),
        });
    }

    let film = database
        .films()
        .by_id(track.film_id)
        .map_err(Failure::of)?
        .ok_or_else(|| Failure::saying("that film is no longer in the library"))?;

    // The exact path first, where the film carries something to be exact
    // against. It reads two tracks and decodes nothing, so trying it and falling
    // through costs a fraction of the reading it would otherwise go straight to.
    let mut authored = None;
    if let Some(reference) = reference_for(database, &track)? {
        let cues = database
            .tracks()
            .authored_cues(track.id)
            .map_err(Failure::of)?;
        if let Some(settled) = against(database, &track, &cues, &reference)? {
            return Ok(settled);
        }
        // The reference did not settle it, and that is not a refusal to report:
        // it says the two tracks disagree without saying which of them is wrong,
        // and the film is the authority on that. The cues are the same either
        // way, so they are carried down rather than read twice.
        authored = Some(cues);
    }

    // Before the cues, where they have not already been read, so that a film
    // whose audio cannot be read costs the header of one file rather than the
    // whole of a transcript as well.
    let speech = match subtext_speech::speech_of_with(&film.path, progress) {
        Ok(speech) => speech,
        Err(refusal) => return Ok(refused(refusal)),
    };

    // As the file wrote them. Corrected timings would measure this track
    // against its own last answer rather than against the film.
    let cues = match authored {
        Some(cues) => cues,
        None => database
            .tracks()
            .authored_cues(track.id)
            .map_err(Failure::of)?,
    };

    let found = subtext_align::align(&cues, &speech);
    let confidence = found.confidence().score();
    if confidence < THRESHOLD {
        return Ok(AlignmentView::uncertain(&found, confidence, THRESHOLD));
    }

    // A film one correction cannot describe, before the figure that would refuse
    // it anyway. Both of these shapes are already turned away by the bar below,
    // since a file that is right for half of a film does not land enough of it,
    // and being turned away is not the same as being told why. A recording with
    // breaks in it and a subtitle for another cut are different files with
    // different remedies, and somebody who is told which they have can act on it.
    //
    // After the threshold rather than before it, because a subtitle belonging to
    // another film altogether also goes unexplained everywhere, and that is the
    // commonest wrong pairing there is. It has its own wording, which says the
    // ordinary thing about a file that does not match, and it should keep it.
    if let Some(misfit) = found.misfit() {
        return Ok(AlignmentView::misfit(&found, misfit));
    }

    // The answer is clear. Whether it is any good is a different question, and
    // it is asked of the film rather than of the correlation that produced it.
    if !worth_writing(found.landing(), found.as_written()) {
        return Ok(AlignmentView::no_better(&found, BAR));
    }

    database
        .tracks()
        .set_correction(track.id, found.correction())
        .map_err(Failure::of)?;

    Ok(AlignmentView::aligned(
        &found,
        track.correction,
        confidence,
        ReferenceView::Speech,
    ))
}

/// Another text track of the same film, standing in for its speech.
///
/// The signal is built the way a cue signal always is, so everything downstream
/// of it, the correlation and the landing figure alike, is the same code
/// measuring the same shapes. What has changed is only what the shapes came
/// from.
struct Reference {
    dialogue: Signal,
    language: Option<String>,
    inside: bool,
}

impl Reference {
    fn named(&self) -> ReferenceView {
        ReferenceView::Track {
            language: self.language.clone(),
            inside: self.inside,
        }
    }
}

/// The best track of this film to measure another one against, where there is
/// one.
///
/// Three conditions and then an order. It has to be text, since a track of
/// pictures carries nothing to read and no amount of correlation will change
/// that. It has to be a different track, since measuring a file against itself
/// answers the identity every time. And it has to carry enough lines for the
/// same reason the track being measured does: a few dozen cannot tell one moment
/// of a film from another, whichever side of the comparison they sit on.
///
/// Of what is left, a track from inside the film comes first. It was muxed by
/// whoever produced this encode and is therefore on these frames' own clock,
/// where a file beside the film is a pairing made by name and is exactly the
/// kind of thing this whole feature exists to put right. Then the fullest, since
/// more lines is more to line up on. Then the lowest identifier, so that a film
/// with two equally good candidates chooses the same one twice.
fn reference_for(database: &Database, track: &TrackRecord) -> Result<Option<Reference>, Failure> {
    let mut candidates: Vec<TrackRecord> = database
        .tracks()
        .for_film(track.film_id)
        .map_err(Failure::of)?
        .into_iter()
        .filter(|other| other.id != track.id)
        .filter(|other| SubtitleCodec::from_stored(&other.codec).is_text())
        .filter(|other| other.cue_count >= FEWEST_CUES)
        .collect();

    candidates.sort_by(|one, other| {
        inside(other)
            .cmp(&inside(one))
            .then(other.cue_count.cmp(&one.cue_count))
            .then(one.id.cmp(&other.id))
    });

    let Some(chosen) = candidates.first() else {
        return Ok(None);
    };

    // Corrected, which is the opposite of how the track being measured is read
    // and is right for the same reason. What a reference has to say is where the
    // dialogue falls in the film as it plays, and a correction is precisely the
    // arithmetic that turns one into the other. A track from inside the film has
    // none and takes the identity, which is what makes it the better reference.
    let cues = database.tracks().cues(chosen.id).map_err(Failure::of)?;
    Ok(Some(Reference {
        dialogue: Signal::from_cues(&cues),
        language: chosen.language.clone(),
        inside: inside(chosen),
    }))
}

fn inside(track: &TrackRecord) -> bool {
    matches!(track.origin, TrackOrigin::Stream)
}

/// The measurement against a reference, where it settled the question.
///
/// Nothing where it did not, rather than an outcome saying so. A reference that
/// disagrees with the track being measured has said that the two do not describe
/// the same moments, and it has not said which of them is at fault: the
/// reference may itself be a mispaired file, or for a different cut. Reporting
/// that to somebody as a refusal would be reporting a doubt about a track they
/// did not ask about, while the film that could settle it went unread.
fn against(
    database: &Database,
    track: &TrackRecord,
    cues: &[Cue],
    reference: &Reference,
) -> Result<Option<AlignmentView>, Failure> {
    let found = subtext_align::align(cues, &reference.dialogue);
    let confidence = found.confidence().score();
    if confidence < REFERENCE_THRESHOLD || !worth_writing(found.landing(), found.as_written()) {
        return Ok(None);
    }
    // A film the reference says one correction cannot describe is set aside for
    // the soundtrack rather than reported, which is what everything else on this
    // path does with a reference that did not settle the question. The reference
    // may be the file with breaks in it, or the one for another cut, and it has
    // not said which of the two tracks is at fault. The film can.
    if found.misfit().is_some() {
        return Ok(None);
    }

    database
        .tracks()
        .set_correction(track.id, found.correction())
        .map_err(Failure::of)?;

    Ok(Some(AlignmentView::aligned(
        &found,
        track.correction,
        confidence,
        reference.named(),
    )))
}

/// Whether a measurement earns the right to be written over what is there.
///
/// Separated out because it is the whole of the judgement in [`BAR`] and it can
/// then be checked at its edges without a film, a database or a decoder.
fn worth_writing(found: Landing, as_written: Landing) -> bool {
    // Nothing was measured, so there is no evidence either way, and evidence is
    // what this is for. A film with no speech in it and a track with no lines
    // inside the film both arrive here.
    if !found.is_measured() {
        return false;
    }
    // Equal rather than better, because a track that already lands is answered
    // with the identity, and refusing to write nothing over nothing would report
    // a failure to somebody whose file was right all along.
    found.fraction() >= as_written.fraction() && found.fraction() >= BAR
}

/// A refusal from the reading, as an ending the front end can put words to.
fn refused(refusal: Refusal) -> AlignmentView {
    match refusal {
        Refusal::NoAudio => AlignmentView::NoAudio,
        Refusal::Codec { name } => AlignmentView::Unsupported { codec: name },
        Refusal::Unreadable(why) => AlignmentView::Unreadable { message: why },
        Refusal::Stopped => AlignmentView::Stopped,
    }
}

#[cfg(test)]
mod tests {
    // A test that cannot build the library it is about to measure has nothing
    // to say, so it stops rather than passing quietly.
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use subtext_align::{BIN_MS, Landing, Signal, landing_of};
    use subtext_core::{Correction, Cue, SubtitleLabel, Timestamp};
    use subtext_index::{Database, NewFilm, NewTrack, TrackMatch, TrackOrigin};
    use subtext_speech::fixture::Film;
    use subtext_speech::{Reading, Unwatched};
    use tempfile::TempDir;

    use super::{BAR, FEWEST_CUES, THRESHOLD, run, worth_writing};
    use crate::dto::{AlignmentView, ReferenceView};

    /// How long the film the measurements are made against runs for.
    ///
    /// Twelve minutes, which is the shortest that holds comfortably more than
    /// [`FEWEST_CUES`] lines at the spacing dialogue actually has. Written at
    /// eight kilohertz in mono, since what is being measured here is the
    /// coordination rather than the decoding, and a film mixed the way a real
    /// one is would be a hundred megabytes of temporary file per test.
    const LENGTH_MS: u32 = 720_000;

    /// How far the film's dialogue falls after the subtitle claims it does.
    const TRUTH: i32 = 3_500;

    /// How far out the answer may be and still be an answer.
    ///
    /// Fifty milliseconds, which is under two frames and below what anybody
    /// watching can tell.
    const SLACK: i64 = 50;

    /// A film's worth of dialogue, in exchanges rather than at a fixed spacing.
    ///
    /// The unevenness is the point. It is what makes one moment in a film tell
    /// itself apart from another, and a transcript of evenly spaced lines would
    /// line up equally well almost anywhere. What the lines say is of no
    /// interest to any of this: the measurement is of when somebody speaks.
    ///
    /// Uneven over the whole film rather than in a pattern that comes round
    /// again. Spacing a line by the index modulo a small number reads as uneven
    /// and repeats every few lines, which puts a moment of this film a minute
    /// from a moment that looks exactly like it, and a film like that has two
    /// answers rather than one. Real dialogue does not do that, and a fixture
    /// that does would be testing the wrong thing.
    fn dialogue() -> Vec<Cue> {
        let mut cues = Vec::new();
        let mut at_ms = 20_000;
        while at_ms < LENGTH_MS - 20_000 {
            let index = u32::try_from(cues.len()).unwrap();
            let length = 1_000 + (index * 613) % 2_000;
            cues.push(Cue {
                index: index + 1,
                start: Timestamp::from_millis(at_ms),
                end: Timestamp::from_millis(at_ms + length),
                text: "line".to_owned(),
                position: None,
            });
            at_ms += length + 700 + (index * 977) % 4_100;
        }
        cues
    }

    /// A different film's transcript, at the same length and to the same
    /// general shape: exchanges, pauses, lines of a second or three. Nothing
    /// about it says it belongs elsewhere except the one thing that matters,
    /// which is where the lines fall.
    fn another_film() -> Vec<Cue> {
        let mut cues = Vec::new();
        let mut at_ms = 46_000;
        while at_ms < LENGTH_MS - 20_000 {
            let index = u32::try_from(cues.len()).unwrap();
            let length = 1_300 + (index % 3) * 900;
            cues.push(Cue {
                index: index + 1,
                start: Timestamp::from_millis(at_ms),
                end: Timestamp::from_millis(at_ms + length),
                text: "line".to_owned(),
                position: None,
            });
            at_ms += length + 900 + (index % 17) * 330;
        }
        cues
    }

    /// The dialogue above where the film actually says it, which is
    /// [`TRUTH`] milliseconds after the subtitle claims.
    ///
    /// What a track written against these frames holds, and therefore what a
    /// reference has to say to be any use.
    fn spoken() -> Vec<Cue> {
        let late = Correction::of_offset(TRUTH);
        dialogue()
            .iter()
            .map(|cue| Cue {
                start: late.apply(cue.start),
                end: late.apply(cue.end),
                ..cue.clone()
            })
            .collect()
    }

    /// The dialogue above with the sounds of the film captioned alongside it,
    /// which is what a track written for the hearing impaired holds.
    ///
    /// The descriptions sit in the quiet, because that is when a door slams and
    /// a phone rings, and it is where a reading looking for voices finds
    /// nothing.
    fn captioned_cues() -> Vec<Cue> {
        let spoken = dialogue();
        let mut cues = spoken.clone();
        for pair in spoken.windows(2) {
            let quiet = pair[1].start.millis().saturating_sub(pair[0].end.millis());
            if quiet < 1_200 {
                continue;
            }
            let at = pair[0].end.millis() + quiet / 3;
            cues.push(Cue {
                index: 0,
                start: Timestamp::from_millis(at),
                end: Timestamp::from_millis(at + 900),
                text: "[DOOR SLAMS]".to_owned(),
                position: None,
            });
        }

        cues.sort_by_key(|cue| cue.start);
        for (at, cue) in cues.iter_mut().enumerate() {
            cue.index = u32::try_from(at).unwrap() + 1;
        }
        cues
    }

    /// The film's soundtrack, talking where `spoken` says it talks.
    ///
    /// Built once and kept, because laying out six million samples costs more
    /// than everything else in these tests put together and every one of them
    /// wants the same film.
    fn soundtrack(spoken: &[Cue]) -> &'static [u8] {
        static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
        BYTES.get_or_init(|| {
            let mut film = Film::new(LENGTH_MS).recorded(8_000, 1);
            for cue in spoken {
                film = film.speaking(cue.start.millis(), cue.end.millis());
            }
            film.matroska()
        })
    }

    /// How far into the film a break is cut, and how much it inserts.
    ///
    /// A quarter of a minute at two fifths of the way through, which is what an
    /// advertisement break in a recorded broadcast does. Further than a stretch of
    /// film is searched by at first and well inside the second, wider search, so
    /// the film after the break reports where it is rather than reporting nothing.
    /// That is the difference this fixture exists to exercise.
    const BREAK_AT: u32 = 288_000;
    const BREAK_MS: u32 = 15_000;

    /// The scene a different cut of the film does not have, and where it sits.
    ///
    /// Three minutes, which is further than any stretch is searched by at all. The
    /// film talks all the way through it and the subtitle has no lines for any of
    /// it, which is what a subtitle for a different cut looks like.
    const SCENE_AT: u32 = 400_000;
    const SCENE_MS: u32 = 180_000;

    /// The film as a recording off a broadcast has it, with a break cut in.
    fn broadcast(spoken: &[Cue]) -> &'static [u8] {
        static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
        BYTES.get_or_init(|| {
            let moved = |at: u32| if at < BREAK_AT { at } else { at + BREAK_MS };
            let mut film = Film::new(LENGTH_MS).recorded(8_000, 1);
            for cue in spoken {
                film = film.speaking(moved(cue.start.millis()), moved(cue.end.millis()));
            }
            film.matroska()
        })
    }

    /// The film talking exactly where its subtitle says, for the cases where what
    /// is wrong is the subtitle's cut rather than its timing.
    fn as_written(spoken: &[Cue]) -> &'static [u8] {
        static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
        BYTES.get_or_init(|| {
            let mut film = Film::new(LENGTH_MS).recorded(8_000, 1);
            for cue in spoken {
                film = film.speaking(cue.start.millis(), cue.end.millis());
            }
            film.matroska()
        })
    }

    /// A subtitle for a cut of the film that does not have one of its scenes: no
    /// lines for it, and everything after it three minutes earlier.
    fn a_shorter_cut() -> Vec<Cue> {
        dialogue()
            .iter()
            .filter(|cue| {
                cue.start.millis() < SCENE_AT || cue.start.millis() >= SCENE_AT + SCENE_MS
            })
            .map(|cue| {
                let moved = |at: u32| {
                    if at < SCENE_AT {
                        at
                    } else {
                        at.saturating_sub(SCENE_MS)
                    }
                };
                Cue {
                    start: Timestamp::from_millis(moved(cue.start.millis())),
                    end: Timestamp::from_millis(moved(cue.end.millis())),
                    ..cue.clone()
                }
            })
            .collect()
    }

    /// A library holding one film and one subtitle track for it.
    struct Fixture {
        database: Database,
        film_id: i64,
        track_id: i64,
        root: PathBuf,
        // Held so that the directory outlasts the database inside it.
        _directory: TempDir,
    }

    impl Fixture {
        /// A film written from `film`, with `cues` claiming to describe it.
        fn new(film: &Film, cues: &[Cue]) -> Self {
            Self::written(&film.matroska(), cues)
        }

        fn written(film: &[u8], cues: &[Cue]) -> Self {
            let directory = TempDir::new().unwrap();
            let root = directory.path().to_path_buf();
            let database = Database::open(root.join("library.db")).unwrap();

            let path = root.join("Heat.mkv");
            std::fs::write(&path, film).unwrap();
            let folder = database.folders().add(&root).unwrap();
            let film_id = database
                .films()
                .upsert(&NewFilm {
                    folder_id: folder.id,
                    path: &path,
                    title: "Heat",
                    year: Some(1_995),
                    size_bytes: film.len() as u64,
                    modified_at: 1_700_000_000_000,
                })
                .unwrap()
                .id;

            let track_id = database
                .tracks()
                .upsert(&NewTrack {
                    film_id,
                    path: &root.join("Heat.srt"),
                    label: SubtitleLabel {
                        language: Some("en"),
                        forced: false,
                        hearing_impaired: false,
                    },
                    origin: TrackOrigin::Sidecar,
                    stream_number: 0,
                    codec: "subrip",
                    match_kind: TrackMatch::Exact,
                    encoding: "UTF-8",
                    size_bytes: 60_000,
                    modified_at: 1_700_000_000_000,
                })
                .unwrap()
                .id;
            database.tracks().replace_cues(track_id, cues).unwrap();

            Self {
                database,
                film_id,
                track_id,
                root,
                _directory: directory,
            }
        }

        /// A film whose dialogue falls [`TRUTH`] milliseconds after its
        /// subtitle says it does.
        fn mistimed() -> Self {
            Self::written(soundtrack(&spoken()), &dialogue())
        }

        /// The same film with the same timings, captioned for somebody who
        /// cannot hear it: every line of the dialogue, and a description of the
        /// sounds in the quiet between them.
        fn captioned() -> Self {
            Self::written(soundtrack(&spoken()), &captioned_cues())
        }

        /// Another subtitle track for the same film, of the kind a reference is
        /// chosen from.
        ///
        /// A track inside the film shares the film's own path and is told apart
        /// by its stream number, which is how one is stored, so the number is
        /// what keeps these from colliding whichever kind they are.
        fn add_track(
            &self,
            origin: TrackOrigin,
            stream_number: u64,
            language: &'static str,
            codec: &str,
            cues: &[Cue],
        ) -> i64 {
            let path = match origin {
                TrackOrigin::Stream => self.root.join("Heat.mkv"),
                TrackOrigin::Sidecar => self.root.join(format!("Heat.{language}.srt")),
            };
            let id = self
                .database
                .tracks()
                .upsert(&NewTrack {
                    film_id: self.film_id,
                    path: &path,
                    label: SubtitleLabel {
                        language: Some(language),
                        forced: false,
                        hearing_impaired: false,
                    },
                    origin,
                    stream_number,
                    codec,
                    match_kind: TrackMatch::Exact,
                    encoding: "UTF-8",
                    size_bytes: 60_000,
                    modified_at: 1_700_000_000_000,
                })
                .unwrap()
                .id;
            self.database.tracks().replace_cues(id, cues).unwrap();
            id
        }

        /// Takes the film off the disk.
        ///
        /// What the reference tests are built on. Anything that opened the film
        /// would come back unreadable, so an answer at all is proof that no part
        /// of it was read.
        fn take_the_film_away(&self) {
            std::fs::remove_file(self.root.join("Heat.mkv")).unwrap();
        }

        /// A recording of the film off a broadcast, with a break cut into the
        /// middle of it. Every part of it says where its own lines are and the two
        /// parts disagree, which is a file one correction cannot describe.
        fn with_breaks() -> Self {
            Self::written(broadcast(&dialogue()), &dialogue())
        }

        /// The film, with a subtitle written for a shorter cut of it.
        fn for_another_cut() -> Self {
            Self::written(as_written(&dialogue()), &a_shorter_cut())
        }

        /// A film that speaks only the first third of what its subtitle claims,
        /// and is quiet through the rest. What a subtitle for a different cut
        /// looks like, and the shape a correlation can be perfectly clear about
        /// while being no use at all.
        fn mostly_unspoken() -> Self {
            let cues = dialogue();
            let late = Correction::of_offset(TRUTH);
            let mut film = Film::new(LENGTH_MS).recorded(8_000, 1);
            for cue in cues.iter().take(cues.len() / 3) {
                film = film.speaking(late.apply(cue.start).millis(), late.apply(cue.end).millis());
            }
            Self::written(&film.matroska(), &cues)
        }

        fn align(&self) -> AlignmentView {
            run(&self.database, self.track_id, &Unwatched).unwrap()
        }

        fn correction(&self) -> Correction {
            self.database
                .tracks()
                .by_id(self.track_id)
                .unwrap()
                .expect("the track should still be there")
                .correction
        }

        /// Opens the library file again, as a restart of the application would.
        fn reopen(&self) -> Database {
            Database::open(self.root.join("library.db")).unwrap()
        }
    }

    /// How far the corrected timings sit from where the film actually talks, at
    /// the worst line in it.
    fn residual(correction: Correction) -> i64 {
        let truth = Correction::of_offset(TRUTH);
        dialogue()
            .iter()
            .map(|cue| {
                i64::from(correction.apply(cue.start).millis())
                    - i64::from(truth.apply(cue.start).millis())
            })
            .map(i64::abs)
            .max()
            .unwrap_or(0)
    }

    /// A track of ten lines, `landed` of which arrive as somebody starts
    /// talking, measured through the same code an alignment measures with.
    ///
    /// Built rather than described, because a figure is only worth deciding on
    /// if it came from a film, and the decision below is what these tests are
    /// about.
    fn measured(landed: u32) -> Landing {
        // A film that says a second's worth of something every ten seconds.
        let mut bins = vec![false; 10 * 10 * 100];
        for at in 0..10 {
            bins[at * 1_000..at * 1_000 + 100].fill(true);
        }
        let speech = Signal::from_bins(bins);

        // Lines on those moments, and the rest of them in the quiet between.
        let cues: Vec<Cue> = (0..10_u32)
            .map(|at| {
                let start = at * 10_000 + if at < landed { 0 } else { 5_000 };
                Cue {
                    index: at + 1,
                    start: Timestamp::from_millis(start),
                    end: Timestamp::from_millis(start + 900),
                    text: "line".to_owned(),
                    position: None,
                }
            })
            .collect();

        landing_of(&cues, &speech, Correction::IDENTITY)
    }

    #[test]
    fn a_film_that_is_out_by_a_known_amount_is_put_right() {
        let fixture = Fixture::mistimed();

        let outcome = fixture.align();
        let AlignmentView::Aligned {
            previous,
            confidence,
            landing,
            as_written,
            reference,
            ..
        } = outcome
        else {
            panic!("a film measured against its own subtitle should line up: {outcome:?}");
        };

        assert_eq!(previous.offset_ms, 0);
        assert!(confidence >= THRESHOLD, "only {confidence} sure");
        // A film carrying nothing else to be measured against is measured
        // against its own soundtrack, which is what every film could do before
        // there was a second way and what most films still do.
        assert!(matches!(reference, ReferenceView::Speech));

        // And the evidence for it, which is the part that did not come out of
        // the correlation that produced the answer.
        assert!(landing.fraction > as_written.fraction);
        assert!(landing.fraction >= BAR, "only {} landed", landing.fraction);
        assert!(landing.examined > 0);

        let error = residual(fixture.correction());
        assert!(error <= SLACK, "out by {error}ms");
    }

    /// The exact path, and the whole reason it is worth having. A film that
    /// carries its own text subtitle track carries timings written against these
    /// frames, so the comparison is authored timings on both sides: no decoder,
    /// no detector, and an answer good to the bin.
    ///
    /// The film is taken off the disk first, so that reading a single byte of it
    /// would end this as unreadable. An answer is therefore proof that none was
    /// read, which is the claim in section 24.3 and the one the timing figure
    /// rests on.
    #[test]
    fn a_film_that_carries_its_own_timings_is_lined_up_without_being_listened_to() {
        let fixture = Fixture::mistimed();
        fixture.add_track(TrackOrigin::Stream, 1, "en", "subrip", &spoken());
        fixture.take_the_film_away();

        let outcome = fixture.align();
        let AlignmentView::Aligned { reference, .. } = &outcome else {
            panic!("a track measured against the film's own timings should line up: {outcome:?}");
        };
        assert!(matches!(
            reference,
            ReferenceView::Track {
                inside: true,
                language: Some(name),
            } if name == "en"
        ));

        // To the bin rather than to the fifty milliseconds the audio path is
        // held to. Nothing here was estimated.
        let error = residual(fixture.correction());
        assert!(error <= i64::from(BIN_MS), "out by {error}ms");
    }

    /// A film with several tracks to choose between. The one inside the film
    /// comes first, because it was muxed against this encode rather than paired
    /// to it by name, and among those the fullest wins, because more lines is
    /// more to line up on.
    ///
    /// All three describe the film correctly, so nothing but the order decides
    /// this, and the language is what says which was taken.
    #[test]
    fn the_fullest_track_inside_the_film_is_the_one_measured_against() {
        let fixture = Fixture::mistimed();
        let spoken = spoken();
        fixture.add_track(TrackOrigin::Sidecar, 0, "en", "subrip", &spoken);
        fixture.add_track(TrackOrigin::Stream, 1, "fr", "subrip", &spoken[..120]);
        fixture.add_track(TrackOrigin::Stream, 2, "de", "subrip", &spoken);
        fixture.take_the_film_away();

        let outcome = fixture.align();
        let AlignmentView::Aligned { reference, .. } = &outcome else {
            panic!("a film with three references should use one of them: {outcome:?}");
        };
        assert!(matches!(
            reference,
            ReferenceView::Track {
                inside: true,
                language: Some(name),
            } if name == "de"
        ));
    }

    /// A track of pictures carries nothing to read, and no amount of correlation
    /// changes that. It is given cues it could never have, so that what turns it
    /// down is plainly its codec rather than an empty track.
    #[test]
    fn a_film_whose_only_other_track_is_pictures_is_measured_against_its_audio() {
        let fixture = Fixture::mistimed();
        fixture.add_track(TrackOrigin::Stream, 1, "en", "pgs", &spoken());

        let outcome = fixture.align();
        let AlignmentView::Aligned { reference, .. } = &outcome else {
            panic!("a film with no readable reference should fall back to its audio: {outcome:?}");
        };
        assert!(matches!(reference, ReferenceView::Speech));
        assert!(residual(fixture.correction()) <= SLACK);
    }

    /// A forced or signs-only track is no better as a reference than it is as a
    /// track to be measured, and for the same reason: a few dozen lines cannot
    /// tell one moment of a film from another whichever side they sit on.
    #[test]
    fn a_reference_with_too_few_lines_is_not_used() {
        let fixture = Fixture::mistimed();
        let spoken = spoken();
        fixture.add_track(
            TrackOrigin::Stream,
            1,
            "en",
            "subrip",
            &spoken[..FEWEST_CUES - 1],
        );

        let outcome = fixture.align();
        let AlignmentView::Aligned { reference, .. } = &outcome else {
            panic!("a reference too short to use should leave the audio to it: {outcome:?}");
        };
        assert!(matches!(reference, ReferenceView::Speech));
        assert!(residual(fixture.correction()) <= SLACK);
    }

    /// A reference that disagrees has said the two tracks do not describe the
    /// same moments, and it has not said which of them is at fault. Reporting
    /// that as a refusal would be reporting a doubt about a track nobody asked
    /// about while the film that settles it went unread, so the film is read.
    #[test]
    fn a_reference_that_does_not_describe_the_film_is_set_aside_for_the_audio() {
        let fixture = Fixture::mistimed();
        fixture.add_track(TrackOrigin::Stream, 1, "en", "subrip", &another_film());

        let outcome = fixture.align();
        let AlignmentView::Aligned { reference, .. } = &outcome else {
            panic!("a reference that disagrees should leave the audio to it: {outcome:?}");
        };
        assert!(matches!(reference, ReferenceView::Speech));
        assert!(residual(fixture.correction()) <= SLACK);
    }

    /// A reference is read as the film plays it, which is the opposite of how
    /// the track being measured is read and is right for the same reason. What a
    /// reference has to say is where the dialogue falls in the film, and a
    /// correction is exactly the arithmetic that turns an authored timing into
    /// one.
    ///
    /// The reference here is written to the same clock as the track being
    /// measured and put onto the film by a correction. Ignoring that correction
    /// would make the two identical, and the answer would come back as the
    /// identity rather than as the shift the film actually needs.
    #[test]
    fn a_reference_is_read_as_the_film_plays_it_rather_than_as_its_file_wrote_it() {
        let fixture = Fixture::mistimed();
        let reference = fixture.add_track(TrackOrigin::Stream, 1, "en", "subrip", &dialogue());
        fixture
            .database
            .tracks()
            .set_correction(reference, Correction::of_offset(TRUTH))
            .unwrap();
        fixture.take_the_film_away();

        let outcome = fixture.align();
        let AlignmentView::Aligned { .. } = &outcome else {
            panic!("a corrected reference should still describe its film: {outcome:?}");
        };
        let error = residual(fixture.correction());
        assert!(error <= i64::from(BIN_MS), "out by {error}ms");
    }

    /// A captioned track describes the same film as a plain one and has to be
    /// measured to the same answer. What it must not do is lose a caption: the
    /// descriptions are left out of the measurement and out of nothing else, so
    /// the cues the player draws and the cues in the library are the file's own.
    #[test]
    fn the_sounds_a_film_makes_are_measured_past_rather_than_thrown_away() {
        let fixture = Fixture::captioned();

        let outcome = fixture.align();
        let AlignmentView::Aligned { .. } = outcome else {
            panic!("a captioned track describes the film as well as a plain one: {outcome:?}");
        };
        let error = residual(fixture.correction());
        assert!(error <= SLACK, "out by {error}ms");

        let expected = captioned_cues();
        let drawn = fixture.database.tracks().cues(fixture.track_id).unwrap();
        assert_eq!(drawn.len(), expected.len());
        assert_eq!(
            drawn
                .iter()
                .filter(|cue| cue.text == "[DOOR SLAMS]")
                .count(),
            expected
                .iter()
                .filter(|cue| cue.text == "[DOOR SLAMS]")
                .count()
        );
    }

    /// The judgement the figure exists for, checked where it turns over. A
    /// measurement that would make a film worse is refused however sure the
    /// correlation was, and one that would improve a film and still leave most
    /// of it missing the talking is refused as well.
    #[test]
    fn a_measurement_is_written_only_where_it_helps_and_lands() {
        assert!(worth_writing(measured(7), measured(5)));
        assert!(!worth_writing(measured(5), measured(7)));
        assert!(!worth_writing(measured(3), measured(1)));
    }

    /// A track that already lands is answered with the identity, and writing
    /// nothing over nothing is not a failure to report to somebody whose file
    /// was right all along.
    #[test]
    fn a_track_that_is_already_right_is_not_reported_as_a_refusal() {
        assert!(worth_writing(measured(9), measured(9)));
    }

    #[test]
    fn a_measurement_of_nothing_is_never_written() {
        assert!(!worth_writing(Landing::NONE, measured(1)));
    }

    #[test]
    fn what_was_measured_is_still_there_after_a_restart() {
        let fixture = Fixture::mistimed();
        let written = match fixture.align() {
            AlignmentView::Aligned { correction, .. } => correction,
            outcome => panic!("expected an alignment, got {outcome:?}"),
        };

        let reopened = fixture.reopen();
        let track = reopened.tracks().by_id(fixture.track_id).unwrap().unwrap();
        assert_eq!(track.correction.offset_ms(), written.offset_ms);
        assert!(residual(track.correction) <= SLACK);

        // And the cues the player would be handed come out where the film
        // talks, since they go through the same correction every read does.
        let cues = reopened.tracks().cues(fixture.track_id).unwrap();
        let first = i64::from(cues[0].start.millis());
        assert!((first - i64::from(20_000 + TRUTH as u32)).abs() <= SLACK);
    }

    /// The measurement is of the track against the film, not of the track
    /// against its own last answer, so a value already in force changes the
    /// answer not at all and comes back so it can be put back.
    #[test]
    fn a_correction_already_in_force_is_measured_past_and_handed_back() {
        let fixture = Fixture::mistimed();
        let by_hand = Correction::of_offset(-9_000);
        fixture
            .database
            .tracks()
            .set_correction(fixture.track_id, by_hand)
            .unwrap();

        let AlignmentView::Aligned {
            correction,
            previous,
            ..
        } = fixture.align()
        else {
            panic!("a film measured against its own subtitle should line up");
        };

        assert_eq!(previous.offset_ms, by_hand.offset_ms());
        assert!(residual(correction.wanted()) <= SLACK);
    }

    /// The case the whole confidence figure exists for. A subtitle for another
    /// film correlates a little at every lag, because dialogue is spread
    /// through every film in much the same way, and acting on that would leave
    /// somebody with a film that is wrong from beginning to end and no reason
    /// to suspect the tool.
    #[test]
    fn a_subtitle_belonging_to_a_different_film_is_declined() {
        let fixture = Fixture::mistimed();
        fixture
            .database
            .tracks()
            .replace_cues(fixture.track_id, &another_film())
            .unwrap();

        let outcome = fixture.align();
        let AlignmentView::Uncertain {
            confidence, wanted, ..
        } = outcome
        else {
            panic!("another film's subtitle should not be believed: {outcome:?}");
        };

        assert!((wanted - THRESHOLD).abs() < f32::EPSILON);
        assert!(confidence < THRESHOLD, "believed at {confidence}");
        assert!(fixture.correction().is_identity());
    }

    /// The case the landing figure exists for, end to end. The correlation
    /// finds the shift and is sure about it, because the third of the film that
    /// does speak lines up exactly; the answer is still not one to write,
    /// because most of the track lands on a film that says nothing there.
    #[test]
    fn a_measurement_that_leaves_most_of_the_film_unspoken_is_not_written() {
        let fixture = Fixture::mostly_unspoken();

        let outcome = fixture.align();
        let AlignmentView::NoBetter {
            landing,
            as_written,
            wanted,
            ..
        } = outcome
        else {
            panic!("a film that says a third of its subtitle should not be written: {outcome:?}");
        };

        assert!(landing.fraction > as_written.fraction, "no better either");
        assert!(landing.fraction < wanted, "{} landed", landing.fraction);
        assert!(fixture.correction().is_identity());
    }

    /// A recording with an advertisement break in it. The lines are all there and
    /// no single shift places them all, so nothing is written and the file is
    /// named for what it is. Being turned away by the landing figure would have
    /// happened anyway; being told which kind of file this is, and roughly where
    /// it jumps, is what somebody can act on.
    #[test]
    fn a_film_with_a_break_cut_into_it_is_named_rather_than_corrected() {
        let fixture = Fixture::with_breaks();

        let outcome = fixture.align();
        let AlignmentView::Breaks { at_ms, as_written } = outcome else {
            panic!("a recording with a break in it should be named as one: {outcome:?}");
        };

        // Within a stretch of film of where the break is, which is as near as
        // readings a minute wide can honestly put it.
        assert!(
            at_ms.abs_diff(BREAK_AT) < 120_000,
            "the break is at {BREAK_AT}ms and was found at {at_ms}ms"
        );
        assert!(as_written.examined > 0);
        assert!(fixture.correction().is_identity());
    }

    /// A subtitle for a shorter cut of the same film. The film talks through three
    /// minutes the subtitle has no lines for at all, and everything after that is
    /// out by the length of the missing scene.
    ///
    /// This is the case a correction must never be offered for. Half the film
    /// would line up and every added scene would be further out than it started,
    /// and nothing about the result would look wrong until somebody reached one.
    #[test]
    fn a_subtitle_for_a_different_cut_is_refused_and_says_where_it_stopped_matching() {
        let fixture = Fixture::for_another_cut();

        let outcome = fixture.align();
        let AlignmentView::DifferentCut { from_ms, .. } = outcome else {
            panic!("a subtitle for another cut should be refused as one: {outcome:?}");
        };

        assert!(
            from_ms > 0 && from_ms < LENGTH_MS,
            "it stopped matching at {from_ms}ms"
        );
        assert!(fixture.correction().is_identity());
    }

    /// A forced or signs-only track, which is the common case and is settled
    /// before any audio is read. The film named here is not on the disk at all,
    /// so anything that opened it would come back unreadable instead.
    #[test]
    fn a_track_with_too_few_lines_is_declined_before_the_film_is_touched() {
        let cues = dialogue();
        let fixture = Fixture::written(&[], &cues[..12]);
        std::fs::remove_file(fixture.root.join("Heat.mkv")).unwrap();

        assert!(matches!(
            fixture.align(),
            AlignmentView::TooFewCues { cues: 12, wanted } if wanted as usize == FEWEST_CUES
        ));
        assert!(fixture.correction().is_identity());
    }

    #[test]
    fn a_film_whose_audio_cannot_be_read_says_so_by_name() {
        // Short, because it is turned away at the header and the samples under
        // it are never reached.
        let film = Film::new(10_000)
            .recorded(8_000, 1)
            .speaking(1_000, 5_000)
            .claiming("A_AC3");
        let fixture = Fixture::new(&film, &dialogue());

        assert!(matches!(
            fixture.align(),
            AlignmentView::Unsupported { codec: Some(name) } if name == "AC-3"
        ));
        assert!(fixture.correction().is_identity());
    }

    #[test]
    fn a_film_with_no_soundtrack_says_so() {
        let film = Film::new(10_000).without_audio();
        let fixture = Fixture::new(&film, &dialogue());

        assert!(matches!(fixture.align(), AlignmentView::NoAudio));
        assert!(fixture.correction().is_identity());
    }

    #[test]
    fn a_film_that_is_not_there_is_unreadable_rather_than_a_failure() {
        let fixture = Fixture::written(&[], &dialogue());
        std::fs::remove_file(fixture.root.join("Heat.mkv")).unwrap();

        assert!(matches!(fixture.align(), AlignmentView::Unreadable { .. }));
        assert!(fixture.correction().is_identity());
    }

    /// Stopping leaves the track as it was found. Nothing is written until a
    /// measurement has been made and believed, and a reading that stops makes
    /// no measurement.
    #[test]
    fn stopping_the_reading_leaves_the_correction_alone() {
        let fixture = Fixture::mistimed();
        let seen = AtomicUsize::new(0);

        // The reading asks two hundred times over a film, so a request made
        // part way through is noticed within a two hundredth of it.
        let outcome = run(&fixture.database, fixture.track_id, &|_: f32| {
            if seen.fetch_add(1, Ordering::Relaxed) < 5 {
                Reading::Continue
            } else {
                Reading::Stop
            }
        })
        .unwrap();

        assert!(matches!(outcome, AlignmentView::Stopped));
        assert!(fixture.correction().is_identity());
        assert!(seen.load(Ordering::Relaxed) > 0, "nothing was ever read");
    }
}
