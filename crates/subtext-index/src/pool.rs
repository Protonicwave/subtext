//! Connections, kept and reused.
//!
//! Scanning writes while the library reads, and both happen on whichever thread
//! the runtime picked, so a single shared connection would serialise the
//! application on itself. Opening one per query instead would pay for the
//! pragmas and the page cache every time. This keeps a small set of prepared
//! connections and hands them out for the length of one piece of work.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{Error, Result};

/// How many connections are kept for reuse.
///
/// Reads are short and writes are serialised by SQLite regardless, so a handful
/// is enough for any amount of concurrency the application produces. Anything
/// beyond this is opened, used and closed.
const IDLE_LIMIT: usize = 8;

#[derive(Debug)]
pub(crate) struct Pool {
    path: PathBuf,
    idle: Mutex<Vec<Connection>>,
}

impl Pool {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            idle: Mutex::new(Vec::new()),
        }
    }

    /// Runs one piece of work against a connection.
    ///
    /// The connection is returned to the pool afterwards, unless the work
    /// panicked, in which case it is dropped along with whatever state the
    /// panic left it in.
    pub(crate) fn with<T>(&self, work: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut connection = match self.take()? {
            Some(connection) => connection,
            None => self.open()?,
        };
        let outcome = work(&mut connection);
        self.release(connection);
        outcome
    }

    fn take(&self) -> Result<Option<Connection>> {
        let mut idle = self.idle.lock().map_err(|_| Error::Unavailable)?;
        Ok(idle.pop())
    }

    fn release(&self, connection: Connection) {
        if let Ok(mut idle) = self.idle.lock()
            && idle.len() < IDLE_LIMIT
        {
            idle.push(connection);
        }
    }

    fn open(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        prepare(&connection)?;
        Ok(connection)
    }
}

/// The pragmas every connection needs, in the order they need to be set.
///
/// Write-ahead logging is the important one: it lets the library be read while
/// a scan is still writing to it, which is the whole first-run experience.
/// `synchronous = NORMAL` is the matching trade: a power cut can cost the last
/// few transactions, and since every row can be rebuilt by rescanning the
/// folder, that is a cheap thing to risk for the write throughput.
///
/// Recursive triggers matter more than they look. Deleting a film cascades to
/// its cues, and without this the delete triggers that take those cues back out
/// of the search index would not fire, leaving the index claiming lines that no
/// longer exist.
fn prepare(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA recursive_triggers = ON;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -16384;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}
