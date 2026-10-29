//! A fixed-window rate limiter for the one command an untrusted page may call
//! freely.
//!
//! `log_ui_error` exists so the frontend can report its own failures. That
//! makes it the one command whose whole purpose is to write attacker-supplied
//! text into a log file, so it is capped. A fixed window rather than a token
//! bucket: the cap is about bounding a log file, not about smoothing traffic,
//! and one integer plus one timestamp is the whole implementation.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Calls allowed per window.
pub const LIMIT: u32 = 10;

/// The window.
pub const WINDOW: Duration = Duration::from_secs(60);

/// A counter that resets every window.
#[derive(Debug)]
pub struct RateLimiter {
    state: Mutex<Window>,
    limit: u32,
    window: Duration,
}

#[derive(Debug)]
struct Window {
    started: Instant,
    used: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(LIMIT, WINDOW)
    }
}

impl RateLimiter {
    /// A limiter allowing `limit` calls per `window`.
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            state: Mutex::new(Window {
                started: Instant::now(),
                used: 0,
            }),
            limit,
            window,
        }
    }

    /// Take one slot. `false` means the caller is over the cap.
    pub fn allow(&self) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.started.elapsed() >= self.window {
            state.started = Instant::now();
            state.used = 0;
        }
        if state.used >= self.limit {
            return false;
        }
        state.used += 1;
        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_first_ten_calls_pass_and_the_eleventh_does_not() {
        let limiter = RateLimiter::default();
        for i in 0..LIMIT {
            assert!(limiter.allow(), "call {i} should be allowed");
        }
        assert!(!limiter.allow());
        assert!(!limiter.allow(), "still refused inside the same window");
    }

    #[test]
    fn the_window_resets_and_the_cap_applies_again() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(!limiter.allow());
        std::thread::sleep(Duration::from_millis(70));
        assert!(limiter.allow(), "a new window starts fresh");
        assert!(limiter.allow());
        assert!(!limiter.allow(), "and is capped the same way");
    }
}
