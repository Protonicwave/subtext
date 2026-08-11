//! Where the frame captured from each film is kept.
//!
//! One directory inside the application's own data, never beside the film. A
//! watched folder is read and not written to, and a cache that lives in one
//! place is a cache somebody can delete.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::dto::Failure;

/// The directory the posters are cached in, made if it is not there yet.
pub(crate) fn directory<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, Failure> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| Failure::saying("there is nowhere to keep the posters on this machine"))?
        .join("posters");
    std::fs::create_dir_all(&directory).map_err(Failure::of)?;
    Ok(directory)
}

/// What a film's poster is called.
///
/// Keyed by the path and the modification time together, so that replacing a
/// file with a different cut of the same film asks for a new frame rather than
/// showing the frame from the old one for ever. A film whose row still names a
/// different file is a film with no poster, which is how a stale one is noticed
/// without storing anything else to compare.
pub(crate) fn file_name(path: &Path, modified_at: i64) -> String {
    format!("{:016x}.webp", key(path, modified_at))
}

/// FNV-1a, over the path and the modification time.
///
/// Written out rather than taken from the standard library because
/// `DefaultHasher` makes no promise that two releases of Rust hash the same
/// bytes to the same value, and every cached poster in the application would be
/// orphaned by an upgrade that changed it. Collisions cost one film the wrong
/// frame until its file changes, which is why this is a hash and not a digest.
fn key(path: &Path, modified_at: i64) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

    let hash = eaten(OFFSET, path.as_os_str().as_encoded_bytes());
    eaten(hash, &modified_at.to_le_bytes())
}

fn eaten(mut hash: u64, bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    // A name that is not shaped as expected fails the assertion below it rather
    // than being unwrapped into something to compare.
    #![allow(clippy::expect_used)]

    use std::path::Path;

    use super::file_name;

    #[test]
    fn a_name_is_hexadecimal_and_a_webp() {
        let name = file_name(Path::new("/films/Heat.1995.mkv"), 1_700_000_000_000);

        let digits = name
            .strip_suffix(".webp")
            .expect("a poster is written as a WebP");

        assert_eq!(digits.len(), 16);
        assert!(digits.chars().all(|digit| digit.is_ascii_hexdigit()));
    }

    #[test]
    fn the_same_file_is_always_the_same_name() {
        let path = Path::new("/films/Heat.1995.mkv");

        assert_eq!(file_name(path, 12), file_name(path, 12));
    }

    #[test]
    fn a_file_that_changed_asks_for_a_new_frame() {
        let path = Path::new("/films/Heat.1995.mkv");

        assert_ne!(file_name(path, 12), file_name(path, 13));
        assert_ne!(
            file_name(path, 12),
            file_name(Path::new("/films/Ronin.mkv"), 12)
        );
    }
}
