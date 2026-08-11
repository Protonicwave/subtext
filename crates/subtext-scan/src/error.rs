//! What can go wrong between a folder and the library.

use core::fmt;
use std::path::PathBuf;

/// The result of anything in this crate.
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// The database refused something.
    Database(subtext_index::Error),
    /// A folder could not be watched, which on every platform means either that
    /// it is not there or that the operating system ran out of watches.
    Watch { path: PathBuf, reason: String },
    /// The platform's file change notification could not be started at all, so
    /// folders will have to be rescanned by hand.
    WatchUnavailable(String),
    /// A scan was cut short by a thread that gave up part way through.
    ///
    /// Nothing is left half written: batches are committed whole, so this
    /// means some files were not read, not that some were read badly.
    Interrupted,
    /// A subtitle file could not be given to a film by hand.
    Attach { path: PathBuf, reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(f, "{error}"),
            Self::Watch { path, reason } => {
                write!(f, "{} could not be watched: {reason}", path.display())
            }
            Self::WatchUnavailable(reason) => {
                write!(f, "folders cannot be watched for changes: {reason}")
            }
            Self::Interrupted => f.write_str("the scan stopped before it had finished"),
            Self::Attach { path, reason } => {
                write!(f, "{} could not be attached: {reason}", path.display())
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<subtext_index::Error> for Error {
    fn from(error: subtext_index::Error) -> Self {
        Self::Database(error)
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn says_which_folder_could_not_be_watched() {
        let error = Error::Watch {
            path: "/films".into(),
            reason: "no such directory".to_owned(),
        };
        assert!(error.to_string().contains("/films"));
        assert!(error.to_string().contains("no such directory"));
    }
}
