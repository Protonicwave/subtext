//! Turning how loud each bin is into whether somebody is talking in it.
//!
//! Two things are asked of a bin, and the second matters more than the first.
//!
//! It has to stand above the floor. A fixed threshold cannot judge that, because
//! films are mastered at wildly different levels and the same conversation is
//! forty decibels apart between a quiet drama and an action picture with the
//! dialogue riding over a mix. What is constant is the distance between speech
//! and whatever the film sounds like when nobody is speaking, so the floor is
//! measured from the film itself and moves with it.
//!
//! And it has to be moving. Loudness on its own answers whether there is sound,
//! which in a film is nearly always, and a signal that says yes for two thirds
//! of a film has nothing in it to line anything up against. What tells a voice
//! from a score is that a voice moves at the rate of syllables. See [`swing`].

/// How far apart two bins can be and still be judged against the same floor.
///
/// A minute. The failure this length is guarding against is a window with so
/// little silence in it that the floor is measured from the speech and rises to
/// meet it, which erases the middle of a long unbroken run and leaves its two
/// ends. A minute of film holds a great deal more than [`FLOOR_PERCENTILE`] of
/// gap between one line and the next, and it is still short enough to follow a
/// film out of a quiet room and into a street.
const WINDOW_BINS: usize = 60 * (1000 / crate::BIN_MS as usize);

/// Where in a window the floor sits.
///
/// The fifteenth percentile rather than the minimum, because a minimum is one
/// bin and one bin is whatever the quietest moment of tape hiss happened to be.
/// A low percentile is the same idea with the outlier taken out of it.
const FLOOR_PERCENTILE: f32 = 0.15;

/// How far above the floor a bin has to stand to be worth looking at, in
/// decibels.
///
/// Six, which is four times the power. This is a gate rather than the test: what
/// separates a voice from the rest of a film is [`swing`], and measured against
/// a real film this bar can be dropped to nothing without changing the answer
/// much. It is kept because loudness measured near the noise floor moves about
/// on its own, and a stretch of near silence would otherwise offer the movement
/// of a voice without the voice.
const MARGIN_DB: f32 = 6.0;

/// How far the loudness has to be moving for it to be a voice, in decibels.
///
/// See [`swing`] for what this measures and why it, rather than the loudness,
/// is what separates dialogue from everything else in a film mix.
const SWING_DB: f32 = 2.5;

/// The window the slow part of the loudness is taken over, in bins.
///
/// Half a second, which is slower than any syllable and faster than any scene.
const SLOW_BINS: usize = 51;

/// The window the grain is smoothed out over, in bins.
const FAST_BINS: usize = 5;

/// The window the movement is measured over, in bins.
///
/// A quarter of a second, which is about a word.
const DEPTH_BINS: usize = 25;

/// How long a quiet stretch has to be to be a gap between two utterances rather
/// than a gap inside one, in bins.
///
/// Three hundred milliseconds. This is the value that decides what shape the
/// signal has, and the shape matters more than the accuracy of any one bin,
/// because of what it is compared against. A subtitle cue is on screen for a
/// whole utterance: it does not blink off between the words, or between the
/// syllables, or through the stop in the middle of a consonant. Energy does. A
/// signal left as the energy found it toggles several times a second while the
/// cue it is being measured against holds for two, and two signals of the same
/// density but different granularity barely correlate at any lag at all.
///
/// So the quiet inside an utterance is closed up, and what is left is a signal
/// that says somebody is speaking here in the same sense that a cue does.
const GAP_BINS: usize = 30;

/// How long a run has to be to be somebody talking, in bins.
///
/// Two hundred milliseconds, which is about a syllable. Below it are the door
/// slams, the footsteps and the single notes that stand above the floor without
/// being anybody speaking. Applied after the gaps are closed rather than before,
/// so that a quiet word is kept by the sentence it belongs to instead of being
/// thrown away on its own.
const RUN_BINS: usize = 20;

/// The level of a bin with nothing in it at all, in decibels.
///
/// Digital silence has no level, and the logarithm of it is not a number. Films
/// carry stretches of exactly this over their opening frames, so the floor of
/// the scale is a value rather than a special case running through everything
/// downstream.
const SILENCE_DB: f32 = -120.0;

/// How loud one bin was, in decibels, from its summed squares and how many
/// samples went into it.
///
/// Empty bins can happen at the tail of a film and wherever a packet was
/// dropped, and they are silence rather than a gap, since the whole point of
/// this signal is that it lines up with a clock.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn level(sum: f32, count: u32) -> f32 {
    if count == 0 {
        return SILENCE_DB;
    }
    let mean_square = sum / count as f32;
    // The addition is what keeps digital silence on the scale, and it sits far
    // enough below any real signal to leave one alone.
    10.0 * (mean_square + 1e-12).log10()
}

