//! Subtitle tracks and the cues inside them.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};
use subtext_core::{Correction, Cue, CuePosition, Timestamp};

use crate::clock::now_millis;
use crate::database::Database;
use crate::error::Result;
use crate::model::{
    FilmStreams, NewTrack, Stored, TrackMatch, TrackOrigin, TrackPairing, TrackRecord,
};
use crate::repository::{count_to_sql, from_sql_count, from_sql_int, path_text, to_sql_int};

const COLUMNS: &str = "id, film_id, path, language, forced, hearing_impaired, \
                       match_kind, encoding, cue_count, size_bytes, modified_at, \
                       offset_ms, rate, origin, stream_number, codec";

#[derive(Debug)]
pub struct Tracks<'a> {
    database: &'a Database,
}

impl<'a> Tracks<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Records a subtitle file, and returns its identifier.
    ///
    /// A pairing made by hand is never overwritten by a later scan. Someone who
    /// has told the application which subtitle belongs to a film should not have
    /// to tell it twice because the file was touched.
    pub fn upsert(&self, track: &NewTrack<'_>) -> Result<Stored> {
        self.database
            .with(|connection| upsert_one(connection, track))
    }

    /// Replaces the cues of a track with what the parser just read.
    ///
    /// One transaction and one prepared statement for the whole file. The
    /// alternative, a transaction per cue, turns a two thousand line film into
    /// two thousand fsyncs, which is the difference between a library indexing
    /// in seconds and in minutes.
    pub fn replace_cues(&self, track_id: i64, cues: &[Cue]) -> Result<usize> {
        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            replace_cues_on(&transaction, track_id, cues)?;
            transaction.commit()?;
            Ok(cues.len())
        })
    }

    /// Writes a batch of freshly parsed subtitle files, tracks and cues
    /// together, in one transaction.
    ///
    /// Together matters. A scan that is interrupted half way through must not
    /// leave a track row claiming a file has been read when its cues were never
    /// written: the next scan would see a fingerprint that matches, skip the
    /// file, and the film would have a transcript of nothing. Either both are
    /// there or neither is.
    pub fn write_batch(&self, entries: &[(NewTrack<'_>, &[Cue])]) -> Result<Vec<Stored>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            let mut stored = Vec::with_capacity(entries.len());
            for (track, cues) in entries {
                let written = upsert_one(&transaction, track)?;
                replace_cues_on(&transaction, written.id, cues)?;
                stored.push(written);
            }
            transaction.commit()?;
            Ok(stored)
        })
    }

    /// Records the tracks found inside a batch of films, and their dialogue,
    /// in one transaction.
    ///
    /// Each film's list replaces whatever was recorded for it before: a track
    /// the film no longer carries is taken away, and one it still carries keeps
    /// its identifier along with the choice and the correction that point at
    /// it. A film that was looked inside and found to carry nothing is passed
    /// with an empty list, which is how a film that lost its tracks is cleared.
    ///
    /// A track of pictures comes with no cues, since nothing turns those into
    /// words, and is recorded so that it can be named rather than read.
    ///
    /// Recording what a film carries also records that it was looked inside,
    /// in the same transaction, so a scan that stopped half way through does
    /// not leave a film marked as read with nothing to show for it.
    pub fn write_streams(&self, films: &[FilmStreams<'_>]) -> Result<usize> {
        if films.is_empty() {
            return Ok(0);
        }

        let at = now_millis();
        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            let mut written = 0;
            for (film_id, tracks) in films {
                for (track, cues) in tracks {
                    let stored = upsert_one(&transaction, track)?;
                    // Written whether or not the row itself changed. A film is
                    // only looked inside when it is new or has been replaced,
                    // so there is nothing to save by comparing, and a row that
                    // was recorded before there was any reading of dialogue
                    // would otherwise keep its empty transcript for ever.
                    replace_cues_on(&transaction, stored.id, cues)?;
                    written += 1;
                }

                let numbers: Vec<u64> = tracks
                    .iter()
                    .map(|(track, _)| track.stream_number)
                    .collect();
                remove_other_streams(&transaction, *film_id, &numbers)?;
                transaction
                    .prepare_cached("UPDATE film SET probed_at = ?2 WHERE id = ?1")?
                    .execute(params![film_id, at])?;
            }
            transaction.commit()?;
            Ok(written)
        })
    }

    /// Moves a track to a different film after a rescan changed its mind.
    ///
    /// This is for a file whose contents have not changed but whose best match
    /// has, which happens when the film it was paired with is joined in the
    /// folder by one that fits the name better. A pairing made by hand is left
    /// alone, and reports `false`.
    pub fn repoint(&self, id: i64, film_id: i64, match_kind: TrackMatch) -> Result<bool> {
        self.database.with(|connection| {
            let moved = connection.execute(
                &format!(
                    "UPDATE subtitle_track SET film_id = ?2, match_kind = ?3, {FORGET}
                 WHERE id = ?1 AND match_kind <> 'by-hand'"
                ),
                params![id, film_id, match_kind.as_str()],
            )?;
            Ok(moved > 0)
        })
    }

    /// The cues of a track in playback order, as the player and the transcript
    /// want them.
    ///
    /// Corrected on the way out. This is the point at which an authored timing
    /// becomes a playback timing, so nothing above here ever sees the other
    /// kind, and a track with no correction takes exactly the path it did
    /// before there was such a thing.
    pub fn cues(&self, track_id: i64) -> Result<Vec<Cue>> {
        self.database.with(|connection| {
            let correction = correction_of(connection, track_id)?;
            read_cues(connection, track_id, correction)
        })
    }

    /// The cues of a track as the file wrote them, with no correction applied.
    ///
    /// For working out a correction, and for nothing else. Measuring a track
    /// against its film means seeing the timings the file claims, since cues
    /// that have already been put through a correction would yield the residual
    /// of that correction rather than the track's own error, and asking twice
    /// would converge on nothing.
    ///
    /// Every other caller wants [`cues`](Self::cues). A line drawn over the
    /// picture from this would be drawn at the wrong moment for any track
    /// somebody has corrected.
    pub fn authored_cues(&self, track_id: i64) -> Result<Vec<Cue>> {
        self.database
            .with(|connection| read_cues(connection, track_id, Correction::IDENTITY))
    }

    /// The tracks of one film, the language a name was found for first.
    pub fn for_film(&self, film_id: i64) -> Result<Vec<TrackRecord>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {COLUMNS} FROM subtitle_track WHERE film_id = ?1
                 ORDER BY language IS NULL, language, forced, path"
            ))?;
            let tracks = statement
                .query_map([film_id], from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(tracks)
        })
    }

    pub fn by_id(&self, id: i64) -> Result<Option<TrackRecord>> {
        self.database.with(|connection| {
            let track = connection
                .query_row(
                    &format!("SELECT {COLUMNS} FROM subtitle_track WHERE id = ?1"),
                    [id],
                    from_row,
                )
                .optional()?;
            Ok(track)
        })
    }

    /// Records how a track's timings line up with its film.
    ///
    /// Written once, when somebody has finished nudging rather than while they
    /// are still doing it: every intermediate value would mean this write and a
    /// re-read of the whole track behind it.
    pub fn set_correction(&self, id: i64, correction: Correction) -> Result<bool> {
        self.database.with(|connection| {
            let written = connection.execute(
                "UPDATE subtitle_track SET offset_ms = ?2, rate = ?3 WHERE id = ?1",
                params![id, correction.offset_ms(), correction.rate()],
            )?;
            Ok(written > 0)
        })
    }

    pub fn by_path(&self, path: &Path) -> Result<Option<TrackRecord>> {
        let path = path_text(path)?;
        self.database.with(|connection| {
            let track = connection
                .query_row(
                    &format!("SELECT {COLUMNS} FROM subtitle_track WHERE path = ?1"),
                    [path],
                    from_row,
                )
                .optional()?;
            Ok(track)
        })
    }

    /// What a rescan compares the subtitle files in a folder against.
    ///
    /// The film and the kind of match come back alongside the fingerprint,
    /// because a rescan has two questions to ask of each file: whether it needs
    /// reading again, and whether it still belongs to the film it was paired
    /// with.
    ///
    /// Files only. A track inside a film is not paired with anything, is not
    /// found by walking a folder, and would read as a subtitle file that had
    /// been deleted if it came back here.
    pub fn pairings(&self, folder_id: i64) -> Result<Vec<TrackPairing>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT t.id, t.film_id, t.path, t.match_kind, t.size_bytes, t.modified_at
                 FROM subtitle_track AS t
                 JOIN film AS f ON f.id = t.film_id
                 WHERE f.folder_id = ?1 AND t.origin = 'sidecar'",
            )?;
            let pairings = statement
                .query_map([folder_id], |row| {
                    Ok(TrackPairing {
                        id: row.get(0)?,
                        film_id: row.get(1)?,
                        path: row.get::<_, String>(2)?.into(),
                        match_kind: TrackMatch::from_stored(&row.get::<_, String>(3)?),
                        size_bytes: from_sql_int(row.get(4)?),
                        modified_at: row.get(5)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(pairings)
        })
    }

    /// Attaches a subtitle file to a film by hand, which is what the amber row
    /// in the import sheet does.
    pub fn attach(&self, track_id: i64, film_id: i64) -> Result<()> {
        self.database.with(|connection| {
            connection.execute(
                &format!(
                    "UPDATE subtitle_track SET film_id = ?2, match_kind = ?3, {FORGET}
                     WHERE id = ?1"
                ),
                params![track_id, film_id, TrackMatch::ByHand.as_str()],
            )?;
            Ok(())
        })
    }

    /// Removes a track and its cues, taking them out of the search index too.
    pub fn remove(&self, id: i64) -> Result<bool> {
        self.database.with(|connection| {
            let removed = connection.execute("DELETE FROM subtitle_track WHERE id = ?1", [id])?;
            Ok(removed > 0)
        })
    }
}

