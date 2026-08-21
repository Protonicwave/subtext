//! Turning a folder on disk into rows in the library.
//!
//! The shape of the pipeline is the point. Walking a folder is one thread and
//! almost free. Reading and parsing a thousand subtitle files is the expensive
//! part and is embarrassingly parallel. Writing is serial whatever we do,
//! because SQLite serialises writers anyway, so the parsers feed a single
//! writer through a bounded queue and the writer commits in batches. Bounded,
//! because a thousand parsed files ahead of the writer is a million cues
//! sitting in memory waiting their turn.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use subtext_container::{EmbeddedTrack, MediaStreams, SubtitleCodec};
use subtext_core::{
    Cover, Cue, MatchKind, Matching, ParseWarning, SubtitleLabel, Timestamp, pair_with, parse_srt,
};
use subtext_index::{
    AudioDetails, Database, FilmStreams, MediaDetails, NewFilm, NewTrack, Stored, StreamEntry,
    TrackMatch, TrackOrigin, TrackPairing, VideoDetails, WatchedFolder,
};

use crate::covers;
use crate::error::{Error, Result};
use crate::media;
use crate::progress::{ProgressSink, ScanProgress, ScanStage};
use crate::walk::{self, FoundFile};

/// How many subtitle files one transaction covers, and how many cues, whichever
/// comes first.
///
/// A batch too small pays for a commit too often. A batch too large holds the
/// write lock long enough for the library to feel unresponsive while a scan is
/// running, which is the thing write-ahead logging is there to avoid.
const BATCH_TRACKS: usize = 64;
const BATCH_CUES: usize = 20_000;

/// How many parsed files may wait for the writer.
///
/// This is the memory ceiling of a scan. Parsing runs ahead of writing on any
/// machine with more than a couple of cores, and without a bound it would run
/// ahead by the whole folder.
const QUEUE_DEPTH: usize = 128;

/// What a scan found and what it did about it.
#[derive(Clone, Debug)]
pub struct ScanOutcome {
    pub folder_id: i64,
    pub files_seen: usize,
    pub films_found: usize,
    pub subtitles_found: usize,
    /// Films with at least one subtitle file to their name.
    pub films_paired: usize,
    pub subtitles_read: usize,
    pub cues_indexed: usize,
    pub films_missing: usize,
    pub tracks_removed: usize,
    /// Films whose cover changed, which is what the report shown after a scan
    /// exists to describe. Nought for a rescan of a library nobody has touched,
    /// since deciding a cover writes only where the answer is different.
    pub covers_changed: usize,
    /// Films opened to see what they are and what subtitle tracks they carry.
    pub films_probed: usize,
    /// Subtitle tracks found inside those films.
    pub embedded_tracks: usize,
    /// Subtitle files that belong to no film, which the import sheet offers to
    /// attach by hand.
    pub unpaired_subtitles: Vec<PathBuf>,
    /// Films with no subtitle at all, which the import sheet marks as such.
    pub films_without_subtitles: Vec<PathBuf>,
    /// Files and folders that could not be read.
    pub unreadable: Vec<PathBuf>,
    pub warnings: Vec<TrackWarnings>,
}

/// What the parser had to work around in one file.
#[derive(Clone, Debug)]
pub struct TrackWarnings {
    pub path: PathBuf,
    pub warnings: Vec<ParseWarning>,
}

