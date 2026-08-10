//! Subtitle tracks and the cues inside them.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};
use subtext_core::{Cue, CuePosition, Timestamp};

use crate::database::Database;
use crate::error::Result;
use crate::model::{NewTrack, Stored, TrackMatch, TrackPairing, TrackRecord};
use crate::repository::{count_to_sql, from_sql_count, from_sql_int, path_text, to_sql_int};

const COLUMNS: &str = "id, film_id, path, language, forced, hearing_impaired, \
                       match_kind, encoding, cue_count, size_bytes, modified_at";

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

    /// Moves a track to a different film after a rescan changed its mind.
    ///
    /// This is for a file whose contents have not changed but whose best match
    /// has, which happens when the film it was paired with is joined in the
    /// folder by one that fits the name better. A pairing made by hand is left
    /// alone, and reports `false`.
    pub fn repoint(&self, id: i64, film_id: i64, match_kind: TrackMatch) -> Result<bool> {
        self.database.with(|connection| {
            let moved = connection.execute(
                "UPDATE subtitle_track SET film_id = ?2, match_kind = ?3
                 WHERE id = ?1 AND match_kind <> 'by-hand'",
                params![id, film_id, match_kind.as_str()],
            )?;
            Ok(moved > 0)
        })
    }

    /// The cues of a track in playback order, as the player and the transcript
    /// want them.
    pub fn cues(&self, track_id: i64) -> Result<Vec<Cue>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT ordinal, start_ms, end_ms, position, text
                 FROM cue WHERE track_id = ?1 ORDER BY start_ms, ordinal",
            )?;
            let cues = statement
                .query_map([track_id], |row| {
                    Ok(Cue {
                        index: row.get(0)?,
                        start: Timestamp::from_millis(row.get(1)?),
                        end: Timestamp::from_millis(row.get(2)?),
                        position: row
                            .get::<_, Option<u8>>(3)?
                            .and_then(CuePosition::from_alignment),
                        text: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(cues)
        })
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
    pub fn pairings(&self, folder_id: i64) -> Result<Vec<TrackPairing>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT t.id, t.film_id, t.path, t.match_kind, t.size_bytes, t.modified_at
                 FROM subtitle_track AS t
                 JOIN film AS f ON f.id = t.film_id
                 WHERE f.folder_id = ?1",
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
                "UPDATE subtitle_track SET film_id = ?2, match_kind = ?3 WHERE id = ?1",
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

const UPSERT: &str = "INSERT INTO subtitle_track (
         film_id, path, language, forced, hearing_impaired,
         match_kind, encoding, cue_count, size_bytes, modified_at
     )
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)
     ON CONFLICT (path) DO UPDATE SET
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
         modified_at = excluded.modified_at
     WHERE (subtitle_track.match_kind <> 'by-hand'
            AND subtitle_track.film_id IS NOT excluded.film_id)
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
        .prepare_cached("SELECT id FROM subtitle_track WHERE path = ?1")?
        .query_row([path], |row| row.get(0))?;
    Ok(Stored { id, changed: false })
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
        language: row.get(3)?,
        forced: row.get(4)?,
        hearing_impaired: row.get(5)?,
        match_kind: TrackMatch::from_stored(&row.get::<_, String>(6)?),
        encoding: row.get(7)?,
        cue_count: from_sql_count(row.get(8)?),
        size_bytes: from_sql_int(row.get(9)?),
        modified_at: row.get(10)?,
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
