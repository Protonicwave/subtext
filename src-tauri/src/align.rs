//! Putting a subtitle track where the speech in its film actually is.
//!
//! Coordination and nothing else. Reading the authored cues, asking
//! `subtext-speech` where the talking is, asking `subtext-align` what would
//! explain one in terms of the other, and writing the answer down are four
//! things that happen in an order, with no arithmetic of their own. The order
//! is the only judgement here, and it is to refuse cheaply: a track nothing
//! could be made of is turned away before a byte of audio is read.
//!
//! What is not coordination is deciding. Whether a measurement is worth writing
//! to somebody's library is a judgement about the product rather than a property
//! of the correlation, so the engine reports its figures and this weighs them.
//! There are two of those judgements and they ask different questions. The
//! threshold asks how clearly the correlation chose its answer. The bar asks
//! whether the answer it chose actually puts the lines on the talking, which is
//! the only question the correlation cannot mark its own work on.

use subtext_align::Landing;
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
/// Four measurements set it. Against a real film, a hundred and five minute
/// AAC rip whose own subtitle runs two seconds early, that subtitle scores
/// 0.027 and a transcript of the same density belonging to another film scores
/// 0.0035. Against the fixture films below, whose speech is bursts against
/// digital silence, the same two cases score 0.456 and 0.0074.
///
/// So the number has to sit above 0.0074 and below 0.027, and 0.015 is the
/// middle of that in the sense that matters. Confidence is a ratio and moves by
/// multiples, so the middle of a range is the geometric one: this is a factor of
/// about two from either side, where a value placed halfway by subtraction would
/// sit almost on top of the correct answer.
///
/// The two regimes are four hundredths apart at the top and a hundredth apart at
/// the bottom, which is the useful thing to know about this number. Real audio
/// carries music and effects through the gaps and the reading marks some of it,
/// so a correct answer on a real film is a quarter of a perfect match rather
/// than most of one, and everything shifts down with it. What separates a right
/// answer from a wrong one in both regimes is the height of the peak rather than
/// the score as a whole: 0.247 against 0.027 on the real film, where the margins
/// were 0.110 and 0.130 and told the two apart not at all. Somebody revisiting
/// this should look there first, and should measure a second real film before
/// moving the number on the strength of one.
///
/// Erring low would be worse than erring high. Refusing a file that could have
/// been helped leaves somebody exactly where they were, with the keys they
/// already had and a sentence saying why. Accepting one that could not leaves
/// them watching a film that is wrong from beginning to end.
const THRESHOLD: f32 = 0.015;

/// How much of a track has to land on the talking before anything is written.
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
/// Four tenths is set from what chance gives. A track measured against a film it
/// has nothing to do with lands wherever an utterance happens to fall inside the
/// half second either side of a line, which on dialogue starting every four
/// seconds or so is about an eighth of it. This sits at more than three times
/// that, and well under what a correct answer reaches, since a track written for
/// its own film lands nearly everywhere the reading found a voice. The room
/// between those two is wide, and it has to be: no film scores one, because
/// whispers, lines away from the microphone and dialogue under a loud mix are
/// speech that the reading misses on every film there is.
///
/// It errs the same way the threshold does, and for the same reason. Refusing a
/// file that could have been helped leaves somebody where they were, with the
/// keys they already had and a sentence saying why. This number should be
/// settled against a run of real films rather than moved on the strength of one.
const BAR: f32 = 0.4;

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
        return Ok(AlignmentView::uncertain(&found, confidence, THRESHOLD));
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

    Ok(AlignmentView::aligned(&found, track.correction, confidence))
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

    use subtext_align::{Landing, Signal, landing_of};
    use subtext_core::{Correction, Cue, SubtitleLabel, Timestamp};
    use subtext_index::{Database, NewFilm, NewTrack, TrackMatch, TrackOrigin};
    use subtext_speech::fixture::Film;
    use subtext_speech::{Reading, Unwatched};
    use tempfile::TempDir;

    use super::{BAR, FEWEST_CUES, THRESHOLD, run, worth_writing};
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
    /// line up equally well almost anywhere. What the lines say is of no
    /// interest to any of this: the measurement is of when somebody speaks.
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

        let AlignmentView::Aligned {
            previous,
            confidence,
            landing,
            as_written,
            ..
        } = fixture.align()
        else {
            panic!("a film measured against its own subtitle should line up");
        };

        assert_eq!(previous.offset_ms, 0);
        assert!(confidence >= THRESHOLD, "only {confidence} sure");

        // And the evidence for it, which is the part that did not come out of
        // the correlation that produced the answer.
        assert!(landing.fraction > as_written.fraction);
        assert!(landing.fraction >= BAR, "only {} landed", landing.fraction);
        assert!(landing.examined > 0);

        let error = residual(fixture.correction());
        assert!(error <= SLACK, "out by {error}ms");
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
