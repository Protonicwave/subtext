//! The films the library knows about.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};
use subtext_core::Timestamp;

use crate::clock::now_millis;
use crate::database::Database;
use crate::error::Result;
use crate::model::{FilmRecord, Fingerprint, NewFilm, Stored, TrackChoice, VideoDetails};
use crate::repository::{from_sql_int, path_text, to_sql_int};

const COLUMNS: &str = "id, folder_id, path, title, year, size_bytes, modified_at, \
                       duration_ms, poster_path, cover_path, accent, missing_since, \
                       chosen_track_id, subtitles_off, added_at, container, \
                       video_codec, video_width, video_height, bit_depth, frame_rate";

/// How many columns [`COLUMNS`] names, for queries that read a film alongside
/// something else and need to know where the film ends.
pub(super) const COLUMN_COUNT: usize = 21;

/// The same columns, qualified with a table alias for use in a join.
pub(super) fn qualified_columns(alias: &str) -> String {
    COLUMNS
        .split(',')
        .map(str::trim)
        .map(|column| format!("{alias}.{column}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug)]
pub struct Films<'a> {
    database: &'a Database,
}

impl<'a> Films<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Records a film found on disk, and returns its identifier.
    ///
    /// The update is guarded so that a film whose file has not changed is left
    /// exactly as it was. This is what makes rescanning a library that nobody
    /// has touched cost nothing: no rows are written, so nothing downstream
    /// thinks anything happened.
    ///
    /// Finding the file again also clears the missing mark, since the drive
    /// being plugged back in is the common reason for a path to reappear.
    pub fn upsert(&self, film: &NewFilm<'_>) -> Result<Stored> {
        self.database
            .with(|connection| upsert_one(connection, film, now_millis()))
    }

    /// Records every film a scan found, in one transaction.
    ///
    /// A folder of a thousand films is a thousand statements either way, but
    /// one commit rather than a thousand, which is most of the difference
    /// between a scan measured in seconds and one measured in minutes.
    pub fn upsert_many(&self, films: &[NewFilm<'_>]) -> Result<Vec<Stored>> {
        if films.is_empty() {
            return Ok(Vec::new());
        }

        // One timestamp for the whole batch, so that films found together are
        // ordered together rather than by how quickly they were written.
        let added_at = now_millis();
        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            let stored = films
                .iter()
                .map(|film| upsert_one(&transaction, film, added_at))
                .collect::<Result<Vec<_>>>()?;
            transaction.commit()?;
            Ok(stored)
        })
    }

    /// Every film, present or missing, by title.
    pub fn list(&self) -> Result<Vec<FilmRecord>> {
        self.query(
            &format!("SELECT {COLUMNS} FROM film ORDER BY title, year"),
            &[],
        )
    }

    pub fn in_folder(&self, folder_id: i64) -> Result<Vec<FilmRecord>> {
        self.query(
            &format!("SELECT {COLUMNS} FROM film WHERE folder_id = ?1 ORDER BY title, year"),
            &[&folder_id],
        )
    }

    pub fn by_id(&self, id: i64) -> Result<Option<FilmRecord>> {
        self.first(&format!("SELECT {COLUMNS} FROM film WHERE id = ?1"), &[&id])
    }

    pub fn by_path(&self, path: &Path) -> Result<Option<FilmRecord>> {
        let path = path_text(path)?;
        self.first(
            &format!("SELECT {COLUMNS} FROM film WHERE path = ?1"),
            &[&path],
        )
    }

    /// What a rescan compares the folder against before reading anything.
    pub fn fingerprints(&self, folder_id: i64) -> Result<Vec<Fingerprint>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, path, size_bytes, modified_at, missing_since
                 FROM film WHERE folder_id = ?1",
            )?;
            let fingerprints = statement
                .query_map([folder_id], |row| {
                    Ok(Fingerprint {
                        id: row.get(0)?,
                        path: row.get::<_, String>(1)?.into(),
                        size_bytes: from_sql_int(row.get(2)?),
                        modified_at: row.get(3)?,
                        missing: row.get::<_, Option<i64>>(4)?.is_some(),
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(fingerprints)
        })
    }

    /// The films in a folder that have never been looked inside.
    ///
    /// A film is opened for the tracks it carries when it is new or when its
    /// file has changed, which between them cover everything a scan does. They
    /// do not cover a library that was indexed before this build existed: those
    /// rows are unchanged, so nothing would ever open them. This is what asks.
    pub fn unprobed(&self, folder_id: i64) -> Result<Vec<i64>> {
        self.database.with(|connection| {
            let mut statement = connection
                .prepare("SELECT id FROM film WHERE folder_id = ?1 AND probed_at IS NULL")?;
            let ids = statement
                .query_map([folder_id], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(ids)
        })
    }

    /// The films in a folder whose file has never been described.
    ///
    /// A film is described when it is new or when its file has changed, which
    /// between them cover everything a scan does. They do not cover a library
    /// indexed before this build existed: those rows are unchanged and have
    /// already been looked inside, so nothing would ever open them again. This
    /// is what asks, and it costs a header read each rather than a whole film,
    /// since the dialogue in them was read by the build that came before.
    pub fn undescribed(&self, folder_id: i64) -> Result<Vec<i64>> {
        self.database.with(|connection| {
            let mut statement = connection
                .prepare("SELECT id FROM film WHERE folder_id = ?1 AND container IS NULL")?;
            let ids = statement
                .query_map([folder_id], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(ids)
        })
    }

    /// Where each film in a folder is drawing its cover from.
    ///
    /// A rescan needs this because deciding a cover is not only a question
    /// about what is on disk now: a film that has not changed is not opened
    /// again, so whether it carries its own artwork is something only the row
    /// still remembers.
    pub fn covers(&self, folder_id: i64) -> Result<Vec<(i64, Option<PathBuf>)>> {
        self.database.with(|connection| {
            let mut statement =
                connection.prepare("SELECT id, cover_path FROM film WHERE folder_id = ?1")?;
            let covers = statement
                .query_map([folder_id], |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, Option<String>>(1)?.map(Into::into),
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(covers)
        })
    }

    /// Records where each film's cover comes from, in one transaction.
    ///
    /// Guarded the way the upsert is, so a rescan of a library nobody has
    /// touched writes no rows. Passing nothing for a film says there is no
    /// image for it anywhere and a frame is the only answer left.
    pub fn set_covers(&self, covers: &[(i64, Option<&Path>)]) -> Result<usize> {
        if covers.is_empty() {
            return Ok(0);
        }

        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            let mut written = 0;
            {
                let mut statement = transaction.prepare_cached(
                    "UPDATE film SET cover_path = ?2 WHERE id = ?1 AND cover_path IS NOT ?2",
                )?;
                for (id, cover) in covers {
                    let cover = cover.map(path_text).transpose()?;
                    written += statement.execute(params![id, cover])?;
                }
            }
            transaction.commit()?;
            Ok(written)
        })
    }

    /// Marks films whose files are no longer there.
    ///
    /// They are kept rather than deleted, so that a drive being unplugged does
    /// not throw away where someone had got to in a three hour film. Marking a
    /// film that is already missing changes nothing, which keeps the original
    /// date rather than moving it forward on every scan.
    pub fn mark_missing(&self, ids: &[i64]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let at = now_millis();
        self.database.with(|connection| {
            let transaction = connection.transaction()?;
            let mut marked = 0;
            {
                let mut statement = transaction.prepare_cached(
                    "UPDATE film SET missing_since = ?2 WHERE id = ?1 AND missing_since IS NULL",
                )?;
                for id in ids {
                    marked += statement.execute(params![id, at])?;
                }
            }
            transaction.commit()?;
            Ok(marked)
        })
    }

    /// Records how long the film runs, which is not known until the player has
    /// opened it once.
    pub fn set_duration(&self, id: i64, duration: Timestamp) -> Result<()> {
        self.database.with(|connection| {
            connection.execute(
                "UPDATE film SET duration_ms = ?2 WHERE id = ?1 AND duration_ms IS NOT ?2",
                params![id, duration.millis()],
            )?;
            Ok(())
        })
    }

    /// Records which subtitle track a film is watched with.
    ///
    /// Kept on the film rather than as a flag on the track, because it is a
    /// choice between the tracks and not a property of any one of them: writing
    /// it here is one row whichever track is picked, and a film cannot end up
    /// with two tracks each believing they were chosen.
    ///
    /// A scan never touches this. The upsert above names the columns it writes
    /// and these are not among them, so a rescan leaves a choice where it was.
    pub fn set_choice(&self, id: i64, choice: TrackChoice) -> Result<bool> {
        let (track_id, off) = choice.columns();
        self.database.with(|connection| {
            let written = connection.execute(
                "UPDATE film SET chosen_track_id = ?2, subtitles_off = ?3 WHERE id = ?1",
                params![id, track_id, off],
            )?;
            Ok(written > 0)
        })
    }

    /// Records the captured poster and the colour pair taken from it.
    pub fn set_poster(&self, id: i64, poster: &Path, accent: Option<&str>) -> Result<()> {
        let poster = path_text(poster)?;
        self.database.with(|connection| {
            connection.execute(
                "UPDATE film SET poster_path = ?2, accent = ?3 WHERE id = ?1",
                params![id, poster, accent],
            )?;
            Ok(())
        })
    }

    /// Forgets a film outright, along with its tracks, cues and position.
    ///
    /// Scanning never calls this: a file that is gone is marked missing. It is
    /// here for someone removing a film by hand.
    pub fn remove(&self, id: i64) -> Result<bool> {
        self.database.with(|connection| {
            let removed = connection.execute("DELETE FROM film WHERE id = ?1", [id])?;
            Ok(removed > 0)
        })
    }

    fn query(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<FilmRecord>> {
        self.database.with(|connection| {
            let mut statement = connection.prepare(sql)?;
            let films = statement
                .query_map(params, from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(films)
        })
    }

    fn first(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Option<FilmRecord>> {
        self.database.with(|connection| {
            let film = connection.query_row(sql, params, from_row).optional()?;
            Ok(film)
        })
    }
}

const UPSERT: &str =
    "INSERT INTO film (folder_id, path, title, year, size_bytes, modified_at, added_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
     ON CONFLICT (path) DO UPDATE SET
         folder_id = excluded.folder_id,
         title = excluded.title,
         year = excluded.year,
         size_bytes = excluded.size_bytes,
         modified_at = excluded.modified_at,
         missing_since = NULL
     WHERE film.folder_id IS NOT excluded.folder_id
        OR film.title IS NOT excluded.title
        OR film.year IS NOT excluded.year
        OR film.size_bytes IS NOT excluded.size_bytes
        OR film.modified_at IS NOT excluded.modified_at
        OR film.missing_since IS NOT NULL
     RETURNING id";

/// One film, against a connection that may already be inside a transaction.
fn upsert_one(connection: &Connection, film: &NewFilm<'_>, added_at: i64) -> Result<Stored> {
    let path = path_text(film.path)?;
    let updated: Option<i64> = connection
        .prepare_cached(UPSERT)?
        .query_row(
            params![
                film.folder_id,
                path,
                film.title,
                film.year,
                to_sql_int(film.size_bytes),
                film.modified_at,
                added_at,
            ],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = updated {
        return Ok(Stored { id, changed: true });
    }

    // Nothing was written because nothing had changed, so the row is still
    // there to be asked for its identifier.
    let id = connection
        .prepare_cached("SELECT id FROM film WHERE path = ?1")?
        .query_row([path], |row| row.get(0))?;
    Ok(Stored { id, changed: false })
}

pub(super) fn from_row(row: &Row<'_>) -> rusqlite::Result<FilmRecord> {
    Ok(FilmRecord {
        id: row.get(0)?,
        folder_id: row.get(1)?,
        path: row.get::<_, String>(2)?.into(),
        title: row.get(3)?,
        year: row
            .get::<_, Option<i64>>(4)?
            .and_then(|year| u16::try_from(year).ok()),
        size_bytes: from_sql_int(row.get(5)?),
        modified_at: row.get(6)?,
        duration: row
            .get::<_, Option<i64>>(7)?
            .and_then(|millis| u32::try_from(millis).ok())
            .map(Timestamp::from_millis),
        poster_path: row.get::<_, Option<String>>(8)?.map(Into::into),
        cover_path: row.get::<_, Option<String>>(9)?.map(Into::into),
        accent: row.get(10)?,
        missing_since: row.get(11)?,
        choice: TrackChoice::from_columns(row.get(12)?, row.get(13)?),
        added_at: row.get(14)?,
        container: row.get(15)?,
        video: video_in(row)?,
    })
}

/// The picture a film's row describes, where it describes one.
///
/// The codec is what says there is a picture to describe. A film whose header
/// named no video track, or that has not been looked at, has nothing here, and
/// the dimensions of such a film are absent rather than zero.
fn video_in(row: &Row<'_>) -> rusqlite::Result<Option<VideoDetails>> {
    let Some(codec) = row.get::<_, Option<String>>(16)? else {
        return Ok(None);
    };

    Ok(Some(VideoDetails {
        codec,
        width: row.get::<_, Option<i64>>(17)?.and_then(narrow),
        height: row.get::<_, Option<i64>>(18)?.and_then(narrow),
        bit_depth: row
            .get::<_, Option<i64>>(19)?
            .and_then(|depth| u8::try_from(depth).ok()),
        frame_rate: row.get(20)?,
    }))
}

/// A dimension on the way out, where anything a column cannot mean reads as
/// nothing said rather than as a number nobody wrote.
fn narrow(value: i64) -> Option<u32> {
    u32::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::{COLUMN_COUNT, COLUMNS, qualified_columns};

    #[test]
    fn the_column_count_matches_the_columns() {
        assert_eq!(COLUMNS.split(',').count(), COLUMN_COUNT);
    }

    #[test]
    fn columns_can_be_qualified_for_a_join() {
        let qualified = qualified_columns("f");
        assert!(qualified.starts_with("f.id, f.folder_id, f.path"));
        assert!(qualified.ends_with("f.frame_rate"));
        assert_eq!(qualified.split(',').count(), COLUMN_COUNT);
    }
}
