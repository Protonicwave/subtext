//! Whether the lines land where the talking is.
//!
//! The correction and the confidence beside it come out of one correlation peak,
//! which means the only test an answer has ever been put to is the arithmetic
//! that produced it. This is a different question, asked of the film rather than
//! of the estimator: with a correction applied, do the lines arrive when
//! somebody starts speaking?
//!
//! Starts rather than whole cues. A viewer notices when a line arrives and not
//! when it leaves, and how long a line is held is an authoring choice about
//! reading speed rather than a fact about the film. It is the same reasoning
//! that has the speech reading close the quiet inside an utterance: what is
//! being compared is the moment somebody starts talking.
//!
//! Only the lines somebody speaks in are measured, which is the same rule the
//! signal is built by and is here for the same reason. A cue captioning a door
//! slamming marks a moment of no speech, so it was never going to arrive as
//! somebody started talking, and counting it as a line that failed to would
//! score a track written for the hearing impaired below one written for
//! everybody else for describing the film more fully.
//!
//! A correct alignment does not score one, and nothing here should be read as
//! though it might. Whispers, lines away from the microphone and dialogue under
//! a loud mix are all speech that the reading misses, on every film, however
//! well timed the subtitle is. The figure earns its place by being comparable
//! with itself: what it says about a track as the file wrote it and what it says
//! about the same track corrected are two numbers in the units a person
//! perceives, and the difference between them is the only evidence in this crate
//! that a measurement is worth writing to somebody's library.

use subtext_core::{Correction, Cue};

use crate::signal::{BIN_MS, Signal};
use crate::spoken::is_spoken;

/// How far ahead of an utterance a line may arrive and still have landed.
///
/// Seven tenths of a second, and much wider than the bound below it, because
/// arriving early and arriving late are not the same thing. Reading is slower
/// than hearing, so broadcast practice puts a line on screen a little before the
/// words are said, and a subtitle written that way is right rather than early. A
/// speech reading also finds the front edge of an utterance late as often as
/// not, since a line that opens quietly is not marked until it is loud enough to
/// be, and that error points this way too.
///
/// The number is what a run over real films showed. On nine of the ten films the
/// corpus could measure, the engine's own answer for the untouched file was
/// early, by between twenty and six hundred and thirty milliseconds, and the
/// middle line of a correctly aligned track sat three hundred milliseconds from
/// the nearest utterance. A quarter of a second either way, which is what this
/// used to be, therefore counted more than half of every correctly placed track
/// as missing, and the figure was measuring its own tolerance.
pub const LEAD_TOLERANCE_MS: u32 = 700;

/// How far behind an utterance a line may arrive and still have landed.
///
/// A quarter of a second, which is roughly the length of a spoken word. This is
/// the side somebody notices: a line that appears after the words have started
/// is the complaint the whole feature exists to answer, so it is held to what a
/// viewer would call together and no further.
pub const LATE_TOLERANCE_MS: u32 = 250;

/// How far either side of a line the search looks before giving up on it.
///
/// Two seconds is already several times past anything anybody would call
/// landing, so the distance beyond it is of no interest except as a way of
/// saying that there was nothing anywhere near. Bounding the search is also what
/// keeps the whole measure to a fixed cost a line rather than one that grows
/// with the film.
const REACH_MS: u32 = 2_000;

const REACH_BINS: usize = (REACH_MS / BIN_MS) as usize;
const LEAD_BINS: usize = (LEAD_TOLERANCE_MS / BIN_MS) as usize;
const LATE_BINS: usize = (LATE_TOLERANCE_MS / BIN_MS) as usize;

/// One slot for each distance the search can report, and one for further away.
const SLOTS: usize = REACH_BINS + 2;

/// How a track's lines sit against the talking in its film.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landing {
    landed: u32,
    examined: u32,
    median_ms: u32,
}

impl Landing {
    /// Nothing could be measured, which is not the same as measuring badly.
    ///
    /// What a track with no lines inside the film produces, and what a film with
    /// no speech found in it produces. Neither is a figure of zero: a fraction
    /// of nothing says nothing at all, and [`Landing::examined`] is what tells
    /// the two apart.
    pub const NONE: Self = Self {
        landed: 0,
        examined: 0,
        median_ms: 0,
    };

