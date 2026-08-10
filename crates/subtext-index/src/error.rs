//! What can go wrong between the application and the database.

use core::fmt;

/// The result of anything that touches the database.
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// SQLite refused something. Almost always a locked file, a full disk, or a
    /// bug in a query.
    Sqlite(rusqlite::Error),
    /// The database was written by a later version of Subtext than this one.
    ///
    /// Migrations only run forwards, so the honest thing is to stop rather than
    /// to work on a schema this build does not understand.
    FromTheFuture { found: u32, supported: u32 },
    /// A path that is not valid UTF-8, which cannot go into a text column.
    ///
    /// Rare enough that refusing the one file is better than storing paths as
    /// blobs everywhere to accommodate it.
    UnreadablePath,
    /// A connection was left behind by a thread that panicked while holding it.
    Unavailable,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "the library database refused the request: {error}"),
            Self::FromTheFuture { found, supported } => write!(
                f,
                "the library was written by a later version of Subtext \
                 (it is at version {found}, this build understands {supported})"
            ),
            Self::UnreadablePath => f.write_str("the path is not valid text"),
            Self::Unavailable => f.write_str("the library database is not available"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn says_which_version_is_the_problem() {
        let error = Error::FromTheFuture {
            found: 4,
            supported: 2,
        };
        assert!(error.to_string().contains("version 4"));
        assert!(error.to_string().contains("understands 2"));
    }
}
