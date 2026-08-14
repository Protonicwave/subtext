//! Putting a subtitle track where the speech in its film actually is.
//!
//! Coordination and nothing else. Reading the authored cues, asking
//! `subtext-speech` where the talking is, asking `subtext-align` what would
//! explain one in terms of the other, and writing the answer down are four
//! things that happen in an order, with no arithmetic of their own. The order
//! is the only judgement here, and it is to refuse cheaply: a track nothing
//! could be made of is turned away before a byte of audio is read.
//!
//! What is not coordination is the threshold. Deciding when a measurement is
//! worth writing to somebody's library is a judgement about the product rather
//! than a property of the correlation, which is why the engine reports a number
//! and this decides what to do about it.

use subtext_index::Database;
use subtext_speech::{Progress, Refusal};

use crate::dto::{AlignmentView, Answer, Failure};

/// How sure the measurement has to be before a correction is written.
///
/// Nudging by ear has a floor under it: somebody watching can hear that a
/// change made it worse and press the other key. An answer arrived at by
/// measurement has no such floor, because it is applied to a film nobody has
/// watched yet, and if it is wrong it is wrong for the whole film with no
/// reason to suspect the tool rather than the file. So the number is set where
/// a wrong answer is refused rather than where the most files are helped.
///
/// A quarter, and the two cases either side of it have been measured. A
/// subtitle belonging to a different film scores under a hundredth, because
/// dialogue is spread through every film in much the same way and a wrong
/// pairing correlates a little at every lag rather than plainly at one. A track
/// that is genuinely this film's, read from real audio with music and effects
/// over it, scores around a half.
///
/// Set nearer the upper of the two than halfway, because the two mistakes are
/// not equally bad. Refusing a file that could have been helped leaves somebody
/// exactly where they were, with the keys they already had and a sentence
/// saying why. Accepting one that could not leaves them watching a film that is
/// wrong from beginning to end.
const THRESHOLD: f32 = 0.25;

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

    // Before the cues, so that a film whose audio cannot be read costs the
    // header of one file rather than the whole of a transcript as well.
    let speech = match subtext_speech::speech_of_with(&film.path, progress) {
        Ok(speech) => speech,
        Err(refusal) => return Ok(refused(refusal)),
    };

    // As the file wrote them. Corrected timings would measure this track
    // against its own last answer rather than against the film.
    let cues = database
        .tracks()
        .authored_cues(track.id)
        .map_err(Failure::of)?;

    let found = subtext_align::align(&cues, &speech);
    let confidence = found.confidence().score();
    if confidence < THRESHOLD {
        return Ok(AlignmentView::uncertain(
            found.correction(),
            confidence,
            THRESHOLD,
        ));
    }

    database
        .tracks()
        .set_correction(track.id, found.correction())
        .map_err(Failure::of)?;

    Ok(AlignmentView::aligned(
        found.correction(),
        track.correction,
        confidence,
    ))
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

    use subtext_core::{Correction, Cue, SubtitleLabel, Timestamp};
    use subtext_index::{Database, NewFilm, NewTrack, TrackMatch, TrackOrigin};
    use subtext_speech::fixture::Film;
    use subtext_speech::{Reading, Unwatched};
    use tempfile::TempDir;

    use super::{FEWEST_CUES, THRESHOLD, run};
    use crate::dto::AlignmentView;

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
    /// line up equally well almost anywhere.
    fn dialogue() -> Vec<Cue> {
        let mut cues = Vec::new();
        let mut at_ms = 20_000;
        while at_ms < LENGTH_MS - 20_000 {
            let index = u32::try_from(cues.len()).unwrap();
            let length = 1_000 + (index % 5) * 400;
            cues.push(Cue {
                index: index + 1,
                start: Timestamp::from_millis(at_ms),
                end: Timestamp::from_millis(at_ms + length),
                text: "line".to_owned(),
                position: None,
            });
            at_ms += length + 800 + (index % 11) * 500;
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

    /// A library holding one film and one subtitle track for it.
    struct Fixture {
        database: Database,
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
                track_id,
                root,
                _directory: directory,
            }
        }

        /// A film whose dialogue falls [`TRUTH`] milliseconds after its
        /// subtitle says it does.
        fn mistimed() -> Self {
            let cues = dialogue();
            let late = Correction::of_offset(TRUTH);
            let spoken: Vec<Cue> = cues
                .iter()
                .map(|cue| Cue {
                    start: late.apply(cue.start),
                    end: late.apply(cue.end),
                    ..cue.clone()
                })
                .collect();
            Self::written(soundtrack(&spoken), &cues)
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

    #[test]
    fn a_film_that_is_out_by_a_known_amount_is_put_right() {
        let fixture = Fixture::mistimed();

        let AlignmentView::Aligned {
            previous,
            confidence,
            ..
        } = fixture.align()
        else {
            panic!("a film measured against its own subtitle should line up");
        };

        assert_eq!(previous.offset_ms, 0);
        assert!(confidence >= THRESHOLD, "only {confidence} sure");

        let error = residual(fixture.correction());
        assert!(error <= SLACK, "out by {error}ms");
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