    /// How many lines arrived near enough to somebody starting to speak.
    ///
    /// Near enough is [`LEAD_TOLERANCE_MS`] ahead of them or
    /// [`LATE_TOLERANCE_MS`] behind, which is not the same distance either way
    /// and is not meant to be.
    #[must_use]
    pub fn landed(self) -> u32 {
        self.landed
    }

    /// How many lines were measured at all.
    ///
    /// Lines falling past the end of the film are not among them. There is
    /// nothing there for them to land on, and counting them as misses would
    /// score a track for the length of its film rather than for its timing.
    ///
    /// Neither are the cues nobody speaks in. A line reading that a door slams
    /// marks a moment of no speech, so it was never going to arrive as somebody
    /// started talking, and counting it as a line that failed to would score a
    /// track written for the hearing impaired below one written for everybody
    /// else for describing the film more fully. It is the same rule that keeps
    /// those cues out of the signal, applied to the figure the signal is judged
    /// by, and for the same reason.
    #[must_use]
    pub fn examined(self) -> u32 {
        self.examined
    }

    /// The share of lines that landed, from none to all.
    #[must_use]
    pub fn fraction(self) -> f32 {
        if self.examined == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let fraction = self.landed as f32 / self.examined as f32;
        fraction
    }

    /// How far the middle line falls from the nearest utterance, in
    /// milliseconds.
    ///
    /// The middle rather than the mean, because a handful of lines with no
    /// speech anywhere near them, which every track has, would drag an average
    /// somewhere that describes none of it.
    ///
    /// Capped at [`REACH_MS`]. A median sitting on the cap means at least half
    /// the track has nothing within two seconds of it, and how much further than
    /// that is not worth the pass it would cost to find out.
    #[must_use]
    pub fn median_ms(self) -> u32 {
        self.median_ms
    }

    /// Whether there was anything to measure.
    #[must_use]
    pub fn is_measured(self) -> bool {
        self.examined > 0
    }
}

/// How well `cues` land on the speech in a film once `correction` is applied.
///
/// The correction is applied as the cues are walked, so nothing here builds a
/// corrected copy of a transcript to throw away afterwards, and the whole
/// measure allocates nothing at all. Handing it [`Correction::IDENTITY`] gives
/// the figure for the file exactly as it was written, which is the only thing
/// any other figure is worth comparing against.
#[must_use]
pub fn landing_of(cues: &[Cue], speech: &Signal, correction: Correction) -> Landing {
    // Distances, counted rather than collected. A median wants the distribution
    // and a sort would want a buffer the length of the transcript; there are
    // only ever [`SLOTS`] answers, so counting how often each came up is the
    // same information in a fixed array.
    let mut counts = [0_u32; SLOTS];
    let mut examined = 0_u32;
    let mut landed = 0_u32;

    // The lines somebody speaks in and no others, which is the rule the signal
    // is built by. A track is otherwise scored down for captioning the sounds a
    // film makes, on exactly the tracks most likely to want putting right.
    for cue in cues.iter().filter(|cue| is_spoken(cue)) {
        let at = usize::try_from(correction.apply(cue.start).millis()).unwrap_or(usize::MAX)
            / BIN_MS as usize;
        if at >= speech.len() {
            continue;
        }

        examined += 1;
        let (away, near_enough) = nearest_utterance(speech, at);
        counts[away] += 1;
        if near_enough {
            landed += 1;
        }
    }

    if examined == 0 {
        return Landing::NONE;
    }

    Landing {
        landed,
        examined,
        median_ms: median_of(&counts, examined),
    }
}

/// How many bins from `at` the nearest utterance starts, and whether any
/// utterance is near enough for the line to have landed.
///
/// Two answers rather than one, because the window is not the same width either
/// way and the nearest utterance is therefore not always the one that decides.
/// A line six hundred milliseconds ahead of somebody speaking has landed; the
/// same line three hundred milliseconds behind a different utterance has not,
/// and that second utterance is the nearer of the two.
///
/// Searching outwards from the line rather than walking the film, so that a
/// track whose cues arrive out of order, which the parser allows and files
/// contain, is measured the same as one whose cues are in order. The search
/// stops as soon as it has both answers: once something has landed there is
/// nothing nearer to find, and once the search is past the wider bound with a
/// distance in hand nothing further out can land.
fn nearest_utterance(speech: &Signal, at: usize) -> (usize, bool) {
    let outermost = LEAD_BINS.max(LATE_BINS);
    let mut away = None;

    for distance in 0..=REACH_BINS {
        // Ahead of the line is a line that arrived early, which is what a
        // subtitle written to be read looks like.
        let early = starts_speaking(speech, at + distance);
        let late = distance <= at && starts_speaking(speech, at - distance);

        if early || late {
            let first = *away.get_or_insert(distance);
            if (early && distance <= LEAD_BINS) || (late && distance <= LATE_BINS) {
                return (first, true);
            }
        }
        if away.is_some() && distance > outermost {
            break;
        }
    }

    (away.unwrap_or(REACH_BINS + 1), false)
}

