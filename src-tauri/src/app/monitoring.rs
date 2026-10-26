//! Whether the recorder is recording.
//!
//! One shared flag, read by the tray, the titlebar pill and every collector
//! once the runtime lands. A pause has an optional deadline so "pause for an
//! hour" resumes by itself: a recorder that stays off because someone forgot
//! to turn it back on is worse than one that was never paused, since the user
//! believes it is watching.
//!
//! The state lives in an atomic rather than behind a lock because collectors
//! check it at the top of every cycle, and a cycle must never wait on the UI.
//!
//! **A timed pause is read as expired immediately and finalized by the
//! collector watchdog.** The split keeps this value platform-neutral while
//! still giving the runtime one place to wake collectors, update the tray and
//! emit the frontend lifecycle event at the deadline.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::domain::now_ms;
use crate::dto::MonitoringDto;

const RUNNING: u8 = 0;
const PAUSED: u8 = 1;
const TIMED_PAUSE: u8 = 2;
const MAX_PAUSE_MINUTES: u32 = 7 * 24 * 60;

#[derive(Debug, Clone, Copy)]
struct Deadline {
    monotonic: Instant,
    epoch_ms: i64,
}

/// The pause gate.
#[derive(Debug)]
pub struct Monitoring {
    state: AtomicU8,
    deadline: Mutex<Option<Deadline>>,
}

impl Default for Monitoring {
    fn default() -> Self {
        Self::running()
    }
}

impl Monitoring {
    /// A gate that starts running.
    pub fn running() -> Self {
        Self {
            state: AtomicU8::new(RUNNING),
            deadline: Mutex::new(None),
        }
    }

    /// Whether collection should happen right now.
    ///
    /// An elapsed deadline reads as running even if the watchdog has not yet
    /// finalized and announced the transition.
    pub fn is_paused(&self) -> bool {
        self.read().0
    }

    /// The pause state and its deadline, as one consistent pair.
    ///
    /// Both callers need the two together, and reading them separately admits
    /// an interleaving where a concurrent `pause(Some(n))` lands between the
    /// loads and the caller reports "running" while discarding the deadline
    /// just set. `Acquire`/`Release` on `paused` orders the pair: `pause`
    /// writes the deadline first and publishes it by storing `paused`, so a
    /// reader that observes `paused` observes the deadline that came with it.
    fn read(&self) -> (bool, Option<i64>) {
        match self.state.load(Ordering::Acquire) {
            RUNNING => (false, None),
            PAUSED => (true, None),
            TIMED_PAUSE => {
                let deadline = *lock(&self.deadline);
                let Some(deadline) = deadline else {
                    return (false, None);
                };
                if Instant::now() >= deadline.monotonic {
                    (false, None)
                } else {
                    (true, Some(deadline.epoch_ms))
                }
            }
            _ => (false, None),
        }
    }

    /// Pause, optionally until `minutes` have passed.
    pub fn pause(&self, minutes: Option<u32>) {
        if let Some(minutes) = minutes {
            let minutes = minutes.min(MAX_PAUSE_MINUTES);
            self.pause_for(Duration::from_secs(u64::from(minutes).saturating_mul(60)));
        } else {
            *lock(&self.deadline) = None;
            self.state.store(PAUSED, Ordering::Release);
        }
    }

    /// Pause for an exact duration.
    ///
    /// Commands use whole minutes, while the runtime's deterministic tests
    /// use short durations to exercise deadline delivery without sleeping for
    /// a minute. Keeping deadline construction here preserves one clock and
    /// one state transition implementation.
    pub fn pause_for(&self, duration: Duration) {
        *lock(&self.deadline) = Some(Deadline {
            monotonic: Instant::now() + duration,
            epoch_ms: now_ms()
                .saturating_add(i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)),
        });
        self.state.store(TIMED_PAUSE, Ordering::Release);
    }

    /// Time until an automatic resume is due, if this is a timed pause.
    pub fn resume_in(&self) -> Option<Duration> {
        if self.state.load(Ordering::Acquire) != TIMED_PAUSE {
            return None;
        }
        lock(&self.deadline)
            .map(|deadline| deadline.monotonic.saturating_duration_since(Instant::now()))
    }

    /// Atomically expire the current timed pause if its deadline passed.
    ///
    /// Returns whether a real state transition happened, allowing the
    /// supervisor to publish exactly one lifecycle notification.
    pub fn expire_if_due(&self) -> bool {
        let mut deadline = lock(&self.deadline);
        if self.state.load(Ordering::Acquire) != TIMED_PAUSE
            || deadline.is_some_and(|value| Instant::now() < value.monotonic)
        {
            return false;
        }
        *deadline = None;
        self.state.store(RUNNING, Ordering::Release);
        true
    }

    /// Resume immediately.
    pub fn resume(&self) {
        self.state.store(RUNNING, Ordering::Release);
        *lock(&self.deadline) = None;
    }

    /// Apply a boolean from the UI or the tray.
    pub fn set_paused(&self, paused: bool, minutes: Option<u32>) {
        if paused {
            self.pause(minutes);
        } else {
            self.resume();
        }
    }

    /// The state as the frontend sees it.
    ///
    /// This reflects an elapsed deadline as running; the supervisor watchdog
    /// owns mutation and lifecycle notification. See `read`.
    pub fn snapshot(&self) -> MonitoringDto {
        let (paused, resume_at) = self.read();
        MonitoringDto { paused, resume_at }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_gate_is_running_and_says_so() {
        let gate = Monitoring::running();
        assert!(!gate.is_paused());
        let snapshot = gate.snapshot();
        assert!(!snapshot.paused);
        assert!(snapshot.resume_at.is_none());
    }

    #[test]
    fn an_indefinite_pause_stays_paused_and_offers_no_resume_time() {
        let gate = Monitoring::running();
        gate.pause(None);
        assert!(gate.is_paused());
        assert_eq!(gate.snapshot().resume_at, None);
        gate.resume();
        assert!(!gate.is_paused());
    }

    #[test]
    fn a_timed_pause_carries_its_deadline_and_expires_on_its_own() {
        let gate = Monitoring::running();
        gate.pause(Some(60));
        let snapshot = gate.snapshot();
        assert!(snapshot.paused);
        let resume_at = snapshot.resume_at.expect("a timed pause has a deadline");
        assert!(resume_at > now_ms(), "the deadline is in the future");

        // A deadline already in the past resumes on the next read, with no
        // timer thread involved.
        *lock(&gate.deadline) = Some(Deadline {
            monotonic: Instant::now() - Duration::from_millis(1),
            epoch_ms: now_ms() - 1,
        });
        gate.state.store(TIMED_PAUSE, Ordering::Release);
        assert!(!gate.is_paused());
        assert!(!gate.snapshot().paused);
        assert!(gate.expire_if_due());
        assert!(gate.resume_in().is_none());
    }

    #[test]
    fn the_pause_state_and_its_deadline_are_read_together() {
        // Reported separately they can disagree: a pause landing between two
        // loads yields "running" with the deadline discarded.
        let gate = Monitoring::running();
        gate.pause(Some(30));
        let (paused, resume_at) = gate.read();
        assert!(paused);
        assert!(resume_at.is_some());

        let snapshot = gate.snapshot();
        assert_eq!(snapshot.paused, paused);
        assert_eq!(snapshot.resume_at, resume_at);
    }

    #[test]
    fn set_paused_is_the_toggle_the_tray_and_the_ui_share() {
        let gate = Monitoring::running();
        gate.set_paused(true, None);
        assert!(gate.is_paused());
        gate.set_paused(false, None);
        assert!(!gate.is_paused());
    }
}