/// Scans one watched folder and brings the library into line with it.
///
/// The matching says how much evidence a pairing needs. Asking for more of it
/// than a pairing already in the library was made on takes that pairing away
/// again, since a rescan is the library agreeing with the folder afresh.
///
/// Only one of these may run at a time against a given database, since two of
/// them would be two sets of batches deciding what the same folder holds.
/// [`Scanner`] is what enforces that.
///
/// [`Scanner`]: crate::Scanner
pub fn scan_folder(
    database: &Database,
    folder: &WatchedFolder,
    matching: Matching,
    sink: &dyn ProgressSink,
) -> Result<ScanOutcome> {
    let mut progress = ScanProgress::new(folder.id);
    sink.report(&progress);

    let found = walk::discover(&folder.path);

    progress.stage = ScanStage::Pairing;
    progress.files_seen = found.files_seen;
    progress.films_found = found.films.len();
    progress.subtitles_found = found.subtitles.len();
    sink.report(&progress);

    let names = Names::of(&found);
    let report = pair_with(&names.films, &names.subtitles, matching);

    let known_films = database.films().fingerprints(folder.id)?;
    let films: Vec<NewFilm<'_>> = found
        .films
        .iter()
        .zip(&report.films)
        .map(|(file, name)| NewFilm {
            folder_id: folder.id,
            path: &file.path,
            title: &name.title,
            year: name.year,
            size_bytes: file.size_bytes,
            modified_at: file.modified_at,
        })
        .collect();
    let stored_films = database.films().upsert_many(&films)?;

    let on_disk: HashSet<&Path> = found.films.iter().map(|file| file.path.as_path()).collect();
    let vanished: Vec<i64> = known_films
        .iter()
        .filter(|film| !on_disk.contains(film.path.as_path()))
        .map(|film| film.id)
        .collect();
    let films_missing = database.films().mark_missing(&vanished)?;

    let films_to_open = films_to_read(
        &found.films,
        &stored_films,
        &database.films().unprobed(folder.id)?,
        &database.films().undescribed(folder.id)?,
    );

    let plan = Plan::draw_up(
        &found,
        &report,
        &stored_films,
        &database.tracks().pairings(folder.id)?,
    );

    for &id in &plan.removed {
        database.tracks().remove(id)?;
    }
    for &(id, film_id, match_kind) in &plan.repointed {
        database.tracks().repoint(id, film_id, match_kind)?;
    }

    progress.stage = ScanStage::Indexing;
    progress.films_paired = plan.films_with_subtitles.len();
    progress.subtitles_to_read = plan.jobs.len();
    progress.films_to_read = films_to_open.len();
    sink.report(&progress);

    let written = read_and_write_all(database, &plan.jobs, &films_to_open, sink, progress)?;

    let chosen = chosen_covers(
        &found,
        &report.films,
        &stored_films,
        &films_to_open,
        &written.carry_artwork,
        &database.films().covers(folder.id)?,
    );
    let covers: Vec<(i64, Option<&Cover>)> = chosen
        .iter()
        .map(|(id, cover)| (*id, cover.as_ref()))
        .collect();
    let covers_changed = database.films().set_covers(&covers)?;

    progress.stage = ScanStage::Finished;
    progress.subtitles_read = written.tracks;
    progress.films_read = written.films;
    progress.cues_indexed = written.cues;
    sink.report(&progress);

    let mut unreadable = found.unreadable;
    unreadable.extend(written.unreadable);

    Ok(ScanOutcome {
        folder_id: folder.id,
        files_seen: found.files_seen,
        films_found: found.films.len(),
        subtitles_found: found.subtitles.len(),
        films_paired: plan.films_with_subtitles.len(),
        subtitles_read: written.tracks,
        cues_indexed: written.cues,
        films_missing,
        covers_changed,
        tracks_removed: plan.removed.len(),
        films_probed: films_to_open.len(),
        embedded_tracks: written.streams,
        unpaired_subtitles: plan.unpaired,
        films_without_subtitles: films_without_subtitles(
            &found.films,
            &stored_films,
            &plan.films_with_subtitles,
        ),
        unreadable,
        warnings: written.warnings,
    })
}

/// The file names, held apart from the files so that pairing can borrow them.
struct Names<'a> {
    films: Vec<&'a str>,
    subtitles: Vec<&'a str>,
}

impl<'a> Names<'a> {
    fn of(found: &'a walk::Discovery) -> Self {
        Self {
            films: found
                .films
                .iter()
                .map(|file| file.file_name.as_str())
                .collect(),
            subtitles: found
                .subtitles
                .iter()
                .map(|file| file.file_name.as_str())
                .collect(),
        }
    }
}

/// How much of a film is read when it is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Depth {
    /// Everything: what the file is, and the dialogue inside it.
    Whole,
    /// The header alone, for a film whose dialogue an earlier build already
    /// read and whose technical facts it did not. Stepping over every frame of
    /// picture again to learn nothing new would turn one upgrade into an hour
    /// of somebody's machine.
    Header,
}

