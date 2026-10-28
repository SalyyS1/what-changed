//! App-scoped status, login release, cancellation and explicit capture review.
use super::{coordinator::PreviewRunner, invalid, PreviewRunnerStatusDto};
use crate::error::AppResult;
use std::path::Path;
use std::sync::atomic::Ordering;

impl PreviewRunner {
    pub fn status(&self, app_id: &str, run_id: &str) -> AppResult<PreviewRunnerStatusDto> {
        let active = self.active.lock().unwrap_or_else(|p| p.into_inner());
        let control = active
            .as_ref()
            .ok_or_else(|| invalid("No preview session is active"))?;
        let mut status = control.status.lock().unwrap_or_else(|p| p.into_inner());
        if status.app_id != app_id || status.run_id != run_id {
            return Err(invalid("Preview session belongs to another app or run"));
        }
        status
            .captures
            .retain(|capture| capture.expires_at > crate::domain::now_ms());
        Ok(status.clone())
    }
    pub fn continue_login(&self, app_id: &str, run_id: &str) -> AppResult<()> {
        let active = self.active.lock().unwrap_or_else(|p| p.into_inner());
        let control = active
            .as_ref()
            .ok_or_else(|| invalid("No preview session"))?;
        let mut status = control.status.lock().unwrap_or_else(|p| p.into_inner());
        if status.app_id != app_id || status.run_id != run_id || !status.can_continue {
            return Err(invalid("This session is not waiting for test login"));
        }
        status.can_continue = false;
        status.lifecycle = "running".into();
        Ok(())
    }
    pub fn cancel(&self, app_id: &str, run_id: &str) -> AppResult<()> {
        let active = self.active.lock().unwrap_or_else(|p| p.into_inner());
        let control = active
            .as_ref()
            .ok_or_else(|| invalid("No preview session is active"))?;
        {
            let status = control.status.lock().unwrap_or_else(|p| p.into_inner());
            if status.app_id != app_id || status.run_id != run_id {
                return Err(invalid("Preview session belongs to another run"));
            }
        }
        control.stop();
        Ok(())
    }
    pub fn capture(&self, app_id: &str, run_id: &str, role: &str) -> AppResult<()> {
        let active = self.active.lock().unwrap_or_else(|p| p.into_inner());
        let control = active
            .as_ref()
            .ok_or_else(|| invalid("No preview session is active"))?;
        let status = control.status.lock().unwrap_or_else(|p| p.into_inner());
        if status.app_id != app_id
            || status.run_id != run_id
            || !status.capture_screenshots
            || !status.roles.iter().any(|expected| expected == role)
            || !["awaiting_login", "recording", "running"].contains(&status.lifecycle.as_str())
            || status.captures.len() >= 50
            || control.finished.load(Ordering::Acquire)
        {
            return Err(invalid(
                "Capturing this test surface was not authorized or the capture limit was reached",
            ));
        }
        if control.capture_pending.swap(true, Ordering::AcqRel) {
            return Err(invalid(
                "Wait for the requested capture before asking for another",
            ));
        }
        *control
            .capture_request
            .lock()
            .unwrap_or_else(|p| p.into_inner()) =
            Some((uuid::Uuid::new_v4().to_string(), role.into()));
        Ok(())
    }
    pub fn review_capture(
        &self,
        data_root: &Path,
        app_id: &str,
        run_id: &str,
        capture_id: &str,
        retain: bool,
    ) -> AppResult<()> {
        let active = self.active.lock().unwrap_or_else(|p| p.into_inner());
        let Some(control) = active.as_ref() else {
            return if retain {
                Err(invalid("Raw capture is no longer available for review"))
            } else {
                super::captures::remove(data_root, app_id, run_id, capture_id)
            };
        };
        let mut status = control.status.lock().unwrap_or_else(|p| p.into_inner());
        if status.app_id != app_id || status.run_id != run_id {
            return if retain {
                Err(invalid("Raw capture belongs to another session"))
            } else {
                super::captures::remove(data_root, app_id, run_id, capture_id)
            };
        }
        let Some(index) = status
            .captures
            .iter()
            .position(|capture| capture.id == capture_id)
        else {
            return if retain {
                Err(invalid("Raw capture is no longer available for review"))
            } else {
                super::captures::remove(data_root, app_id, run_id, capture_id)
            };
        };
        if status.captures[index].retained && retain {
            return Ok(());
        }
        if retain {
            if status.captures[index].expires_at <= crate::domain::now_ms() {
                return Err(invalid("Capture expired before review"));
            }
            super::captures::retain(data_root, app_id, run_id, &status.captures[index])?;
            status.captures[index].retained = true;
        } else {
            if status.captures[index].retained {
                super::captures::remove(data_root, app_id, run_id, capture_id)?;
            }
            status.captures.remove(index);
        }
        Ok(())
    }
}
