//! What callers do with the database, rather than the SQL that does it.
//!
//! Every statement in the crate lives behind one of these. The rest of the
//! application asks for the films in a folder or the cues of a track, and never
//! sees a query, which is what makes the schema something that can be changed
//! without going looking for string literals in the front end.

mod details;
mod films;
mod folders;
mod positions;
mod preferences;
mod tracks;

pub use self::details::Details;
pub use self::films::Films;
pub use self::folders::Folders;
pub use self::positions::Positions;
pub use self::preferences::Preferences;
pub use self::tracks::Tracks;

/// The text a path goes into the database as.
///
/// Paths are stored as text so that they can be compared, matched and returned
/// to the front end. A path that is not valid UTF-8 is refused rather than
/// mangled, since a mangled path would silently stop matching the file it came
/// from.
pub(crate) fn path_text(path: &std::path::Path) -> crate::Result<&str> {
    path.to_str().ok_or(crate::Error::UnreadablePath)
}

/// A file size on the way into the database.
///
/// SQLite integers are signed, and nothing on a filesystem comes near the top
/// of one, so the saturating case is unreachable rather than lossy.
pub(crate) fn to_sql_int(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// A count or a size on the way back out. A negative value would mean the
/// column had been written by something other than this crate.
pub(crate) fn from_sql_int(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

/// A count on the way back out, for the places that index or allocate with it.
pub(crate) fn from_sql_count(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}

/// A count on the way in.
pub(crate) fn count_to_sql(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
