//! The two preferences the back end reads for itself.
//!
//! Almost every preference belongs to the front end. It writes them, it reads
//! them back when the window opens, and nothing in Rust has an opinion about
//! what they mean. These two are the exceptions, because the decisions they
//! change are made before there is a screen to ask: how a folder is paired, and
//! how the webview is built.
//!
//! The names are written here and in `src/features/settings/schema.ts`. Those
//! are the only two places, and a value neither of them recognises reads as the
//! default rather than as a failure: it can only have got there by being edited
//! by hand or written by a later version, and neither is a reason to refuse to
//! scan.

use subtext_core::Matching;
use subtext_index::Database;

/// How much evidence a pairing needs.
const MATCHING: &str = "library.matching";

/// Whether the webview may decode video on the graphics card.
const HARDWARE_DECODING: &str = "playback.hardware";

pub(crate) fn matching(database: &Database) -> Matching {
    match stored(database, MATCHING).as_deref() {
        Some("exact") => Matching::Exact,
        _ => Matching::Relaxed,
    }
}

/// On unless somebody has turned it off, which is the right way round: it is
/// faster and cooler everywhere it works, and the reason to turn it off is a
/// driver that draws a green rectangle rather than a film.
pub(crate) fn hardware_decoding(database: &Database) -> bool {
    stored(database, HARDWARE_DECODING).as_deref() != Some("false")
}

/// What one preference says, or nothing where it has not been set and nothing
/// where the library could not be read. A preference is not worth failing over.
fn stored(database: &Database, key: &str) -> Option<String> {
    database.preferences().get(key).ok().flatten()
}