/// Whether somebody starts talking at this bin, as opposed to carrying on.
fn starts_speaking(speech: &Signal, bin: usize) -> bool {
    speech.is_active(bin) && (bin == 0 || !speech.is_active(bin - 1))
}

/// The middle distance, in milliseconds.
///
/// The lower of the two middles where the count is even, which is a difference
/// of one bin in a figure quoted to the nearest ten milliseconds and not worth
/// the arithmetic to avoid.
fn median_of(counts: &[u32; SLOTS], examined: u32) -> u32 {
    let mut seen = 0_u32;
    for (away, count) in counts.iter().enumerate() {
        seen = seen.saturating_add(*count);
        if seen.saturating_mul(2) > examined {
            return u32::try_from(away.min(REACH_BINS)).unwrap_or(0) * BIN_MS;
        }
    }
    REACH_MS
}

#[cfg(test)]
mod tests {
    use super::{LATE_TOLERANCE_MS, LEAD_TOLERANCE_MS, Landing, REACH_MS, landing_of};
    use crate::signal::{BIN_MS, Signal};
    use subtext_core::{Correction, Cue, Timestamp};

    /// A film that talks for a second and a half every four seconds, starting at
    /// ten, and ending as the last utterance's four seconds run out.
    fn talking(utterances: u32) -> Signal {
        let mut bins = vec![false; (10_000 + utterances * 4_000) as usize / 10];
        for at in 0..utterances {
            let from = (10_000 + at * 4_000) as usize / 10;
            bins[from..from + 150].fill(true);
        }
        Signal::from_bins(bins)
    }

    /// Lines claiming to describe that film, `out_by` milliseconds from where it
    /// actually talks.
    fn lines(count: u32, out_by: i64) -> Vec<Cue> {
        (0..count)
            .map(|at| {
                let start = i64::from(10_000 + at * 4_000) + out_by;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let start = start.max(0) as u32;
                Cue {
                    index: at + 1,
                    start: Timestamp::from_millis(start),
                    end: Timestamp::from_millis(start + 1_500),
                    text: "line".to_owned(),
                    position: None,
                }
            })
            .collect()
    }

    #[test]
    fn a_track_written_against_its_film_lands_on_every_line() {
        let film = talking(100);
        let found = landing_of(&lines(100, 0), &film, Correction::IDENTITY);

        assert_eq!(found.examined(), 100);
        assert_eq!(found.landed(), 100);
        assert!((found.fraction() - 1.0).abs() < f32::EPSILON);
        assert_eq!(found.median_ms(), 0);
    }

    /// The whole reason the figure exists: it is read before and after, and the
    /// difference between the two readings is what says a correction is worth
    /// writing.
    #[test]
    fn a_correction_is_what_makes_a_mistimed_track_land() {
        let film = talking(100);
        let cues = lines(100, -3_000);

        let before = landing_of(&cues, &film, Correction::IDENTITY);
        let after = landing_of(&cues, &film, Correction::of_offset(3_000));

        assert_eq!(before.landed(), 0);
        assert_eq!(after.landed(), 100);
        assert!(after.fraction() > before.fraction());
    }

    #[test]
    fn a_line_just_inside_the_tolerance_lands_and_one_just_outside_does_not() {
        let film = talking(100);

        for out_by in [
            i64::from(LATE_TOLERANCE_MS) - i64::from(BIN_MS),
            i64::from(BIN_MS) - i64::from(LEAD_TOLERANCE_MS),
        ] {
            let found = landing_of(&lines(100, out_by), &film, Correction::IDENTITY);
            assert_eq!(found.landed(), 100, "at {out_by}ms");
        }

        for out_by in [
            i64::from(LATE_TOLERANCE_MS) + i64::from(BIN_MS),
            -i64::from(LEAD_TOLERANCE_MS) - i64::from(BIN_MS),
        ] {
            let found = landing_of(&lines(100, out_by), &film, Correction::IDENTITY);
            assert_eq!(found.landed(), 0, "at {out_by}ms");
        }
    }