/// One film that needs opening to see what it is and what it carries.
#[derive(Clone, Debug)]
struct FilmJob {
    film_id: i64,
    path: PathBuf,
    /// What the name of the file says the film is in, which is the one fact
    /// about it that needs no reading.
    container: &'static str,
    depth: Depth,
    /// The film's own fingerprint, recorded on the tracks found inside it so
    /// that a row says which encode it was read out of.
    size_bytes: u64,
    modified_at: i64,
}

/// The films worth opening, and how far into each.
///
/// A film that has not changed since it was last looked inside is left shut,
/// and nothing is extracted from it. That is what keeps a rescan of a library
/// of Matroska files as cheap as a rescan of a library of anything else, and it
/// is why the row records when it was read rather than only what was found.
///
/// A film recorded by a build that read dialogue but described no files is the
/// one case in between. Its dialogue is where it should be and its row says
/// nothing about what the file is, so its header is read and its frames are
/// left alone.
fn films_to_read(
    films: &[FoundFile],
    stored: &[Stored],
    unprobed: &[i64],
    undescribed: &[i64],
) -> Vec<FilmJob> {
    let never: HashSet<i64> = unprobed.iter().copied().collect();
    let undescribed: HashSet<i64> = undescribed.iter().copied().collect();

    films
        .iter()
        .zip(stored)
        .filter_map(|(file, stored)| {
            let depth = if stored.changed || never.contains(&stored.id) {
                Depth::Whole
            } else if undescribed.contains(&stored.id) {
                Depth::Header
            } else {
                return None;
            };

            Some(FilmJob {
                film_id: stored.id,
                path: file.path.clone(),
                container: media::container_of(&file.file_name)?,
                depth,
                size_bytes: file.size_bytes,
                modified_at: file.modified_at,
            })
        })
        .collect()
}

/// One subtitle file that needs reading.
#[derive(Clone, Debug)]
struct TrackJob {
    path: PathBuf,
    film_id: i64,
    label: SubtitleLabel,
    match_kind: TrackMatch,
    size_bytes: u64,
    modified_at: i64,
}

/// What a scan decided to do before it did any of it.
#[derive(Debug, Default)]
struct Plan {
    jobs: Vec<TrackJob>,
    /// Tracks whose subtitle file is gone, or which now belong to no film.
    removed: Vec<i64>,
    /// Tracks whose file has not changed but whose film has.
    repointed: Vec<(i64, i64, TrackMatch)>,
    unpaired: Vec<PathBuf>,
    films_with_subtitles: HashSet<i64>,
}

impl Plan {
    fn draw_up(
        found: &walk::Discovery,
        report: &subtext_core::PairingReport,
        stored_films: &[Stored],
        known: &[TrackPairing],
    ) -> Self {
        let known: HashMap<&Path, &TrackPairing> = known
            .iter()
            .map(|pairing| (pairing.path.as_path(), pairing))
            .collect();

        // A film whose file was not found is missing rather than gone: an
        // unplugged drive takes its subtitle files with it. Its tracks are left
        // alone so that plugging the drive back in does not mean reparsing
        // everything on it.
        let present_films: HashSet<i64> = stored_films.iter().map(|stored| stored.id).collect();

        let mut paired = vec![None; found.subtitles.len()];
        for found_match in &report.matches {
            paired[found_match.subtitle] = Some((found_match.film, found_match.kind));
        }

        let mut plan = Self::default();
        for (at, file) in found.subtitles.iter().enumerate() {
            let existing = known.get(file.path.as_path()).copied();
            plan.consider(file, existing, paired[at], stored_films, &present_films);
        }

        let on_disk: HashSet<&Path> = found
            .subtitles
            .iter()
            .map(|file| file.path.as_path())
            .collect();
        plan.removed.extend(
            known
                .values()
                .filter(|pairing| !on_disk.contains(pairing.path.as_path()))
                .filter(|pairing| present_films.contains(&pairing.film_id))
                // A subtitle attached by hand may sit outside the folder it was
                // attached inside, so this walk not finding it is no evidence
                // that it has gone. Only the filesystem can say that, and it is
                // worth one stat for the few files anyone attaches themselves.
                .filter(|pairing| {
                    pairing.match_kind != TrackMatch::ByHand || !pairing.path.exists()
                })
                .map(|pairing| pairing.id),
        );
        plan.removed.sort_unstable();

        plan
    }

