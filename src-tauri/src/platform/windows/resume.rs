//! Resume-from-sleep detection without window-message coupling.
//!
//! Wall time advances while a machine sleeps; unbiased interrupt time does not. A
//! gap between the two therefore provides the plan's fallback wake without
//! giving a platform callback access to collectors or storage.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::collectors::runtime::ResumeWakeHandle;
use crate::error::{AppError, AppResult};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(30);
const RESUME_GAP_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ClockSample {
    wall_ms: u64,
    awake_ms: u64,
}

pub struct WindowsResumeMonitor {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl WindowsResumeMonitor {
    pub fn start(wake: ResumeWakeHandle) -> AppResult<Self> {
        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("windows-resume-monitor".to_owned())
            .stack_size(128 * 1024)
            .spawn(move || run(worker_stop, wake))
            .map_err(|source| AppError::File {
                operation: "start resume monitor",
                path: std::path::PathBuf::from("power-resume"),
                source,
            })?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for WindowsResumeMonitor {
    fn drop(&mut self) {
        let (lock, changed) = &*self.stop;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        changed.notify_all();
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                tracing::debug!("the resume monitor panicked while stopping");
            }
        }
    }
}

fn run(stop: Arc<(Mutex<bool>, Condvar)>, wake: ResumeWakeHandle) {
    let Some(mut previous) = sample() else {
        tracing::debug!("resume monitoring unavailable because wall time could not be sampled");
        return;
    };
    loop {
        let (lock, changed) = &*stop;
        let stopped = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (stopped, _) = changed
            .wait_timeout_while(stopped, SAMPLE_INTERVAL, |stopped| !*stopped)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *stopped {
            return;
        }
        drop(stopped);

        let Some(current) = sample() else {
            continue;
        };
        if resume_gap_detected(previous, current) {
            wake.request();
        }
        previous = current;
    }
}

fn sample() -> Option<ClockSample> {
    let wall_ms = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    let mut awake_100ns = 0_u64;
    // SAFETY: the pointer names one initialized `u64` for the duration of the
    // call. Windows writes unbiased interrupt time in 100 ns units.
    let sampled = unsafe {
        windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime(&mut awake_100ns)
    };
    if !sampled.as_bool() {
        return None;
    }
    let awake_ms = awake_100ns / 10_000;
    Some(ClockSample { wall_ms, awake_ms })
}

fn resume_gap_detected(previous: ClockSample, current: ClockSample) -> bool {
    let wall_delta = current.wall_ms.saturating_sub(previous.wall_ms);
    let awake_delta = current.awake_ms.saturating_sub(previous.awake_ms);
    wall_delta.saturating_sub(awake_delta) > RESUME_GAP_MS
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_sleep_gap_is_distinct_from_ordinary_elapsed_time_and_clock_noise() {
        let start = ClockSample {
            wall_ms: 1_000_000,
            awake_ms: 500_000,
        };
        assert!(!resume_gap_detected(
            start,
            ClockSample {
                wall_ms: 1_030_000,
                awake_ms: 530_000,
            }
        ));
        assert!(!resume_gap_detected(
            start,
            ClockSample {
                wall_ms: 1_090_000,
                awake_ms: 530_000,
            }
        ));
        assert!(resume_gap_detected(
            start,
            ClockSample {
                wall_ms: 1_091_001,
                awake_ms: 530_000,
            }
        ));
    }
}
