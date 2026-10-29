//! Sleep/resume or large clock changes end permission before the next action.
use super::invalid;
use crate::error::AppResult;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct ClockGuard {
    wall: u64,
    awake: u64,
}
impl ClockGuard {
    pub fn new() -> AppResult<Self> {
        sample()
    }
    pub fn validate(&mut self) -> AppResult<()> {
        let next = sample()?;
        let wall = next.wall.saturating_sub(self.wall);
        let awake = next.awake.saturating_sub(self.awake);
        if next.wall < self.wall || wall.saturating_sub(awake) > 2000 {
            return Err(invalid(
                "The computer slept or its clock changed; prepare fresh permission before retrying",
            ));
        }
        *self = next;
        Ok(())
    }
}
fn sample() -> AppResult<ClockGuard> {
    let wall = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| invalid("Run clock unavailable"))?
            .as_millis(),
    )
    .map_err(|_| invalid("Run clock unavailable"))?;
    let mut awake = 0;
    if !unsafe {
        windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime(&mut awake)
    }
    .as_bool()
    {
        return Err(invalid("Windows monotonic run clock unavailable"));
    }
    Ok(ClockGuard {
        wall,
        awake: awake / 10000,
    })
}
