//! Finding the films a corpus can be built from, and reading each one twice.
//!
//! A film earns its place here by carrying timings that describe it. The best
//! source is a text subtitle track inside the film, muxed against these exact
//! frames, which is the film's own clock and needs nothing checking. Where there
//! is none, a subtitle file beside the film will do, but only once it has been
//! shown to describe this film rather than claimed to: it is measured against
//! the film's own speech first, and a file that does not land is not truth and
//! is not used.
//!
//! Admitting a file that way is sound for the same reason the plan gives for not
//! worrying whether an embedded track is perfect. What a case measures is the
//! recovery of a perturbation somebody applied on purpose, so truth being a
//! little out moves the case and its answer by the same amount and changes
//! nothing. What truth may not be is a description of some other film, and that
//! is exactly what the landing figure catches.
//!
//! Nor is the check circular. It decides whether a file is admitted as truth. It
//! does not decide whether an answer was right, which is read off how far the
//! corrected lines sit from where truth puts them, and is independent of any
//! landing figure.
//!
//! Everything else about a film is then real: a real mix, a real mastering
//! level, a real language, and audio nobody generated.
//!
//! Reading is the expensive half of a run. The audio of a two hour film takes
//! seconds to decode and every case built from that film is measured against the
//! same reading, so it is done once and kept. That is also why the films are all
//! read before any of them is measured: a mismatched case needs one film's
//! timings and another film's audio, and neither can be manufactured from a film
//! that has not been read yet.

use std::path::{Path, PathBuf};

use subtext_align::{Signal, landing_of};
use subtext_container::EmbeddedTrack;
use subtext_core::{Correction, Cue};
use subtext_speech::Refusal;

use crate::report::Measured;

/// The fewest lines a track can be taken as truth from.
///
/// The application's own figure, and for the same reason: a forced or
/// signs-only track runs to a few dozen lines, which is not enough to tell one
/// lag from another. A track like that is a poor reference and would make a
/// case whose right answer nothing could be expected to find.
const FEWEST_CUES: usize = 100;

/// How much of a subtitle file beside a film has to land on that film's speech
/// before it is believed to be about that film.
///
/// Half of it. A track that has nothing to do with a film lands wherever an
/// utterance happens to fall near a line, which on dialogue every few seconds is
/// about an eighth; this is four times that. It also sits above the figure the
/// application will act on when it writes a correction, which is the useful way
/// to read it: a file admitted here is one the product would already consider
/// well placed on this film.
///
/// It is deliberately not higher. No film scores one, because whispers, lines
/// away from the microphone and dialogue under a loud mix are speech the reading
/// misses on every film there is, and a bar set near the top would throw away
/// good files on well mixed loud ones. Every admitted file has its figure printed
/// and written into the report, so the margin is visible rather than assumed.
pub(crate) const TRUTH_LANDS: f32 = 0.5;

/// The container extensions a film is looked for under.
///
/// Matroska for the tracks inside it, and MP4 because a great many libraries are
/// nothing else. An MP4 is never parsed for subtitles, which the application has
/// always refused to do; it is here because its audio can be read and a file
/// beside it can supply the timings.
const CONTAINERS: [&str; 2] = ["mkv", "mp4"];

/// Where a film's truth came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A text track inside the film, on the film's own clock.
    Inside,
    /// A file beside the film, shown to land on its speech.
    Beside,
}

impl Origin {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Inside => "inside the film",
            Self::Beside => "beside the film",
        }
    }
}

/// A film, its own timings, and where it talks.
#[derive(Debug)]
pub(crate) struct Film {
    pub(crate) title: String,
    pub(crate) path: PathBuf,
    /// The timings taken as the film's own clock.
    pub(crate) truth: Vec<Cue>,
    pub(crate) origin: Origin,
    /// How much of the truth lands on the film's speech before anything is done
    /// to it, which is what admitted a file from beside the film and is worth
    /// recording for one from inside it as well.
    ///
    /// Nothing where the audio could not be read, which is not a figure of zero.
    pub(crate) lands: Option<f32>,
    /// Where the film talks, where this build can hear it.
    ///
    /// A film that brought its own timings is kept without this. Measuring a
    /// subtitle against those timings needs no decoder at all, so a soundtrack
    /// in a codec this build has no reader for takes away the audio cases and
    /// leaves the exact ones, which is better than taking away the film.
    pub(crate) speech: Option<Signal>,
}

