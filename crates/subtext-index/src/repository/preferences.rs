//! What the application has been told to prefer.
//!
//! Keys and values are text. The shapes behind them belong to the settings
//! screen and to the shell, and putting a column here for each would mean a
//! migration every time a control is added.

use rusqlite::{OptionalExtension, params};

use crate::database::Database;
use crate::error::Result;

#[derive(Debug)]
pub struct Preferences<'a> {
    database: &'a Database,
}

impl<'a> Preferences<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        self.database.with(|connection| {
            let value = connection
                .query_row(
                    "SELECT value FROM preference WHERE key = ?1",
                    [key],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(value)
        })
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        self.database.with(|connection| {
            connection.execute(
                "INSERT INTO preference (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value
                 WHERE preference.value IS NOT excluded.value",
                params![key, value],
            )?;
            Ok(())
        })
    }

    /// Every preference that has been set, by key.
    ///
    /// The settings screen reads the lot in one go at startup rather than a
    /// query per control.
    pub fn all(&self) -> Result<Vec<(String, String)>> {
        self.database.with(|connection| {
            let mut statement =
                connection.prepare("SELECT key, value FROM preference ORDER BY key")?;
            let preferences = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(preferences)
        })
    }

    /// Puts a preference back to whatever the application's own default is.
    pub fn remove(&self, key: &str) -> Result<bool> {
        self.database.with(|connection| {
            let removed = connection.execute("DELETE FROM preference WHERE key = ?1", [key])?;
            Ok(removed > 0)
        })
    }
}
