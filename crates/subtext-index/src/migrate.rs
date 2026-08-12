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
];

/// The schema version this build understands.
pub(crate) fn supported_version() -> u32 {
    MIGRATIONS.last().map_or(0, |migration| migration.version)
}

/// Applies whatever has not been applied yet.
pub(crate) fn apply(connection: &mut Connection) -> Result<u32> {
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
                 VALUES (1, 1, '/films/Heat.srt', 0, 0, 'exact', 'UTF-8', 0, 60, 0);",
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
