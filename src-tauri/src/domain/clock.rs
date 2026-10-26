//! The one clock. Every timestamp in the product is epoch milliseconds UTC,
//! and the UI localizes at the display edge — so there is exactly one place
//! that reads the system clock and one unit it reads it in.

use std::time::{SystemTime, UNIX_EPOCH};

/// Now, as epoch milliseconds UTC.
///
/// A clock set before 1970 yields zero rather than an error: a timestamp is
/// never worth failing an event over, and the value is impossible enough to
/// stand out in a timeline.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_after_the_day_this_was_written() {
        // 2026-01-01T00:00:00Z in milliseconds.
        assert!(now_ms() > 1_767_225_600_000);
    }
}
