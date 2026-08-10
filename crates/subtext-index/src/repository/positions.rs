//! Where each film was left.

use rusqlite::{OptionalExtension, Row, params};
use subtext_core::Timestamp;

use crate::clock::now_millis;
use crate::database::Database;
use crate::error::Result;
use crate::model::{PlaybackPosition, Resumable};
use crate::repository::{count_to_sql, films};

#[derive(Debug)]
pub struct Positions<'a> {
    database: &'a Database,
}

impl<'a> Positions<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Records how far through a film someone is.
    ///
    /// Called on a throttle while playing and once on exit, so it is written to
    /// be cheap: one row, replaced.
    pub fn save(
        &self,
        film_id: i64,
        position: Timestamp,
        duration: Option<Timestamp>,
        finished: bool,
    ) -> Result<()> {
        self.database.with(|connection| {
            connection.execute(
                "INSERT INTO playback_position (film_id, position_ms, duration_ms, finished, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (film_id) DO UPDATE SET
                     position_ms = excluded.position_ms,
                     duration_ms = coalesce(excluded.duration_ms, playback_position.duration_ms),
                     finished = excluded.finished,
                     updated_at = excluded.updated_at",
                params![
                    film_id,
                    position.millis(),
                    duration.map(Timestamp::millis),
                    finished,
                    now_millis(),
                ],
            )?;
            Ok(())
        })
    }

    pub fn get(&self, film_id: i64) -> Result<Option<PlaybackPosition>> {
        self.database.with(|connection| {
            let position = connection
                .query_row(
                    "SELECT film_id, position_ms, duration_ms, finished, updated_at
                     FROM playback_position WHERE film_id = ?1",
                    [film_id],
                    from_row,
                )
                .optional()?;
            Ok(position)
        })
    }

    /// The films to carry on with, most recently watched first.
    ///
    /// Missing files are included: the row of a film on an unplugged drive is
    /// the whole reason positions outlive their files, and the library shows it
    /// as missing rather than pretending it was never watched.
    pub fn resumable(&self, limit: usize) -> Result<Vec<Resumable>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {film_columns}, p.film_id, p.position_ms, p.duration_ms, p.finished, p.updated_at
                 FROM playback_position AS p
                 JOIN film AS f ON f.id = p.film_id
                 WHERE p.finished = 0
                 ORDER BY p.updated_at DESC
                 LIMIT ?1",
                film_columns = films::qualified_columns("f"),
            ))?;
            let resumable = statement
                .query_map([count_to_sql(limit)], |row| {
                    Ok(Resumable {
                        film: films::from_row(row)?,
                        position: from_row_at(row, films::COLUMN_COUNT)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(resumable)
        })
    }

    /// Forgets where a film was left, which is what starting it again from the
    /// beginning means.
    pub fn clear(&self, film_id: i64) -> Result<bool> {
        self.database.with(|connection| {
            let cleared = connection.execute(
                "DELETE FROM playback_position WHERE film_id = ?1",
                [film_id],
            )?;
            Ok(cleared > 0)
        })
    }
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<PlaybackPosition> {
    from_row_at(row, 0)
}

fn from_row_at(row: &Row<'_>, offset: usize) -> rusqlite::Result<PlaybackPosition> {
    Ok(PlaybackPosition {
        film_id: row.get(offset)?,
        position: Timestamp::from_millis(row.get(offset + 1)?),
        duration: row
            .get::<_, Option<i64>>(offset + 2)?
            .and_then(|millis| u32::try_from(millis).ok())
            .map(Timestamp::from_millis),
        finished: row.get(offset + 3)?,
        updated_at: row.get(offset + 4)?,
    })
}
