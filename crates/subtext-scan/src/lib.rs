//! Scanning, watching and ingest for Subtext.
//!
//! Walks the watched folders, pairs films with subtitle files through
//! `subtext-core`, parses in parallel and writes to `subtext-index` in batches.
//! Watching keeps the library correct as files are added, changed or removed.
