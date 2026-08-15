//! Bringing a database up to the schema this build expects.
//!
//! Forward only, one numbered step at a time, each applied in its own
//! transaction and recorded as it goes. An empty file and a file from the
//! previous release therefore take the same path, and running the whole thing
//! twice does nothing the second time.

use rusqlite::{Connection, params};

use crate::clock::now_millis;
use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug)]
struct Migration {
    version: u32,
    name: &'static str,
    sql: &'static str,
}

/// Every step, in order. Append only.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial schema",
        sql: include_str!("migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "track correction",
        sql: include_str!("migrations/0002_track_correction.sql"),
    },
    Migration {
        version: 3,
        name: "chosen track",
        sql: include_str!("migrations/0003_track_choice.sql"),
    },
    Migration {
        version: 4,
        name: "track source",
        sql: include_str!("migrations/0004_track_source.sql"),
    },
    Migration {
        version: 5,
        name: "reread films",
        sql: include_str!("migrations/0005_reread_films.sql"),
    },
    Migration {
        version: 6,
        name: "drop cue search",
        sql: include_str!("migrations/0006_drop_cue_search.sql"),
    },
    Migration {
        version: 7,
        name: "film cover",
        sql: include_str!("migrations/0007_film_cover.sql"),
    },
    Migration {
        version: 8,
        name: "media details",
        sql: include_str!("migrations/0008_media_details.sql"),
    },
];

/// The schema version this build understands.
pub(crate) fn supported_version() -> u32 {
    MIGRATIONS.last().map_or(0, |migration| migration.version)
}

/// Applies whatever has not been applied yet.
///
/// Foreign keys are enforced on every connection and are turned off for the
/// length of this. A step that changes a constraint has to build the table
/// again and drop the old one, and dropping a table that other rows reference
/// would cascade those rows away: the cues of every track in the library, in
/// the one case there is. The pragma does nothing inside a transaction, so it
/// has to sit around the whole run rather than inside a step.
///
/// What that gives up is checked back afterwards, so a step that left a row
/// pointing at nothing is a failure to open the library rather than something
/// noticed later.
pub(crate) fn apply(connection: &mut Connection) -> Result<u32> {
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let outcome = migrate(connection);
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;

    let version = outcome?;
    if references_are_broken(connection)? {
        return Err(Error::Inconsistent);
    }
    Ok(version)
}

/// Whether anything is left pointing at a row that is not there.
fn references_are_broken(connection: &Connection) -> Result<bool> {
    let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
    let broken = statement.query([])?.next()?.is_some();
    Ok(broken)
}