/// Which bins hold speech.
pub(crate) fn speech(levels: &[f32]) -> Vec<bool> {
    let floor = rolling_floor(levels);
    let swing = swing(levels);
    let loud: Vec<bool> = levels
        .iter()
        .zip(&floor)
        .zip(&swing)
        .map(|((level, floor), swing)| *level > *floor + MARGIN_DB && *swing > SWING_DB)
        .collect();
    utterances(&loud)
}

/// How much the loudness is moving, bin by bin, in decibels.
///
/// This is the part that tells a voice from a soundtrack, and without it the
/// rest of this file cannot. Loudness alone says whether there is sound, and in
/// a film mixed after about 1970 there is almost always sound: a score under the
/// scene, a room, a street. A signal that says yes for two thirds of a film
/// carries almost nothing to line up against, whatever threshold produced it.
///
/// What a voice does that a mix does not is move at the rate of syllables, which
/// is three to six times a second in every language anybody has measured.
/// Sustained music, room tone and traffic all hold roughly level across a
/// quarter of a second; speech swings several decibels within one. So the slow
/// part of the loudness is subtracted, leaving how far it is moving, and it is
/// that rather than the loudness itself that has to clear a bar.
///
/// It works on the loudness that has already been measured, one value per bin,
/// so the whole of it is three passes over an array a film long and costs
/// nothing next to the decoding.
fn swing(levels: &[f32]) -> Vec<f32> {
    // Everything slower than the syllable rate, which is the mix rather than
    // anybody speaking, taken out by subtracting it.
    let slow = mean_over(levels, SLOW_BINS);
    let moving: Vec<f32> = levels
        .iter()
        .zip(&slow)
        .map(|(level, slow)| level - slow)
        .collect();

    // Everything faster than it, which is the grain of the measurement rather
    // than a syllable, smoothed away.
    let moving = mean_over(&moving, FAST_BINS);

    // How far it is moving, either way, over about the length of a word.
    let depth: Vec<f32> = moving.iter().map(|value| value.abs()).collect();
    mean_over(&depth, DEPTH_BINS)
}

/// The mean of each value and the `width` bins around it.
///
/// Carried along as a running total rather than added up at every bin, so the
/// width costs nothing and a film costs one pass.
fn mean_over(values: &[f32], width: usize) -> Vec<f32> {
    let reach = width / 2;
    let mut out = Vec::with_capacity(values.len());
    let mut total: f32 = values[..values.len().min(reach + 1)].iter().sum();

    for at in 0..values.len() {
        let from = at.saturating_sub(reach);
        let to = (at + reach + 1).min(values.len());
        #[allow(clippy::cast_precision_loss)]
        out.push(total / (to - from) as f32);

        if at >= reach {
            total -= values[at - reach];
        }
        if at + reach + 1 < values.len() {
            total += values[at + reach + 1];
        }
    }

    out
}

/// What the film sounds like when nobody is talking, at each bin.
///
/// Measured once for each window and read between them, rather than measured
/// afresh at every bin. A percentile over a thirty second window recomputed a
/// million times is minutes of work for an answer that cannot move quickly
/// anyway, since the thing it describes is the room a scene was recorded in.
fn rolling_floor(levels: &[f32]) -> Vec<f32> {
    let mut floor = vec![SILENCE_DB; levels.len()];
    if levels.is_empty() {
        return floor;
    }

    let mut scratch = Vec::with_capacity(WINDOW_BINS);
    let windows: Vec<(usize, f32)> = levels
        .chunks(WINDOW_BINS)
        .enumerate()
        .map(|(at, window)| {
            scratch.clear();
            scratch.extend_from_slice(window);
            (
                at * WINDOW_BINS + window.len() / 2,
                percentile(&mut scratch),
            )
        })
        .collect();

    // Before the first window's middle and after the last there is nothing to
    // read towards, so the nearest measurement stands.
    let length = floor.len();
    let (first_at, first) = windows[0];
    floor[..first_at.min(length)].fill(first);
    let (last_at, last) = windows[windows.len() - 1];
    floor[last_at.min(length)..].fill(last);

    for pair in windows.windows(2) {
        let (from_at, from) = pair[0];
        let (to_at, to) = pair[1];
        let span = to_at - from_at;
        for (step, bin) in floor[from_at..to_at].iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let along = step as f32 / span as f32;
            *bin = from + (to - from) * along;
        }
    }

    floor
}