/// Why a film in the directory could not be used.
#[derive(Debug)]
pub(crate) enum Skipped {
    /// Nothing to take as truth: no text track inside it and no subtitle file
    /// beside it.
    NoTimings,
    /// Timings, and too few of them.
    TooFewCues(usize),
    /// A file beside the film that does not describe this film.
    DoesNotFit(f32),
    /// A soundtrack this build cannot read, or a file that will not open.
    Unread(Refusal),
}

impl Skipped {
    pub(crate) fn saying(&self) -> String {
        match self {
            Self::NoTimings => {
                "no text subtitle track inside it and no subtitle file beside it".to_owned()
            }
            Self::TooFewCues(cues) => format!("only {cues} lines to take as truth"),
            Self::DoesNotFit(lands) => format!(
                "the subtitle beside it lands on {:.0}% of the talking, so it is not this film's",
                f64::from(*lands) * 100.0
            ),
            Self::Unread(refusal) => refusal.to_string(),
        }
    }
}

/// What a directory turned out to hold.
#[derive(Debug)]
pub(crate) struct Gathered {
    pub(crate) films: Vec<Film>,
    /// The films that could not be used, and why, so that a run says what it
    /// left out rather than quietly measuring half a library.
    pub(crate) skipped: Vec<(String, Skipped)>,
}

/// Every film under `directory` that can be taken as truth, read and ready to
/// measure.
///
/// Reports what it is doing as it goes, because reading a directory of films is
/// minutes of work and a run that says nothing for ten of them is
/// indistinguishable from one that has hung.
///
/// # Errors
///
/// Only where the directory itself cannot be walked. A film that cannot be used
/// is skipped and named, not a failure: a library holds all sorts, and a run
/// over what is there is the point.
pub(crate) fn gather(
    directory: &Path,
    limit: Option<usize>,
    truth_lands: f32,
) -> Result<Gathered, String> {
    let paths = films_under(directory)?;
    if paths.is_empty() {
        return Err(format!(
            "no films under {}, looking for {}",
            directory.display(),
            CONTAINERS.join(" and ")
        ));
    }

    println!(
        "Reading {} films under {}",
        paths.len(),
        directory.display()
    );
    let mut films = Vec::new();
    let mut skipped = Vec::new();

    for path in paths {
        if limit.is_some_and(|most| films.len() >= most) {
            break;
        }
        let title = title_of(&path);
        match read(&path, truth_lands) {
            Ok(film) => {
                let heard = film.speech.as_ref().map_or_else(
                    || "its soundtrack could not be read".to_owned(),
                    |speech| {
                        format!(
                            "{} utterances in {} minutes, landing on {:.0}%",
                            utterances(speech),
                            minutes(speech),
                            f64::from(film.lands.unwrap_or_default()) * 100.0
                        )
                    },
                );
                println!(
                    "  {title}: {} lines from {}, {heard}",
                    film.truth.len(),
                    film.origin.name(),
                );
                films.push(film);
            }
            Err(why) => {
                println!("  {title}: skipped, {}", why.saying());
                skipped.push((title, why));
            }
        }
    }

    Ok(Gathered { films, skipped })
}

/// One film, read for its timings and then for its audio.
///
/// Whether there is anything to take as truth is settled before a sample is
/// decoded, so a film with neither a track inside it nor a file beside it costs
/// a header rather than two hours.
fn read(path: &Path, truth_lands: f32) -> Result<Film, Skipped> {
    let inside = inside_the_film(path);
    let beside = beside_the_film(path);
    if inside.is_none() && beside.is_empty() {
        return Err(Skipped::NoTimings);
    }

    let film = |truth: Vec<Cue>, origin, lands, speech| Film {
        title: title_of(path),
        path: path.to_path_buf(),
        truth,
        origin,
        lands,
        speech,
    };

    // A track inside the film is preferred without being measured first,
    // because it arrived on the film's own clock and there is nothing to doubt
    // about it. Its figure is still taken, for the report.
    if let Some(truth) = inside
        && truth.len() >= FEWEST_CUES
    {
        // The audio is wanted here and not required. This film has already
        // supplied timings that need no checking, and measuring a subtitle
        // against them decodes nothing, so a soundtrack this build cannot read
        // costs the audio cases and leaves the exact ones. A film like that is
        // precisely the one the exact path was built for, and skipping it would
        // leave that path with nothing to be measured on at all.
        let speech = subtext_speech::speech_of(path).ok();
        let lands = speech.as_ref().map(|speech| lands_on(&truth, speech));
        return Ok(film(truth, Origin::Inside, lands, speech));
    }

    // Truth from beside the film is a claim rather than a clock, and the only
    // thing that can check it is the film's own speech. So here the audio is
    // required, and a film whose soundtrack cannot be read has nothing that
    // could admit its subtitle.
    let speech = subtext_speech::speech_of(path).map_err(Skipped::Unread)?;
    let long_enough: Vec<Vec<Cue>> = beside
        .into_iter()
        .filter(|cues| cues.len() >= FEWEST_CUES)
        .collect();
    let most = long_enough.iter().map(Vec::len).max();
    let Some(best) = long_enough
        .into_iter()
        .map(|cues| {
            let lands = lands_on(&cues, &speech);
            (cues, lands)
        })
        // The one that fits the film best rather than the longest, since what
        // is being chosen is a description of this film and length is only
        // evidence of that where two files both fit.
        .max_by(|one, other| one.1.total_cmp(&other.1))
    else {
        return Err(Skipped::TooFewCues(most.unwrap_or(0)));
    };

    if best.1 < truth_lands {
        return Err(Skipped::DoesNotFit(best.1));
    }
    Ok(film(best.0, Origin::Beside, Some(best.1), Some(speech)))
}

