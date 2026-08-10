//! Scanning, watching and ingest for Subtext.
//!
//! Walks the watched folders, pairs films with subtitle files through
//! `subtext-core`, parses in parallel and writes to `subtext-index` in batches.
//! Watching keeps the library correct as files are added, changed or removed.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! use std::path::Path;
//!
//! use subtext_index::Database;
//! use subtext_scan::{Scanner, Silent};
//!
//! let scanner = Scanner::new(Database::open("library.db")?);
//! let folder = scanner.add_folder(Path::new("/films"))?;
//! let outcome = scanner.scan(&folder, &Silent)?;
//!
//! println!("{} films, {} lines", outcome.films_found, outcome.cues_indexed);
//! # Ok(())
//! # }
//! ```

// Everything that touches the database fails the same way and says so through
// the same type. Repeating that under each method heading would add nothing.
#![allow(clippy::missing_errors_doc)]

mod debounce;
mod error;
mod ingest;
mod media;
mod progress;
mod scanner;
mod walk;
mod watch;

pub use crate::error::{Error, Result};
pub use crate::ingest::{ScanOutcome, TrackWarnings, scan_folder};
pub use crate::progress::{ProgressSink, ScanProgress, ScanStage, Silent};
pub use crate::scanner::Scanner;
pub use crate::walk::{Discovery, FoundFile, discover};
pub use crate::watch::FolderWatcher;
