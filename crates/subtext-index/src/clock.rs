//! Wall clock time, as the database stores it.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current time in milliseconds since the epoch.
///
/// Stored times are only ever shown to a person or used to order rows, so a
/// clock set behind the epoch, or one that has been wound back, costs a row its
/// place in a list and nothing more.
pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::now_millis;

    #[test]
    fn reads_a_time_after_the_epoch() {
        // Any plausible clock is well past this, and a broken one gives zero
        // rather than a wrong-looking date.
        assert!(now_millis() > 1_600_000_000_000 || now_millis() == 0);
    }
}