    fn consider(
        &mut self,
        file: &FoundFile,
        existing: Option<&TrackPairing>,
        paired: Option<(usize, MatchKind)>,
        stored_films: &[Stored],
        present_films: &HashSet<i64>,
    ) {
        // A pairing made by hand outranks anything the names say, in both
        // directions: it keeps its film, and it is never taken as unpaired.
        let attached_by_hand = existing.filter(|track| track.match_kind == TrackMatch::ByHand);
        let belongs_to = match attached_by_hand {
            Some(track) => Some((track.film_id, TrackMatch::ByHand)),
            None => paired.and_then(|(at, kind)| {
                stored_films
                    .get(at)
                    .map(|film| (film.id, TrackMatch::from(kind)))
            }),
        };

        let Some((film_id, match_kind)) = belongs_to else {
            self.unpaired.push(file.path.clone());
            if let Some(track) = existing.filter(|track| present_films.contains(&track.film_id)) {
                self.removed.push(track.id);
            }
            return;
        };

        self.films_with_subtitles.insert(film_id);

        match existing.filter(|track| track.matches(file.size_bytes, file.modified_at)) {
            // The file has not been written to since it was last read, so its
            // cues are still its cues and only the pairing can have moved.
            Some(track) => {
                if track.film_id != film_id || track.match_kind != match_kind {
                    self.repointed.push((track.id, film_id, match_kind));
                }
            }
            None => self.jobs.push(TrackJob {
                path: file.path.clone(),
                film_id,
                label: file_label(file),
                match_kind,
                size_bytes: file.size_bytes,
                modified_at: file.modified_at,
            }),
        }
    }
}

/// What the file name said the subtitle track was.
fn file_label(file: &FoundFile) -> SubtitleLabel {
    subtext_core::ParsedName::from_file_name(&file.file_name).label
}

/// Where each film's cover comes from, once the scan knows everything it is
/// going to know.
///
/// Three things have to meet for this: what the pictures and sidecars on the
/// disk claim, which is a question about names; what the films that were opened
/// turned out to carry inside them; and what the row already said about the
/// films that were not opened, since those have not changed and so neither has
/// the answer.
fn chosen_covers(
    found: &walk::Discovery,
    names: &[subtext_core::ParsedName],
    stored: &[Stored],
    opened: &[FilmJob],
    carry_artwork: &HashSet<i64>,
    recorded: &[(i64, Option<Cover>)],
) -> Vec<(i64, Option<Cover>)> {
    let on_disk = covers::on_disk(&found.films, names, &found.images, &found.sidecars);
    let opened: HashSet<i64> = opened.iter().map(|job| job.film_id).collect();
    let recorded: HashMap<i64, &Cover> = recorded
        .iter()
        .filter_map(|(id, cover)| Some((*id, cover.as_ref()?)))
        .collect();

    found
        .films
        .iter()
        .zip(stored)
        .zip(on_disk)
        .map(|((file, stored), on_disk)| {
            let cover = covers::decide(
                &file.path,
                opened
                    .contains(&stored.id)
                    .then(|| carry_artwork.contains(&stored.id)),
                recorded.get(&stored.id).copied(),
                &on_disk,
            );
            (stored.id, cover)
        })
        .collect()
}

fn films_without_subtitles(
    films: &[FoundFile],
    stored: &[Stored],
    with_subtitles: &HashSet<i64>,
) -> Vec<PathBuf> {
    films
        .iter()
        .zip(stored)
        .filter(|(_, stored)| !with_subtitles.contains(&stored.id))
        .map(|(file, _)| file.path.clone())
        .collect()
}

/// What the writer got through.
#[derive(Debug, Default)]
struct Written {
    tracks: usize,
    cues: usize,
    /// Films opened and read to the end.
    films: usize,
    /// Tracks found inside those films rather than beside them.
    streams: usize,
    /// Films that turned out to carry their own artwork.
    carry_artwork: HashSet<i64>,
    unreadable: Vec<PathBuf>,
    warnings: Vec<TrackWarnings>,
}

/// One file that has been read, or one that could not be.
enum Message<'a> {
    Parsed(Box<Parsed<'a>>),
    Probed(Probed<'a>),
    Unreadable(PathBuf),
}

struct Parsed<'a> {
    job: &'a TrackJob,
    encoding: &'static str,
    cues: Vec<Cue>,
    warnings: Vec<ParseWarning>,
}