    /// Arriving early and arriving late are not the same thing, and the figure
    /// is not the same width either way. Reading is slower than hearing, so a
    /// line put on screen before the words are said is right rather than early,
    /// which is what broadcast practice produces and what most of the files this
    /// measures were written to.
    #[test]
    fn a_line_that_leads_its_utterance_is_given_more_room_than_one_that_trails() {
        let film = talking(100);
        let leading = i64::from(LEAD_TOLERANCE_MS) - i64::from(BIN_MS);

        assert_eq!(
            landing_of(&lines(100, -leading), &film, Correction::IDENTITY).landed(),
            100
        );
        // The same distance the other way, where somebody has already started
        // talking, and the line has not landed.
        assert_eq!(
            landing_of(&lines(100, leading), &film, Correction::IDENTITY).landed(),
            0
        );
    }

    /// The nearest utterance is not always the one that decides. A line can lead
    /// one utterance by more than it trails another, and still have landed on
    /// the first, so the search cannot stop at whichever it meets first.
    #[test]
    fn a_line_lands_on_an_utterance_it_leads_even_where_a_nearer_one_is_behind_it() {
        // A film that says something at ten seconds and again at ten point
        // eight, and a line arriving between the two: three hundred
        // milliseconds after the first, which is too late to have landed on it,
        // and five hundred before the second, which is not.
        let mut bins = vec![false; 3_000];
        bins[1_000..1_030].fill(true);
        bins[1_080..1_110].fill(true);
        let film = Signal::from_bins(bins);

        let cue = Cue {
            index: 1,
            start: Timestamp::from_millis(10_300),
            end: Timestamp::from_millis(11_300),
            text: "line".to_owned(),
            position: None,
        };
        let found = landing_of(&[cue], &film, Correction::IDENTITY);

        assert_eq!(found.landed(), 1);
        // And the distance reported is still the nearest one, which is the
        // utterance it did not land on.
        assert_eq!(found.median_ms(), 300);
    }

    #[test]
    fn the_middle_distance_is_reported_rather_than_the_average() {
        let film = talking(100);
        let mut cues = lines(100, 200);
        // A quarter of the track flung far enough away to have nothing near it,
        // which every real transcript has some of and which an average would let
        // decide the answer.
        for cue in cues.iter_mut().take(25) {
            cue.start = Timestamp::from_millis(cue.start.millis() + 90_000);
            cue.end = Timestamp::from_millis(cue.end.millis() + 90_000);
        }

        let found = landing_of(&cues, &film, Correction::IDENTITY);
        assert_eq!(found.median_ms(), 200);
    }

    #[test]
    fn a_track_that_lands_nowhere_says_so_in_both_figures() {
        let film = talking(100);
        // Halfway between one utterance and the next, all the way through.
        let found = landing_of(&lines(100, 2_000), &film, Correction::IDENTITY);

        assert_eq!(found.examined(), 100);
        assert_eq!(found.landed(), 0);
        assert!(found.fraction() < f32::EPSILON);
        assert_eq!(found.median_ms(), 2_000);
    }

    #[test]
    fn a_distance_past_the_reach_is_reported_as_the_reach() {
        // One utterance at the very start and lines spread across an hour of
        // silence after it.
        let mut bins = vec![false; 360_000];
        bins[0..150].fill(true);
        let film = Signal::from_bins(bins);

        let found = landing_of(&lines(100, 600_000), &film, Correction::IDENTITY);
        assert_eq!(found.median_ms(), REACH_MS);
        assert_eq!(found.landed(), 0);
    }

    #[test]
    fn a_film_with_no_speech_in_it_at_all_is_measured_and_found_empty() {
        let film = Signal::from_bins(vec![false; 100_000]);
        let found = landing_of(&lines(100, 0), &film, Correction::IDENTITY);

        // Measured, unlike the cases below: the lines are inside the film, there
        // is simply nothing there for them to land on.
        assert_eq!(found.examined(), 100);
        assert_eq!(found.landed(), 0);
        assert_eq!(found.median_ms(), REACH_MS);
    }

