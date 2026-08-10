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
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial schema",
    sql: include_str!("migrations/0001_initial.sql"),
}];

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