/// The level [`FLOOR_PERCENTILE`] of the way up a window, which reorders it.
fn percentile(window: &mut [f32]) -> f32 {
    if window.is_empty() {
        return SILENCE_DB;
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let at = ((window.len() - 1) as f32 * FLOOR_PERCENTILE) as usize;
    // Only the one position is wanted, so the window is partitioned around it
    // rather than sorted. Over a film this is the difference between a second of
    // work and none.
    let (_, value, _) = window.select_nth_unstable_by(at, f32::total_cmp);
    *value
}

/// The same bins, gathered into utterances.
///
/// Gaps shorter than [`GAP_BINS`] are closed and what is left shorter than
/// [`RUN_BINS`] is dropped, in that order. Worked on the runs themselves rather
/// than by filtering the bins, which is both quicker and the only way to be sure
/// of the property that matters: a run that survives keeps the bins it started
/// and ended on, so nothing here moves the moment a line begins.
fn utterances(loud: &[bool]) -> Vec<bool> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut from = None;
    for (at, bin) in loud.iter().enumerate() {
        match (bin, from) {
            (true, None) => from = Some(at),
            (false, Some(start)) => {
                runs.push((start, at));
                from = None;
            }
            _ => {}
        }
    }
    if let Some(start) = from {
        runs.push((start, loud.len()));
    }

    let mut gathered: Vec<(usize, usize)> = Vec::with_capacity(runs.len());
    for (from, to) in runs {
        match gathered.last_mut() {
            Some(last) if from - last.1 < GAP_BINS => last.1 = to,
            _ => gathered.push((from, to)),
        }
    }

    let mut speech = vec![false; loud.len()];
    for (from, to) in gathered {
        if to - from >= RUN_BINS {
            speech[from..to].fill(true);
        }
    }
    speech
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, clippy::unwrap_used)]

    use super::{
        GAP_BINS, RUN_BINS, SILENCE_DB, SWING_DB, WINDOW_BINS, level, mean_over, speech, swing,
        utterances,
    };

    /// How many times a second a voice moves.
    const SYLLABLE_HZ: f32 = 4.0;

    /// A film's loudness, bin by bin: a quiet room throughout, with stretches of
    /// somebody talking and stretches of something merely loud.
    ///
    /// The two are the same level and differ only in whether they move, which is
    /// the whole question this file answers.
    fn film(
        length: usize,
        room: f32,
        loud: f32,
        talking: &[(usize, usize)],
        sustained: &[(usize, usize)],
    ) -> Vec<f32> {
        let mut levels = vec![room; length];
        for (from, to) in sustained {
            levels[*from..*to].fill(loud);
        }
        for (from, to) in talking {
            for (step, bin) in levels[*from..*to].iter_mut().enumerate() {
                // A hundred bins to the second.
                #[allow(clippy::cast_precision_loss)]
                let at = step as f32 / 100.0;
                *bin = loud + 6.0 * (core::f32::consts::TAU * SYLLABLE_HZ * at).sin();
            }
        }
        levels
    }

    #[test]
    fn silence_has_a_level_rather_than_no_level() {
        assert_eq!(level(0.0, 100), SILENCE_DB);
        assert_eq!(level(0.0, 0), SILENCE_DB);
        assert!(level(1.0, 1) > SILENCE_DB);
    }

    #[test]
    fn a_louder_bin_reads_as_louder() {
        assert!(level(1.0, 100) > level(0.1, 100));
        // Ten times the power is ten decibels, which is the whole point of the
        // scale and worth pinning down.
        assert!((level(1.0, 100) - level(0.1, 100) - 10.0).abs() < 0.01);
    }

    #[test]
    fn a_voice_is_found_and_a_note_of_the_same_loudness_is_not() {
        // The test this file exists to pass. Both stretches are forty decibels
        // above the room and one of them is somebody talking.
        let levels = film(20_000, -60.0, -20.0, &[(2_000, 4_000)], &[(10_000, 12_000)]);
        let found = speech(&levels);

        assert!(found[3_000]);
        assert!(!found[11_000]);
        assert!(!found[7_000]);
    }

    #[test]
    fn the_edges_of_a_voice_are_where_the_voice_is() {
        // Nothing between the loudness and the signal may move the moment a line
        // starts, since that moment is the answer the whole crate produces.
        let levels = film(20_000, -60.0, -20.0, &[(3_000, 6_000)], &[]);
        let found = speech(&levels);
        let from = found.iter().position(|bin| *bin).unwrap();
        let to = found.iter().rposition(|bin| *bin).unwrap() + 1;

        assert!(from.abs_diff(3_000) <= 2, "started at {from}");
        assert!(to.abs_diff(6_000) <= 2, "ended at {to}");
    }

    #[test]
    fn a_loud_film_and_a_quiet_one_give_the_same_answer() {
        let quiet = speech(&film(20_000, -70.0, -30.0, &[(2_000, 4_000)], &[]));
        let loud = speech(&film(20_000, -30.0, 10.0, &[(2_000, 4_000)], &[]));
        assert_eq!(quiet, loud);
    }

    #[test]
    fn a_film_that_gets_louder_part_way_through_is_followed() {
        // The second half is mixed forty decibels hotter than the first, which is
        // more than the difference between speech and the room in either. A fixed
        // floor would call the whole second half loud.
        let mut levels = film(60_000, -70.0, -40.0, &[(5_000, 8_000)], &[]);
        levels[30_000..].fill(-30.0);
        for (step, bin) in levels[40_000..43_000].iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let at = step as f32 / 100.0;
            *bin = 0.0 + 6.0 * (core::f32::consts::TAU * SYLLABLE_HZ * at).sin();
        }

        let found = speech(&levels);
        assert!(found[6_000]);
        assert!(found[41_000]);
        assert!(!found[20_000]);
        assert!(!found[50_000]);
    }

    #[test]
    fn a_voice_too_quiet_to_be_above_the_room_is_not_speech() {
        // Moving like a voice, but only three decibels above the floor, which is
        // what the level gate is there to keep out.
        let mut levels = vec![-60.0f32; 20_000];
        for (step, bin) in levels[2_000..4_000].iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let at = step as f32 / 100.0;
            *bin = -57.0 + 0.5 * (core::f32::consts::TAU * SYLLABLE_HZ * at).sin();
        }
        assert!(!speech(&levels).iter().any(|bin| *bin));
    }

    #[test]
    fn a_voice_swings_and_a_sustained_note_does_not() {
        let levels = film(20_000, -60.0, -20.0, &[(2_000, 6_000)], &[(10_000, 14_000)]);
        let swing = swing(&levels);

        assert!(swing[4_000] > SWING_DB, "a voice swung {}", swing[4_000]);
        assert!(swing[12_000] < SWING_DB, "a note swung {}", swing[12_000]);
    }

    #[test]
    fn a_mean_is_taken_over_what_is_actually_there() {
        assert_eq!(mean_over(&[1.0, 1.0, 1.0, 1.0, 1.0], 3), vec![1.0; 5]);
        // Five values, a window of three, so the ends average two and the middle
        // three.
        assert_eq!(
            mean_over(&[0.0, 3.0, 0.0, 0.0, 0.0], 3),
            vec![1.5, 1.0, 1.0, 0.0, 0.0]
        );
        assert!(mean_over(&[], 3).is_empty());
    }

    #[test]
    fn a_run_long_enough_to_be_speech_comes_through_unchanged() {
        // The property the whole signal rests on: nothing that gathers bins into
        // utterances may move the moment one starts, or the offset every reading
        // of this signal produces is out by however much it moved.
        let mut loud = vec![false; 500];
        loud[200..200 + RUN_BINS].fill(true);
        assert_eq!(utterances(&loud), loud);
    }

    #[test]
    fn a_noise_too_short_to_be_a_word_is_not_speech() {
        let mut loud = vec![false; 500];
        loud[200..200 + RUN_BINS - 1].fill(true);
        assert!(!utterances(&loud).iter().any(|bin| *bin));
    }

    #[test]
    fn the_quiet_inside_an_utterance_is_closed_up() {
        let mut loud = vec![false; 500];
        loud[100..200].fill(true);
        loud[200 + GAP_BINS - 1..300].fill(true);

        let gathered = utterances(&loud);
        assert!(gathered[100..300].iter().all(|bin| *bin));
        assert!(!gathered[99]);
        assert!(!gathered[300]);
    }

    #[test]
    fn the_quiet_between_two_utterances_is_left_alone() {
        let mut loud = vec![false; 500];
        loud[100..200].fill(true);
        loud[200 + GAP_BINS..300].fill(true);

        let gathered = utterances(&loud);
        assert!(!gathered[200]);
        assert!(!gathered[200 + GAP_BINS - 1]);
        assert!(gathered[200 + GAP_BINS]);
    }

    #[test]
    fn a_word_at_the_very_start_or_the_very_end_is_kept() {
        let mut loud = vec![false; 500];
        loud[..RUN_BINS].fill(true);
        loud[500 - RUN_BINS..].fill(true);
        let gathered = utterances(&loud);
        assert!(gathered[0]);
        assert!(gathered[499]);
    }

    #[test]
    fn nothing_at_all_is_not_a_panic() {
        assert!(speech(&[]).is_empty());
        assert!(utterances(&[]).is_empty());
        assert!(swing(&[]).is_empty());
    }

    #[test]
    fn a_film_shorter_than_one_window_still_has_a_floor() {
        let levels = film(WINDOW_BINS / 2, -60.0, -20.0, &[(500, 1_500)], &[]);
        let found = speech(&levels);
        assert!(found[1_000]);
        assert!(!found[200]);
    }
}