/// How much of a track lands on a film's speech as it was written.
fn lands_on(cues: &[Cue], speech: &Signal) -> f32 {
    landing_of(cues, speech, Correction::IDENTITY).fraction()
}

/// The dialogue of the longest text track inside a film, where there is one.
///
/// More lines is a better reference: it covers more of the film, and a case
/// manufactured from it has more places where a wrong answer can show itself. A
/// picture based track carries nothing to read and arrives empty, so it falls
/// out of this without a case of its own. An MP4 arrives with nothing at all,
/// which is the application's standing decision rather than an omission here.
fn inside_the_film(path: &Path) -> Option<Vec<Cue>> {
    let inside = subtext_container::extract(path).ok()?;
    inside
        .into_iter()
        .map(|track: EmbeddedTrack| track.cues)
        .filter(|cues| !cues.is_empty())
        .max_by_key(Vec::len)
}

/// The dialogue of every subtitle file sitting beside a film.
///
/// A file counts as beside a film when it is in the same directory and its name
/// begins with the film's. That covers both the plain case and the language
/// suffixed one, which is the layout these files actually arrive in, and it is
/// deliberately narrow: a file whose name says nothing about this film has no
/// business being taken as a description of it.
fn beside_the_film(path: &Path) -> Vec<Vec<Cue>> {
    let (Some(directory), Some(stem)) = (path.parent(), path.file_stem()) else {
        return Vec::new();
    };
    let stem = stem.to_string_lossy().to_lowercase();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|beside| {
            beside
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("srt"))
                && beside
                    .file_stem()
                    .is_some_and(|other| other.to_string_lossy().to_lowercase().starts_with(&stem))
        })
        .collect();
    found.sort();

    found
        .iter()
        .filter_map(|beside| std::fs::read(beside).ok())
        .map(|bytes| subtext_core::parse_srt(&bytes).track.into_cues())
        .filter(|cues| !cues.is_empty())
        .collect()
}

/// Every film under a directory, in a settled order.
///
/// A directory inside the walk that cannot be read is stepped over rather than
/// ending the run. Somebody pointing this at the root of a drive will cross the
/// recycle bin and whatever the operating system keeps to itself, and refusing
/// to measure a library because Windows will not show its own volume information
/// would be answering the wrong question. The directory actually asked for is
/// different: if that cannot be read, there is nothing to do.
fn films_under(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];

    while let Some(at) = pending.pop() {
        let entries = match std::fs::read_dir(&at) {
            Ok(entries) => entries,
            Err(why) if at == directory => {
                return Err(format!("{} could not be read: {why}", at.display()));
            }
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| {
                CONTAINERS
                    .iter()
                    .any(|container| extension.eq_ignore_ascii_case(container))
            }) {
                found.push(path);
            }
        }
    }

    // Sorted so that two runs over the same directory measure the same films in
    // the same order, and two reports can be compared line by line.
    found.sort();
    Ok(found)
}

fn title_of(path: &Path) -> String {
    path.file_stem().map_or_else(
        || path.display().to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    )
}

