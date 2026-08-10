//! Persistence and search for Subtext.
//!
//! Wraps a single SQLite database holding watched folders, films, subtitle
//! tracks, cues, playback positions and preferences, together with the full text
//! index over cue text. Callers work through repositories rather than SQL.