    #[test]
    fn nothing_to_measure_is_told_apart_from_measuring_nothing() {
        let film = talking(100);

        for found in [
            landing_of(&[], &film, Correction::IDENTITY),
            landing_of(
                &lines(100, 0),
                &Signal::from_bins(Vec::new()),
                Correction::IDENTITY,
            ),
        ] {
            assert_eq!(found, Landing::NONE);
            assert!(!found.is_measured());
            assert!(found.fraction() < f32::EPSILON);
        }
    }

    /// The same rule that keeps a cue nobody speaks in out of the signal keeps
    /// it out of the figure the signal is judged by. A track that captions the
    /// sounds of a film as well as its words describes the same film as a plain
    /// one, and has to measure the same.
    #[test]
    fn a_cue_nobody_speaks_in_is_not_a_line_that_failed_to_land() {
        let film = talking(100);
        let plain = lines(100, 0);

        // The same track with the sounds of the film captioned in the quiet
        // between the lines, which is where a door slams and where a reading
        // looking for voices finds nothing.
        let mut captioned = plain.clone();
        for at in 0..100 {
            captioned.push(Cue {
                index: 1_000 + at,
                start: Timestamp::from_millis(12_000 + at * 4_000),
                end: Timestamp::from_millis(12_900 + at * 4_000),
                text: "[DOOR SLAMS]".to_owned(),
                position: None,
            });
        }

        assert_eq!(
            landing_of(&captioned, &film, Correction::IDENTITY),
            landing_of(&plain, &film, Correction::IDENTITY)
        );
    }

    /// A track that runs past the end of the film it is paired with, which is
    /// what a subtitle for a longer cut looks like. What sits inside the film is
    /// measured and the rest is left out rather than counted as missing.
    #[test]
    fn lines_falling_past_the_end_of_the_film_are_not_measured() {
        let film = talking(50);
        let found = landing_of(&lines(200, 0), &film, Correction::IDENTITY);

        assert_eq!(found.examined(), 50);
        assert_eq!(found.landed(), 50);
    }

    /// The correction is applied as the cues are walked, so a stretch is
    /// measured at the moment each line actually arrives rather than at the one
    /// the file wrote.
    #[test]
    fn a_stretch_is_measured_where_it_puts_the_lines() {
        let film = talking(100);
        let stretch = 4_000.0 / 3_950.0;
        let cues: Vec<Cue> = lines(100, 0)
            .into_iter()
            .map(|cue| {
                let at = 10_000 + (cue.index - 1) * 3_950;
                Cue {
                    start: Timestamp::from_millis(at),
                    end: Timestamp::from_millis(at + 1_500),
                    ..cue
                }
            })
            .collect();

        let before = landing_of(&cues, &film, Correction::IDENTITY);
        let after = landing_of(&cues, &film, Correction::new(-127, stretch));

        assert!(
            before.fraction() < 0.5,
            "{} landed already",
            before.landed()
        );
        assert!(after.fraction() > 0.9, "only {} landed", after.landed());
    }

    #[test]
    fn cues_out_of_order_are_measured_the_same_as_cues_in_order() {
        let film = talking(100);
        let ordered = lines(100, 120);
        let mut jumbled = ordered.clone();
        jumbled.reverse();

        assert_eq!(
            landing_of(&ordered, &film, Correction::IDENTITY),
            landing_of(&jumbled, &film, Correction::IDENTITY)
        );
    }

    /// Somebody carrying on talking is not somebody starting to, so a line
    /// arriving in the middle of a long utterance has not landed. This is what
    /// keeps a track measured against a film that talks throughout from scoring
    /// well for no reason.
    #[test]
    fn arriving_in_the_middle_of_an_utterance_is_not_landing() {
        // One utterance from just before the first line to past the last.
        let mut bins = vec![false; 100_000];
        bins[990..9_000].fill(true);
        let film = Signal::from_bins(bins);

        let found = landing_of(&lines(20, 0), &film, Correction::IDENTITY);
        assert_eq!(found.examined(), 20);
        assert_eq!(found.landed(), 1, "only the line it begins on has landed");
    }
}