/// A film as the report records it.
///
/// Everything read off the soundtrack is absent together, for a film whose
/// soundtrack this build cannot read. Nothing there is a figure of zero: a film
/// with no utterances and a film that was never listened to are different
/// things, and a report that wrote both as nought would not say which.
pub(crate) fn measured(film: &Film, baseline: Option<Correction>) -> Measured {
    Measured {
        baseline_offset_ms: baseline.map(Correction::offset_ms),
        baseline_rate: baseline.map(Correction::rate),
        title: film.title.clone(),
        path: film.path.display().to_string(),
        truth_from: film.origin.name(),
        truth_lands: film.lands,
        lines: film.truth.len(),
        utterances: film.speech.as_ref().map(utterances),
        minutes: film.speech.as_ref().map(minutes),
    }
}

/// How many separate times somebody starts talking in a film.
///
/// Worth recording beside the number of lines, because a line lands by arriving
/// as an utterance starts, so a reading that found far fewer utterances than the
/// track has lines puts a ceiling on the landing figure that no correction can
/// lift. Without this in the report, a film that cannot score well and a
/// correction that is wrong look the same afterwards.
fn utterances(speech: &Signal) -> u32 {
    let starts = (0..speech.len())
        .filter(|bin| speech.is_active(*bin) && (*bin == 0 || !speech.is_active(bin - 1)))
        .count();
    u32::try_from(starts).unwrap_or(u32::MAX)
}

/// How long a film runs, from how much of it was read.
fn minutes(speech: &Signal) -> u32 {
    u32::try_from(speech.len() * subtext_align::BIN_MS as usize / 60_000).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{beside_the_film, films_under, lands_on, title_of};
    use subtext_align::Signal;
    use subtext_core::{Cue, Timestamp};

    fn lines(count: u32, every_ms: u32) -> Vec<Cue> {
        (0..count)
            .map(|at| {
                let start = 10_000 + at * every_ms;
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

    /// A film that talks for a second and a half wherever those lines say it
    /// does.
    fn talking(cues: &[Cue]) -> Signal {
        Signal::from_cues(cues)
    }

    /// The check that admits a file beside a film: one written for the film
    /// lands on it, and one written for another film does not.
    #[test]
    fn a_subtitle_that_describes_the_film_lands_and_one_that_does_not_does_not() {
        let film = talking(&lines(200, 4_000));

        assert!(lands_on(&lines(200, 4_000), &film) > 0.9);
        // The same number of lines at a different spacing, which is what
        // another film's subtitle amounts to here.
        assert!(lands_on(&lines(200, 5_300), &film) < 0.5);
    }

    #[test]
    fn a_subtitle_beside_a_film_is_found_by_its_name() {
        let directory = tempfile::tempdir().expect("a directory");
        let root = directory.path();

        let srt = "1\n00:00:10,000 --> 00:00:12,000\nHello there.\n\n";
        std::fs::write(root.join("A Film.srt"), srt).expect("a sidecar");
        std::fs::write(root.join("A Film.en.srt"), srt).expect("a second sidecar");
        // Another film's, which is not beside this one however near it sits.
        std::fs::write(root.join("Another Film.srt"), srt).expect("a third sidecar");
        std::fs::write(root.join("A Film.mp4"), b"").expect("a film");

        let beside = beside_the_film(&root.join("A Film.mp4"));
        assert_eq!(beside.len(), 2);
        assert!(beside.iter().all(|cues| cues.len() == 1));
    }

    #[test]
    fn a_film_with_nothing_beside_it_has_nothing_beside_it() {
        let directory = tempfile::tempdir().expect("a directory");
        let root = directory.path();
        std::fs::write(root.join("A Film.mkv"), b"").expect("a film");

        assert!(beside_the_film(&root.join("A Film.mkv")).is_empty());
    }

    /// Films are found below the directory as well as in it, both containers
    /// are looked for, anything that is not one is left alone, and two runs meet
    /// them in the same order so that two reports can be compared line by line.
    #[test]
    fn films_are_found_below_the_directory_and_in_a_settled_order() {
        let directory = tempfile::tempdir().expect("a directory");
        let root = directory.path();
        std::fs::create_dir(root.join("nineties")).expect("a subdirectory");

        for name in [
            "b.mkv",
            "a.mp4",
            "notes.txt",
            "poster.jpg",
            "nineties/c.MKV",
        ] {
            std::fs::write(root.join(name), b"").expect("a file");
        }

        let found = films_under(root).expect("a walk");
        let mut names: Vec<String> = found.iter().map(|path| title_of(path)).collect();
        names.sort();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(found, films_under(root).expect("a second walk"));
    }
}