struct Probed<'a> {
    job: &'a FilmJob,
    /// What the file turned out to be, which every film that was opened has.
    details: MediaDetails,
    /// The subtitle tracks inside the film, or nothing for a film whose frames
    /// were deliberately left alone.
    tracks: Option<Vec<EmbeddedTrack>>,
    /// Whether the film carries its own artwork, which is a walk over the
    /// attachment headers and none of the image.
    carries_artwork: bool,
}

/// One piece of reading, of either kind.
///
/// Both kinds go through the same parallel stage and the same writer. Probing a
/// film costs a fraction of what parsing a subtitle file does, so giving them a
/// stage of their own would leave every core but one idle for the length of it,
/// and would mean a second set of batches to commit.
enum Job<'a> {
    Subtitle(&'a TrackJob),
    Film(&'a FilmJob),
}

fn read_and_write_all(
    database: &Database,
    tracks: &[TrackJob],
    films: &[FilmJob],
    sink: &dyn ProgressSink,
    progress: ScanProgress,
) -> Result<Written> {
    let jobs: Vec<Job<'_>> = tracks
        .iter()
        .map(Job::Subtitle)
        .chain(films.iter().map(Job::Film))
        .collect();

    if jobs.is_empty() {
        return Ok(Written::default());
    }

    read_and_write(database, &jobs, sink, progress)
}

fn read_and_write(
    database: &Database,
    jobs: &[Job<'_>],
    sink: &dyn ProgressSink,
    progress: ScanProgress,
) -> Result<Written> {
    std::thread::scope(|scope| {
        let (sender, receiver) = mpsc::sync_channel(QUEUE_DEPTH);
        let writer = scope.spawn(move || write_batches(database, &receiver, sink, progress));

        // Set when the writer has stopped, so that the remaining files are
        // stepped over rather than read and parsed for a queue nobody is
        // draining. Relaxed is enough: it is a hint, and the worst a stale read
        // costs is one more file parsed than needed.
        let stopped = AtomicBool::new(false);
        jobs.par_iter().for_each_with(sender, |sender, job| {
            if stopped.load(Ordering::Relaxed) {
                return;
            }
            if sender.send(read_one(job)).is_err() {
                stopped.store(true, Ordering::Relaxed);
            }
        });

        writer.join().unwrap_or(Err(Error::Interrupted))
    })
}

fn read_one<'a>(job: &Job<'a>) -> Message<'a> {
    match job {
        Job::Subtitle(job) => parse_one(job),
        Job::Film(job) => read_film(job),
    }
}

fn parse_one(job: &TrackJob) -> Message<'_> {
    let Ok(bytes) = std::fs::read(&job.path) else {
        return Message::Unreadable(job.path.clone());
    };

    let outcome = parse_srt(&bytes);
    Message::Parsed(Box::new(Parsed {
        job,
        encoding: outcome.track.encoding(),
        warnings: outcome.warnings,
        cues: outcome.track.into_cues(),
    }))
}

/// What one film is, and what it carries inside it.
///
/// Reading the dialogue is the expensive half. The header is a few hundred
/// bytes, but the tracks inside a film are found by stepping over every frame
/// of picture between one line and the next, which is why it is done once per
/// film and never again while the file stays as it is.
///
/// A film that is not Matroska, or whose header makes no sense, reports the
/// container its name gives it and nothing else. That is the same answer a film
/// with nothing in it gives, and it is treated the same way: an MP4 is not
/// parsed here and must not appear to have been.
fn read_film(job: &FilmJob) -> Message<'_> {
    let Ok(found) = subtext_container::media(&job.path) else {
        return Message::Unreadable(job.path.clone());
    };

    let tracks = match job.depth {
        Depth::Whole => match subtext_container::extract(&job.path) {
            Ok(tracks) => Some(tracks),
            Err(_) => return Message::Unreadable(job.path.clone()),
        },
        Depth::Header => None,
    };

    Message::Probed(Probed {
        job,
        details: described(job.container, found),
        tracks,
        // Asked here because this is the one moment the film is open for
        // anything else. The image itself is left where it is until something
        // is going to draw it.
        carries_artwork: subtext_container::cover(&job.path).is_ok_and(|found| found.is_some()),
    })
}

