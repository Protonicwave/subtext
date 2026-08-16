//! Finding the films a corpus can be built from, and reading each one twice.
//!
//! A film earns its place here by carrying a text subtitle track of its own.
//! That track was muxed against these exact frames, so its timings are the
//! film's own clock and can be taken as truth without anybody checking them by
//! hand. Everything else about the film is then real: a real mix, a real
//! mastering level, a real language, and audio nobody generated.
//!
//! Reading is the expensive half of a run. The audio of a two hour film takes
//! seconds to decode and every case built from that film is measured against the
//! same reading, so it is done once and kept. That is also why the films are all
//! read before any of them is measured: a mismatched case needs one film's
//! timings and another film's audio, and neither can be manufactured from a film
//! that has not been read yet.

use std::path::{Path, PathBuf};

use subtext_align::Signal;
use subtext_container::EmbeddedTrack;
use subtext_core::Cue;
use subtext_speech::Refusal;

use crate::report::Measured;

/// The fewest lines a track can be taken as truth from.
///
/// The application's own figure, and for the same reason: a forced or
/// signs-only track runs to a few dozen lines, which is not enough to tell one
/// lag from another. A track like that is a poor reference and would make a
/// case whose right answer nothing could be expected to find.
const FEWEST_CUES: usize = 100;

/// A film, its own timings, and where it talks.
#[derive(Debug)]
pub(crate) struct Film {
    pub(crate) title: String,
    pub(crate) path: PathBuf,
    /// The embedded text track, taken as the film's own clock.
    pub(crate) truth: Vec<Cue>,
    pub(crate) speech: Signal,
}

/// Why a film in the directory could not be used.
#[derive(Debug)]
pub(crate) enum Skipped {
    /// Nothing inside it to take as truth.
    NoTextTrack,
    /// A track, and too little of it.
    TooFewCues(usize),
    /// A soundtrack this build cannot read, or a file that will not open.
    Unread(Refusal),
}

impl Skipped {
    pub(crate) fn saying(&self) -> String {
        match self {
            Self::NoTextTrack => "no text subtitle track inside it".to_owned(),
            Self::TooFewCues(cues) => format!("only {cues} lines in its own track"),
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
pub(crate) fn gather(directory: &Path, limit: Option<usize>) -> Result<Gathered, String> {
    let paths = matroska_under(directory)?;
    if paths.is_empty() {
        return Err(format!(
            "no Matroska files under {}, and a corpus is built from films that carry their own subtitles",
            directory.display()
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
        match read(&path) {
            Ok(film) => {
                println!(
                    "  {title}: {} lines, {} minutes of audio",
                    film.truth.len(),
                    minutes(&film.speech)
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
/// The timings first, because a film with nothing to take as truth is no use
/// here however readable its soundtrack is, and reading a header is cheaper than
/// decoding two hours.
fn read(path: &Path) -> Result<Film, Skipped> {
    let inside = subtext_container::extract(path)
        .map_err(|why| Skipped::Unread(Refusal::Unreadable(why.to_string())))?;

    let truth = truest(inside).ok_or(Skipped::NoTextTrack)?;
    if truth.len() < FEWEST_CUES {
        return Err(Skipped::TooFewCues(truth.len()));
    }

    let speech = subtext_speech::speech_of(path).map_err(Skipped::Unread)?;
    Ok(Film {
        title: title_of(path),
        path: path.to_path_buf(),
        truth,
        speech,
    })
}

/// The track with the most lines in it.
///
/// More lines is a better reference: it covers more of the film, and a case
/// manufactured from it has more places where a wrong answer can show itself.
/// A picture based track carries nothing to read and arrives here empty, so it
/// falls out of this without a case of its own.
fn truest(inside: Vec<EmbeddedTrack>) -> Option<Vec<Cue>> {
    inside
        .into_iter()
        .map(|track| track.cues)
        .filter(|cues| !cues.is_empty())
        .max_by_key(Vec::len)
}

/// Every Matroska file under a directory, in a settled order.
///
/// Matroska only. A film carries the track that makes it usable here inside a
/// Matroska container or not at all, which is what the application already says
/// about embedded subtitles.
fn matroska_under(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];

    while let Some(at) = pending.pop() {
        let entries = std::fs::read_dir(&at)
            .map_err(|why| format!("{} could not be read: {why}", at.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mkv"))
            {
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
pub(crate) fn measured(film: &Film) -> Measured {
    Measured {
        title: film.title.clone(),
        path: film.path.display().to_string(),
        lines: film.truth.len(),
        minutes: minutes(&film.speech),
    }
}

/// How long a film runs, from how much of it was read.
fn minutes(speech: &Signal) -> u32 {
    u32::try_from(speech.len() * subtext_align::BIN_MS as usize / 60_000).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{matroska_under, title_of, truest};
    use subtext_container::{EmbeddedTrack, StreamTrack, SubtitleCodec};
    use subtext_core::{Cue, Timestamp};

    fn track(number: u64, cues: usize) -> EmbeddedTrack {
        EmbeddedTrack {
            track: StreamTrack {
                number,
                codec: SubtitleCodec::SubRip,
                language: Some("en"),
                default: false,
                forced: false,
                hearing_impaired: false,
            },
            cues: (0..cues)
                .map(|at| {
                    let start = u32::try_from(at * 4_000).unwrap_or(u32::MAX);
                    Cue {
                        index: u32::try_from(at + 1).unwrap_or(u32::MAX),
                        start: Timestamp::from_millis(start),
                        end: Timestamp::from_millis(start + 1_500),
                        text: "line".to_owned(),
                        position: None,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn the_track_with_the_most_lines_is_the_reference() {
        let inside = vec![track(1, 40), track(2, 900), track(3, 300)];
        let truth = truest(inside).expect("a reference");
        assert_eq!(truth.len(), 900);
    }

    /// A track of pictures arrives with nothing in it, and a film carrying only
    /// those has no reference at all rather than an empty one.
    #[test]
    fn a_film_with_nothing_to_read_has_no_reference() {
        assert!(truest(Vec::new()).is_none());
        assert!(truest(vec![track(1, 0)]).is_none());
    }

    /// Films are found below the directory as well as in it, anything that is
    /// not one is left alone, and two runs meet them in the same order so that
    /// two reports can be compared line by line.
    #[test]
    fn films_are_found_below_the_directory_and_in_a_settled_order() {
        let directory = tempfile::tempdir().expect("a directory");
        let root = directory.path();
        std::fs::create_dir(root.join("nineties")).expect("a subdirectory");

        for name in ["b.mkv", "a.mkv", "notes.txt", "nineties/c.MKV"] {
            std::fs::write(root.join(name), b"").expect("a file");
        }

        let found = matroska_under(root).expect("a walk");
        let mut names: Vec<String> = found.iter().map(|path| title_of(path)).collect();
        names.sort();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(found, matroska_under(root).expect("a second walk"));
    }
}
