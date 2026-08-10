//! Waiting for a folder to stop changing.
//!
//! A download does not arrive as one event. It arrives as a file being created,
//! then written to a few hundred times, then closed, and a scan started at the
//! first of those would read a file that is a tenth of the way there. So
//! changed paths are collected and held until the folder has been quiet for a
//! moment, and only then handed on.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How long a folder waits before what changed in it is acted on.
///
/// Short enough that a file dropped into a watched folder appears in the
/// library while the person who dropped it is still looking at the window, and
/// long enough to cover the gap between the chunks of a file being copied.
pub(crate) const QUIET: Duration = Duration::from_millis(300);

/// The longest a run of changes may hold up the ones before it.
///
/// Something writing continuously, a recording or a very slow download, would
/// otherwise never be quiet and would keep everything that changed alongside it
/// waiting indefinitely.
pub(crate) const PATIENCE: Duration = Duration::from_secs(30);

/// Paths that have changed, waiting for the folder to settle.
#[derive(Debug)]
pub(crate) struct Settling {
    quiet: Duration,
    patience: Duration,
    // A set, because the two hundred writes that make up one file are two
    // hundred events naming the same path.
    paths: BTreeSet<PathBuf>,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Settling {
    pub(crate) fn new(quiet: Duration, patience: Duration) -> Self {
        Self {
            quiet,
            patience,
            paths: BTreeSet::new(),
            first: None,
            last: None,
        }
    }

    pub(crate) fn touch(&mut self, path: PathBuf, now: Instant) {
        self.paths.insert(path);
        self.first.get_or_insert(now);
        self.last = Some(now);
    }

    /// The paths that changed, once nothing more has changed for a while.
    pub(crate) fn take_if_settled(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        let quiet_for = now.saturating_duration_since(self.last?) >= self.quiet;
        let waited = now.saturating_duration_since(self.first?) >= self.patience;
        (quiet_for || waited).then(|| self.take())
    }

    fn take(&mut self) -> Vec<PathBuf> {
        self.first = None;
        self.last = None;
        core::mem::take(&mut self.paths).into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Settling;

    const QUIET: Duration = Duration::from_millis(300);
    const PATIENCE: Duration = Duration::from_secs(30);

    #[test]
    fn nothing_changed_means_nothing_to_do() {
        let mut settling = Settling::new(QUIET, PATIENCE);
        assert!(settling.take_if_settled(Instant::now()).is_none());
    }

    #[test]
    fn a_file_written_in_chunks_is_one_change() {
        let start = Instant::now();
        let mut settling = Settling::new(QUIET, PATIENCE);

        for chunk in 0..200 {
            settling.touch(
                "/films/Heat.mkv".into(),
                start + Duration::from_millis(chunk),
            );
            assert!(
                settling
                    .take_if_settled(start + Duration::from_millis(chunk))
                    .is_none()
            );
        }

        let settled = settling.take_if_settled(start + Duration::from_millis(600));
        assert_eq!(
            settled.as_deref(),
            Some(["/films/Heat.mkv".into()].as_slice())
        );

        // And once handed over, it is not handed over again.
        assert!(
            settling
                .take_if_settled(start + Duration::from_secs(10))
                .is_none()
        );
    }

    #[test]
    fn everything_that_changed_together_is_handed_over_together() {
        let start = Instant::now();
        let mut settling = Settling::new(QUIET, PATIENCE);
        settling.touch("/films/Ronin.mkv".into(), start);
        settling.touch("/films/Heat.mkv".into(), start + Duration::from_millis(50));
        settling.touch("/films/Heat.srt".into(), start + Duration::from_millis(90));

        let settled = settling
            .take_if_settled(start + Duration::from_millis(400))
            .unwrap_or_default();
        assert_eq!(settled.len(), 3);
    }

    #[test]
    fn something_that_never_stops_writing_does_not_hold_up_the_rest() {
        let start = Instant::now();
        let mut settling = Settling::new(QUIET, PATIENCE);
        settling.touch("/films/Heat.srt".into(), start);

        let mut at = start;
        while at < start + PATIENCE {
            at += Duration::from_millis(100);
            settling.touch("/films/Recording.mkv".into(), at);
        }

        // The folder has not been quiet for a moment, but the first change has
        // waited long enough.
        assert_eq!(settling.take_if_settled(at).unwrap_or_default().len(), 2);
    }
}