/// What a film is, in the shape the library keeps it.
fn described(container: &str, found: MediaStreams) -> MediaDetails {
    MediaDetails {
        container: container.to_owned(),
        duration: found.duration_ms.map(Timestamp::from_millis),
        video: found.video.map(|picture| VideoDetails {
            codec: picture.codec,
            width: picture.width,
            height: picture.height,
            bit_depth: picture.bit_depth,
            frame_rate: picture.frame_rate,
        }),
        audio: found
            .audio
            .into_iter()
            .map(|sound| AudioDetails {
                stream_number: sound.number,
                codec: sound.codec,
                channels: sound.channels,
                language: sound.language.map(ToOwned::to_owned),
                default: sound.default,
            })
            .collect(),
    }
}

fn write_batches(
    database: &Database,
    receiver: &Receiver<Message<'_>>,
    sink: &dyn ProgressSink,
    mut progress: ScanProgress,
) -> Result<Written> {
    let mut written = Written::default();
    let mut batch = Vec::with_capacity(BATCH_TRACKS);
    let mut probed = Vec::with_capacity(BATCH_TRACKS);
    let mut queued_cues = 0;

    for message in receiver {
        match message {
            Message::Unreadable(path) => written.unreadable.push(path),
            Message::Probed(film) => {
                probed.push(film);
                if probed.len() >= BATCH_TRACKS {
                    flush_probed(database, &mut probed, &mut written)?;
                    progress.films_read = written.films;
                    progress.cues_indexed = written.cues;
                    sink.report(&progress);
                }
            }
            Message::Parsed(parsed) => {
                queued_cues += parsed.cues.len();
                batch.push(*parsed);
                if batch.len() >= BATCH_TRACKS || queued_cues >= BATCH_CUES {
                    flush(database, &mut batch, &mut written)?;
                    queued_cues = 0;
                    progress.subtitles_read = written.tracks;
                    progress.cues_indexed = written.cues;
                    sink.report(&progress);
                }
            }
        }
    }

    flush(database, &mut batch, &mut written)?;
    flush_probed(database, &mut probed, &mut written)?;
    Ok(written)
}

/// Records what a batch of films turned out to be, and what they carry.
///
/// A film with nothing in it is written too. Being looked inside and found to
/// hold nothing is an answer, and it is the one that stops the film being
/// opened again on every scan for the rest of its life.
///
/// The two writes are separate because they answer for different sets of films.
/// Every film that was opened is described. Only the ones whose frames were
/// read say what subtitle tracks they carry, and a film whose header alone was
/// read must not have its dialogue cleared on the strength of a list nothing
/// went looking for.
fn flush_probed(
    database: &Database,
    batch: &mut Vec<Probed<'_>>,
    written: &mut Written,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    let described: Vec<(i64, &MediaDetails)> = batch
        .iter()
        .map(|film| (film.job.film_id, &film.details))
        .collect();
    database.details().record(&described)?;

    let films: Vec<FilmStreams<'_>> = batch
        .iter()
        .filter_map(|film| {
            let tracks = film
                .tracks
                .as_ref()?
                .iter()
                .map(|found| stream_track(film.job, found))
                .collect();
            Some((film.job.film_id, tracks))
        })
        .collect();

    written.streams += database.tracks().write_streams(&films)?;
    written.films += batch.len();
    for film in batch.drain(..) {
        if film.carries_artwork {
            written.carry_artwork.insert(film.job.film_id);
        }
        written.cues += film
            .tracks
            .iter()
            .flatten()
            .map(|found| found.cues.len())
            .sum::<usize>();
    }
    Ok(())
}