/// Drops a correction, for a track that has been given to a different film.
///
/// A correction is a number somebody arrived at by ear against one film. Once
/// the track belongs to another it describes nothing, and carrying it over
/// would put a track that had been made right against one release out of step
/// with the next. A track staying where it is keeps what it was given, which is
/// why the film is compared rather than assumed to have changed. Both
/// expressions read the row as it was before this statement, so the comparison
/// is against the old film and not the new one.
const FORGET: &str = "offset_ms = CASE WHEN film_id = ?2 THEN offset_ms ELSE 0 END, \
                      rate = CASE WHEN film_id = ?2 THEN rate ELSE 1.0 END";

const UPSERT: &str = "INSERT INTO subtitle_track (
         film_id, path, language, forced, hearing_impaired,
         match_kind, encoding, cue_count, size_bytes, modified_at,
         origin, stream_number, codec
     )
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?11, ?12)
     ON CONFLICT (path, stream_number) DO UPDATE SET
         film_id = CASE WHEN subtitle_track.match_kind = 'by-hand'
                        THEN subtitle_track.film_id
                        ELSE excluded.film_id END,
         language = excluded.language,
         forced = excluded.forced,
         hearing_impaired = excluded.hearing_impaired,
         match_kind = CASE WHEN subtitle_track.match_kind = 'by-hand'
                           THEN subtitle_track.match_kind
                           ELSE excluded.match_kind END,
         encoding = excluded.encoding,
         size_bytes = excluded.size_bytes,
         modified_at = excluded.modified_at,
         codec = excluded.codec
     WHERE (subtitle_track.match_kind <> 'by-hand'
            AND subtitle_track.film_id IS NOT excluded.film_id)
        OR subtitle_track.codec IS NOT excluded.codec
        OR subtitle_track.language IS NOT excluded.language
        OR subtitle_track.forced IS NOT excluded.forced
        OR subtitle_track.hearing_impaired IS NOT excluded.hearing_impaired
        OR subtitle_track.encoding IS NOT excluded.encoding
        OR subtitle_track.size_bytes IS NOT excluded.size_bytes
        OR subtitle_track.modified_at IS NOT excluded.modified_at
        OR (subtitle_track.match_kind <> 'by-hand'
            AND subtitle_track.match_kind IS NOT excluded.match_kind)
     RETURNING id";