fn migrate(connection: &mut Connection) -> Result<u32> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migration (
             version    INTEGER PRIMARY KEY,
             name       TEXT    NOT NULL,
             applied_at INTEGER NOT NULL
         ) STRICT;",
    )?;

    let current: u32 = connection.query_row(
        "SELECT coalesce(max(version), 0) FROM schema_migration",
        [],
        |row| row.get(0),
    )?;

    let supported = supported_version();
    if current > supported {
        return Err(Error::FromTheFuture {
            found: current,
            supported,
        });
    }

    for migration in MIGRATIONS
        .iter()
        .filter(|migration| migration.version > current)
    {
        let transaction = connection.transaction()?;
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migration (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![migration.version, migration.name, now_millis()],
        )?;
        transaction.commit()?;
    }

    Ok(supported)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use rusqlite::Connection;

    use super::{MIGRATIONS, apply, supported_version};
    use crate::error::Error;

    #[test]
    fn versions_are_in_order_and_start_at_one() {
        for (at, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(u64::from(migration.version), at as u64 + 1);
        }
    }

    #[test]
    fn applies_from_empty_and_then_does_nothing() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert_eq!(apply(&mut connection).unwrap(), supported_version());

        let applied: u32 = connection
            .query_row("SELECT count(*) FROM schema_migration", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(usize::try_from(applied).unwrap(), MIGRATIONS.len());

        // A second run is the ordinary case: every start of the application
        // calls this, and almost none of them have anything to do.
        apply(&mut connection).unwrap();
        let after: u32 = connection
            .query_row("SELECT count(*) FROM schema_migration", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(after, applied);
    }

    /// The case an appended migration exists for, and the one an empty
    /// database never exercises: a file a previous release wrote, brought
    /// forward without losing what is in it.
    #[test]
    fn a_database_from_an_earlier_version_is_brought_forward() {
        let mut connection = Connection::open_in_memory().unwrap();

        // Only the first step, which is what a release before this one left.
        let first = MIGRATIONS[0];
        connection.execute_batch(first.sql).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migration (
                     version    INTEGER PRIMARY KEY,
                     name       TEXT    NOT NULL,
                     applied_at INTEGER NOT NULL
                 ) STRICT;
                 INSERT INTO schema_migration (version, name, applied_at)
                 VALUES (1, 'initial schema', 0);",
            )
            .unwrap();

        connection
            .execute_batch(
                "INSERT INTO watched_folder (id, path, added_at) VALUES (1, '/films', 0);
                 INSERT INTO film (id, folder_id, path, title, size_bytes, modified_at, added_at)
                 VALUES (1, 1, '/films/Heat.mkv', 'Heat', 4000, 0, 0);
                 INSERT INTO subtitle_track (
                     id, film_id, path, forced, hearing_impaired,
                     match_kind, encoding, cue_count, size_bytes, modified_at
                 )
                 VALUES (1, 1, '/films/Heat.srt', 0, 0, 'exact', 'UTF-8', 1, 60, 0);
                 INSERT INTO cue (id, track_id, ordinal, start_ms, end_ms, text)
                 VALUES (1, 1, 1, 0, 1000, 'the action is the juice');",
            )
            .unwrap();

        assert_eq!(apply(&mut connection).unwrap(), supported_version());

        // The row is still there and says what it always said, and the columns
        // that were added say nothing, which is what the defaults are for.
        let (offset, rate): (i64, f64) = connection
            .query_row(
                "SELECT offset_ms, rate FROM subtitle_track WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(offset, 0);
        assert!((rate - 1.0).abs() < f64::EPSILON);

        // The same for the film, which now records which of its tracks it is
        // watched with and says nothing about a film brought forward.
        let (chosen, off): (Option<i64>, bool) = connection
            .query_row(
                "SELECT chosen_track_id, subtitles_off FROM film WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(chosen, None);
        assert!(!off);

        // A step that rebuilds a table has to drop the old one, and the cues
        // point at it. Losing them would mean every subtitle in the library
        // being read again, which is the one thing an upgrade must not do.
        let (origin, stream, codec, cues): (String, i64, String, i64) = connection
            .query_row(
                "SELECT t.origin, t.stream_number, t.codec, count(c.id)
                 FROM subtitle_track AS t LEFT JOIN cue AS c ON c.track_id = t.id
                 WHERE t.id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(origin, "sidecar");
        assert_eq!(stream, 0);
        assert_eq!(codec, "subrip");
        assert_eq!(cues, 1);

        // And the film has not been looked inside, which is what makes the
        // next scan look.
        let probed: Option<i64> = connection
            .query_row("SELECT probed_at FROM film WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(probed, None);
    }

    /// The upgrade the reading of embedded dialogue depends on.
    ///
    /// The release before this one opened each film, wrote down which tracks
    /// were inside it, and stopped there. Those films match what is on disk, so
    /// nothing would ever open them again, and their tracks would stay empty
    /// for ever.
    #[test]
    fn a_film_already_looked_inside_is_looked_inside_again() {
        let mut connection = Connection::open_in_memory().unwrap();

        // Everything up to the release that found tracks without reading them.
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        for migration in MIGRATIONS.iter().take_while(|it| it.version <= 4) {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute_batch(
                "CREATE TABLE schema_migration (
                     version    INTEGER PRIMARY KEY,
                     name       TEXT    NOT NULL,
                     applied_at INTEGER NOT NULL
                 ) STRICT;
                 INSERT INTO schema_migration (version, name, applied_at)
                 VALUES (1, 'a', 0), (2, 'b', 0), (3, 'c', 0), (4, 'd', 0);

                 INSERT INTO watched_folder (id, path, added_at) VALUES (1, '/films', 0);
                 INSERT INTO film (
                     id, folder_id, path, title, size_bytes, modified_at, added_at, probed_at
                 )
                 VALUES (1, 1, '/films/Heat.mkv', 'Heat', 4000, 0, 0, 1700000000000);
                 INSERT INTO subtitle_track (
                     id, film_id, path, origin, stream_number, codec, forced,
                     hearing_impaired, match_kind, encoding, cue_count, size_bytes, modified_at
                 )
                 VALUES (1, 1, '/films/Heat.mkv', 'stream', 3, 'subrip', 0, 0,
                         'exact', 'UTF-8', 0, 4000, 0);",
            )
            .unwrap();

        assert_eq!(apply(&mut connection).unwrap(), supported_version());

        let (probed, tracks): (Option<i64>, i64) = connection
            .query_row(
                "SELECT f.probed_at, count(t.id) FROM film AS f
                 LEFT JOIN subtitle_track AS t ON t.film_id = f.id
                 WHERE f.id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(probed, None);
        // The track keeps its row, so a chosen track and a correction still
        // point at what they pointed at. Only the cues under it are missing,
        // and the scan this puts the film back in the way of writes them.
        assert_eq!(tracks, 1);
    }

    /// A library file the last release wrote, opened by this one.
    ///
    /// The full text mirror over cue text goes, and nothing else does. The cues
    /// underneath it stay, since they are what opens a film without a parse and
    /// what an alignment measures against the speech in the film.
    #[test]
    fn a_library_from_the_last_release_keeps_everything_but_the_mirror() {
        let mut connection = Connection::open_in_memory().unwrap();

        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        for migration in MIGRATIONS.iter().take_while(|it| it.version <= 5) {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute_batch(
                "CREATE TABLE schema_migration (
                     version    INTEGER PRIMARY KEY,
                     name       TEXT    NOT NULL,
                     applied_at INTEGER NOT NULL
                 ) STRICT;
                 INSERT INTO schema_migration (version, name, applied_at)
                 VALUES (1, 'a', 0), (2, 'b', 0), (3, 'c', 0), (4, 'd', 0), (5, 'e', 0);

                 INSERT INTO watched_folder (id, path, added_at) VALUES (1, '/films', 0);
                 INSERT INTO film (
                     id, folder_id, path, title, size_bytes, modified_at, added_at, chosen_track_id
                 )
                 VALUES (1, 1, '/films/Heat.mkv', 'Heat', 4000, 0, 0, 1);
                 INSERT INTO subtitle_track (
                     id, film_id, path, origin, stream_number, codec, forced,
                     hearing_impaired, match_kind, encoding, cue_count, size_bytes,
                     modified_at, offset_ms, rate
                 )
                 VALUES (1, 1, '/films/Heat.srt', 'sidecar', 0, 'subrip', 0, 0,
                         'exact', 'UTF-8', 1, 60, 0, -1200, 1.0427);
                 INSERT INTO cue (id, track_id, ordinal, start_ms, end_ms, text)
                 VALUES (1, 1, 1, 0, 1000, 'the action is the juice');
                 INSERT INTO playback_position
                     (film_id, position_ms, duration_ms, finished, updated_at)
                 VALUES (1, 600000, 10260000, 0, 0);
                 INSERT INTO preference (key, value)
                 VALUES ('search.recent', '[\"juice\"]'), ('subtitles.size', '4.4');",
            )
            .unwrap();

        assert_eq!(apply(&mut connection).unwrap(), supported_version());

        let (cues, offset, chosen, position): (i64, i64, Option<i64>, i64) = connection
            .query_row(
                "SELECT count(c.id), t.offset_ms, f.chosen_track_id, p.position_ms
                 FROM subtitle_track AS t
                 JOIN film AS f ON f.id = t.film_id
                 JOIN playback_position AS p ON p.film_id = f.id
                 LEFT JOIN cue AS c ON c.track_id = t.id
                 WHERE t.id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(cues, 1);
        assert_eq!(offset, -1_200);
        assert_eq!(chosen, Some(1));
        assert_eq!(position, 600_000);

        // The film says nothing about where its cover comes from, which is what
        // puts it in the way of the next scan deciding.
        let cover: Option<String> = connection
            .query_row("SELECT cover_path FROM film WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cover, None);

        // Nor about what its file is, which is what puts it in the way of the
        // next scan describing it. Every one of these reads as not known rather
        // than as a number nobody wrote.
        for column in [
            "container",
            "video_codec",
            "video_width",
            "video_height",
            "bit_depth",
            "frame_rate",
        ] {
            let said: Option<String> = connection
                .query_row(
                    &format!("SELECT {column} FROM film WHERE id = 1"),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(said, None, "{column}");
        }

        // And it carries no sound tracks until something reads them, which is
        // an empty table rather than a missing one.
        let sound: i64 = connection
            .query_row("SELECT count(*) FROM audio_stream", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sound, 0);

        // The mirror and the flag that said whether it was being kept in step.
        let left: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE name IN ('cue_search', 'index_state', 'cue_indexed')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(left, 0);

        // The searches somebody made, which nothing reads now. Every other
        // preference is left where it was.
        let preferences: Vec<String> = connection
            .prepare("SELECT key FROM preference ORDER BY key")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(preferences, ["subtitles.size"]);
    }

    #[test]
    fn refuses_a_database_from_a_later_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migration (version, name, applied_at) VALUES (?1, 'later', 0)",
                [supported_version() + 1],
            )
            .unwrap();

        assert!(matches!(
            apply(&mut connection),
            Err(Error::FromTheFuture { .. })
        ));
    }
}