/// One track inside a film, as a row and the lines under it.
fn stream_track<'a>(job: &'a FilmJob, found: &'a EmbeddedTrack) -> StreamEntry<'a> {
    let track = NewTrack {
        film_id: job.film_id,
        path: &job.path,
        origin: TrackOrigin::Stream,
        stream_number: found.track.number,
        codec: found.track.codec.as_str(),
        label: SubtitleLabel {
            language: found.track.language,
            forced: found.track.forced,
            hearing_impaired: found.track.hearing_impaired,
        },
        // A track inside a film belongs to it by construction, which is a
        // stronger statement than any pairing of names could make.
        match_kind: TrackMatch::Exact,
        // Text inside a container is UTF-8 by specification, so there is
        // nothing to detect and nothing to be wrong about.
        encoding: "UTF-8",
        // The film's own fingerprint rather than the track's, since the track
        // has no existence apart from the file it was read out of.
        size_bytes: job.size_bytes,
        modified_at: job.modified_at,
    };

    (track, found.cues.as_slice())
}

fn flush(database: &Database, batch: &mut Vec<Parsed<'_>>, written: &mut Written) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    {
        let entries: Vec<(NewTrack<'_>, &[Cue])> = batch
            .iter()
            .map(|parsed| {
                (
                    NewTrack {
                        film_id: parsed.job.film_id,
                        path: &parsed.job.path,
                        origin: TrackOrigin::Sidecar,
                        stream_number: 0,
                        codec: SubtitleCodec::SubRip.as_str(),
                        label: parsed.job.label,
                        match_kind: parsed.job.match_kind,
                        encoding: parsed.encoding,
                        size_bytes: parsed.job.size_bytes,
                        modified_at: parsed.job.modified_at,
                    },
                    parsed.cues.as_slice(),
                )
            })
            .collect();
        database.tracks().write_batch(&entries)?;
    }

    written.tracks += batch.len();
    for parsed in batch.drain(..) {
        written.cues += parsed.cues.len();
        if !parsed.warnings.is_empty() {
            written.warnings.push(TrackWarnings {
                path: parsed.job.path.clone(),
                warnings: parsed.warnings,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Depth, FilmJob, films_to_read};
    use crate::walk::FoundFile;
    use subtext_index::Stored;

    fn found(name: &str) -> FoundFile {
        FoundFile {
            path: format!("/films/{name}").into(),
            file_name: name.to_owned(),
            size_bytes: 4_000,
            modified_at: 1_700_000_000_000,
        }
    }

    fn depths(jobs: &[FilmJob]) -> Vec<(i64, Depth)> {
        jobs.iter().map(|job| (job.film_id, job.depth)).collect()
    }

    #[test]
    fn a_film_is_opened_when_it_is_new_or_has_been_replaced() {
        let films = [found("Heat.mkv"), found("Ronin.mkv")];
        let stored = [
            Stored {
                id: 1,
                changed: true,
            },
            Stored {
                id: 2,
                changed: false,
            },
        ];

        // The second has been read before and has not moved, so it is left shut
        // however long the library is rescanned for.
        assert_eq!(
            depths(&films_to_read(&films, &stored, &[], &[])),
            [(1, Depth::Whole)]
        );

        // And one that has never been looked inside, whatever its fingerprint
        // says.
        assert_eq!(
            depths(&films_to_read(&films, &stored, &[2], &[])),
            [(1, Depth::Whole), (2, Depth::Whole)]
        );
    }

    /// The case an upgrade leaves behind: dialogue already read by the build
    /// before this one, and nothing said about what the file is.
    #[test]
    fn a_film_that_has_never_been_described_gives_up_its_header_only() {
        let films = [found("Heat.mkv")];
        let stored = [Stored {
            id: 1,
            changed: false,
        }];

        assert_eq!(
            depths(&films_to_read(&films, &stored, &[], &[1])),
            [(1, Depth::Header)]
        );

        // A film that is new is read whole, and describing it comes with that
        // rather than instead of it.
        let fresh = [Stored {
            id: 1,
            changed: true,
        }];
        assert_eq!(
            depths(&films_to_read(&films, &fresh, &[], &[1])),
            [(1, Depth::Whole)]
        );
    }

    #[test]
    fn every_film_opened_knows_what_container_it_is_in() {
        let films = [found("Heat.mkv"), found("Ronin.mp4")];
        let stored = [
            Stored {
                id: 1,
                changed: true,
            },
            Stored {
                id: 2,
                changed: true,
            },
        ];

        let containers: Vec<&str> = films_to_read(&films, &stored, &[], &[])
            .iter()
            .map(|job| job.container)
            .collect();
        assert_eq!(containers, ["Matroska", "MP4"]);
    }
}