/// One track, against a connection that may already be inside a transaction.
fn upsert_one(connection: &Connection, track: &NewTrack<'_>) -> Result<Stored> {
    let path = path_text(track.path)?;
    let stream_number = to_sql_int(track.stream_number);
    let updated: Option<i64> = connection
        .prepare_cached(UPSERT)?
        .query_row(
            params![
                track.film_id,
                path,
                track.label.language,
                track.label.forced,
                track.label.hearing_impaired,
                track.match_kind.as_str(),
                track.encoding,
                to_sql_int(track.size_bytes),
                track.modified_at,
                track.origin.as_str(),
                stream_number,
                track.codec,
            ],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = updated {
        return Ok(Stored { id, changed: true });
    }

    // The file has not moved and has not been written to since it was last
    // read, so its cues are still the cues it has.
    let id = connection
        .prepare_cached("SELECT id FROM subtitle_track WHERE path = ?1 AND stream_number = ?2")?
        .query_row(params![path, stream_number], |row| row.get(0))?;
    Ok(Stored { id, changed: false })
}

/// Takes away the tracks a film used to carry inside it and no longer does.
///
/// A film re-encoded with fewer subtitles is the case this exists for. The
/// numbers that are still there are named rather than the ones that have gone,
/// since the probe knows the first and would have to work out the second.
fn remove_other_streams(connection: &Connection, film_id: i64, keeping: &[u64]) -> Result<()> {
    // A number no container gives a track, so a film that turned out to carry
    // none still leaves a list that parses and takes all of them away.
    let kept = std::iter::once("0".to_owned())
        .chain(keeping.iter().map(|number| to_sql_int(*number).to_string()))
        .collect::<Vec<_>>()
        .join(", ");

    // The numbers are integers this crate wrote and read back, so they go into
    // the statement rather than through a binding: SQLite has no way to bind a
    // list, and the alternative is a statement recompiled per track.
    connection.execute(
        &format!(
            "DELETE FROM subtitle_track
             WHERE film_id = ?1 AND origin = 'stream' AND stream_number NOT IN ({kept})"
        ),
        [film_id],
    )?;
    Ok(())
}

/// How a track's timings are to be read, or the identity if it has no row.
///
/// One small query before the cues rather than a join carrying the same two
/// numbers alongside every line of the film.
fn correction_of(connection: &Connection, track_id: i64) -> Result<Correction> {
    let found = connection
        .prepare_cached("SELECT offset_ms, rate FROM subtitle_track WHERE id = ?1")?
        .query_row([track_id], |row| {
            Ok(Correction::new(row.get(0)?, row.get(1)?))
        })
        .optional()?;
    Ok(found.unwrap_or(Correction::IDENTITY))
}

/// The lines of one track in playback order, put through `correction`.
///
/// The one query behind both reads, so that the corrected and the authored
/// timings can only ever differ by the arithmetic and not by the rows.
fn read_cues(connection: &Connection, track_id: i64, correction: Correction) -> Result<Vec<Cue>> {
    let mut statement = connection.prepare(
        "SELECT ordinal, start_ms, end_ms, position, text
         FROM cue WHERE track_id = ?1 ORDER BY start_ms, ordinal",
    )?;
    // A correction never reverses time, so the order the rows came back in is
    // still the order the lines are spoken in.
    let cues = statement
        .query_map([track_id], |row| {
            Ok(Cue {
                index: row.get(0)?,
                start: correction.apply(Timestamp::from_millis(row.get(1)?)),
                end: correction.apply(Timestamp::from_millis(row.get(2)?)),
                position: row
                    .get::<_, Option<u8>>(3)?
                    .and_then(CuePosition::from_alignment),
                text: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(cues)
}

/// The cues of one track, against a connection already inside a transaction.
fn replace_cues_on(connection: &Connection, track_id: i64, cues: &[Cue]) -> Result<()> {
    connection
        .prepare_cached("DELETE FROM cue WHERE track_id = ?1")?
        .execute([track_id])?;
    {
        let mut insert = connection.prepare_cached(
            "INSERT INTO cue (track_id, ordinal, start_ms, end_ms, position, text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for cue in cues {
            insert.execute(params![
                track_id,
                cue.index,
                cue.start.millis(),
                cue.end.millis(),
                cue.position.map(alignment_of),
                cue.text,
            ])?;
        }
    }
    connection
        .prepare_cached("UPDATE subtitle_track SET cue_count = ?2 WHERE id = ?1")?
        .execute(params![track_id, count_to_sql(cues.len())])?;
    Ok(())
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<TrackRecord> {
    Ok(TrackRecord {
        id: row.get(0)?,
        film_id: row.get(1)?,
        path: row.get::<_, String>(2)?.into(),
        origin: TrackOrigin::from_stored(&row.get::<_, String>(13)?),
        stream_number: from_sql_int(row.get(14)?),
        codec: row.get(15)?,
        language: row.get(3)?,
        forced: row.get(4)?,
        hearing_impaired: row.get(5)?,
        match_kind: TrackMatch::from_stored(&row.get::<_, String>(6)?),
        encoding: row.get(7)?,
        cue_count: from_sql_count(row.get(8)?),
        size_bytes: from_sql_int(row.get(9)?),
        modified_at: row.get(10)?,
        correction: Correction::new(row.get(11)?, row.get(12)?),
    })
}

/// The number a position is written as in a subtitle file.
///
/// The inverse of `CuePosition::from_alignment`, which is what reads it back.
/// Storing the number rather than a name keeps the column the same width as the
/// tag it came from.
fn alignment_of(position: CuePosition) -> u8 {
    match position {
        CuePosition::BottomLeft => 1,
        CuePosition::BottomCentre => 2,
        CuePosition::BottomRight => 3,
        CuePosition::MiddleLeft => 4,
        CuePosition::MiddleCentre => 5,
        CuePosition::MiddleRight => 6,
        CuePosition::TopLeft => 7,
        CuePosition::TopCentre => 8,
        CuePosition::TopRight => 9,
    }
}

#[cfg(test)]
mod tests {
    use subtext_core::CuePosition;

    use super::alignment_of;

    #[test]
    fn every_position_survives_the_round_trip() {
        for position in [
            CuePosition::TopLeft,
            CuePosition::TopCentre,
            CuePosition::TopRight,
            CuePosition::MiddleLeft,
            CuePosition::MiddleCentre,
            CuePosition::MiddleRight,
            CuePosition::BottomLeft,
            CuePosition::BottomCentre,
            CuePosition::BottomRight,
        ] {
            assert_eq!(
                CuePosition::from_alignment(alignment_of(position)),
                Some(position)
            );
        }
    }
}
