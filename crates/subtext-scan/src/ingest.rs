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
use subtext_core::{Cue, MatchKind, Matching, ParseWarning, SubtitleLabel, pair_with, parse_srt};
use subtext_index::{Database, NewFilm, NewTrack, Stored, TrackMatch, TrackPairing, WatchedFolder};

use crate::error::{Error, Result};
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

/// Above this many files to read, the search index is built once at the end
/// rather than kept in step row by row.
///
/// Building it in one pass costs roughly a tenth of what feeding a million cues
/// through the triggers does, but it rebuilds the index for the whole library
/// rather than for the part being written. So it is worth it for a scan large
/// enough to be dominated by its own writing, and not for one film being added
/// to a library that is already there.
const BULK_THRESHOLD: usize = 32;

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
    /// Subtitle files that belong to no film, which the import sheet offers to
    /// attach by hand.
    pub unpaired_subtitles: Vec<PathBuf>,
    /// Films with no subtitle at all, which get no transcript and no search.
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
/// Only one of these may run at a time against a given database, since a bulk
/// ingest stands the search index triggers down for the whole file. [`Scanner`]
/// is what enforces that.
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
    sink.report(&progress);

    let written = read_and_write_all(database, &plan.jobs, sink, progress)?;

    progress.stage = ScanStage::Finished;
    progress.subtitles_read = written.tracks;
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
        tracks_removed: plan.removed.len(),
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
    unreadable: Vec<PathBuf>,
    warnings: Vec<TrackWarnings>,
}

/// One parsed file on its way to the writer, or one that could not be read.
enum Message<'a> {
    Parsed(Box<Parsed<'a>>),
    Unreadable(PathBuf),
}

struct Parsed<'a> {
    job: &'a TrackJob,
    encoding: &'static str,
    cues: Vec<Cue>,
    warnings: Vec<ParseWarning>,
}

fn read_and_write_all(
    database: &Database,
    jobs: &[TrackJob],
    sink: &dyn ProgressSink,
    progress: ScanProgress,
) -> Result<Written> {
    if jobs.is_empty() {
        return Ok(Written::default());
    }
    if jobs.len() < BULK_THRESHOLD {
        return read_and_write(database, jobs, sink, progress);
    }

    // The index is rebuilt whether this succeeded or not, so the outcome is
    // carried through rather than turned into a failure the rebuild would skip.
    database.bulk_ingest(|| Ok(read_and_write(database, jobs, sink, progress)))?
}

fn read_and_write(
    database: &Database,
    jobs: &[TrackJob],
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

fn read_one(job: &TrackJob) -> Message<'_> {
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

fn write_batches(
    database: &Database,
    receiver: &Receiver<Message<'_>>,
    sink: &dyn ProgressSink,
    mut progress: ScanProgress,
) -> Result<Written> {
    let mut written = Written::default();
    let mut batch = Vec::with_capacity(BATCH_TRACKS);
    let mut queued_cues = 0;

    for message in receiver {
        match message {
            Message::Unreadable(path) => written.unreadable.push(path),
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
    Ok(written)
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
