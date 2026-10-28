//! Sensitive raw captures: explicit request, review before disk, seven-day maximum.
use super::{invalid, PreviewCaptureDto};
use crate::error::AppResult;
use crate::platform::managed_file_lock as files;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
const PREFIX: &str = "data:image/png;base64,";
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const MAX_IMAGE: usize = 512 * 1024;
const RETENTION: i64 = 7 * 24 * 60 * 60 * 1000;
static ARTIFACT_OPERATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn decode(value: &str) -> AppResult<Vec<u8>> {
    let raw = value
        .strip_prefix(PREFIX)
        .ok_or_else(|| invalid("Unsupported capture format"))?;
    if raw.is_empty() || raw.len() > MAX_IMAGE * 4 / 3 + 4 || raw.len() % 4 != 0 {
        return Err(invalid("Capture size or encoding limit"));
    }
    let mut result = Vec::new();
    for (i, chunk) in raw.as_bytes().chunks_exact(4).enumerate() {
        let final_chunk = (i + 1) * 4 == raw.len();
        let mut digits = [0u8; 4];
        for j in 0..4 {
            digits[j] = if chunk[j] == b'=' && final_chunk && j >= 2 {
                0
            } else {
                ALPHABET
                    .iter()
                    .position(|c| *c == chunk[j])
                    .ok_or_else(|| invalid("Invalid capture encoding"))? as u8
            };
        }
        if chunk[2] == b'=' && (chunk[3] != b'=' || digits[1] & 15 != 0)
            || chunk[3] == b'=' && chunk[2] != b'=' && digits[2] & 3 != 0
        {
            return Err(invalid("Invalid capture padding"));
        }
        result.push((digits[0] << 2) | (digits[1] >> 4));
        if chunk[2] != b'=' {
            result.push((digits[1] << 4) | (digits[2] >> 2));
        }
        if chunk[3] != b'=' {
            result.push((digits[2] << 6) | digits[3]);
        }
    }
    if result.len() > MAX_IMAGE || !result.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(invalid("Capture is not a bounded PNG image"));
    }
    Ok(result)
}
fn encode(bytes: &[u8]) -> String {
    let mut output = String::from(PREFIX);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}
pub(super) fn pending(id: String, role: String, data_url: String) -> AppResult<PreviewCaptureDto> {
    // A one-minute sweep must still remove files before the seven-day boundary.
    decode(&data_url)?;
    Ok(PreviewCaptureDto {
        id,
        role,
        data_url,
        retained: false,
        expires_at: crate::domain::now_ms() + RETENTION - 120000,
    })
}
fn root(data: &Path, app: &str, run: &str) -> AppResult<PathBuf> {
    crate::owner_workspace::validation::uuid(app, "capture app")?;
    crate::owner_workspace::validation::uuid(run, "capture run")?;
    Ok(data.join("owner-artifacts").join(app).join(run))
}
pub(super) fn retain(
    data: &Path,
    app: &str,
    run: &str,
    capture: &PreviewCaptureDto,
) -> AppResult<()> {
    let _operation = ARTIFACT_OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    crate::owner_workspace::validation::uuid(&capture.id, "capture id")?;
    let bytes = decode(&capture.data_url)?;
    let root = root(data, app, run)?;
    crate::app::data_lifecycle::DataLifecycle::prepare_runner_artifacts(&root)?;
    let _pin = files::pin_directory(&root)
        .map_err(|_| invalid("Artifact directory could not be pinned"))?;
    let image = root.join(format!("{}.png", capture.id));
    let meta = root.join(format!("{}.json", capture.id));
    let write = || -> std::io::Result<()> {
        let mut file = files::create_new(&image)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let mut metadata = capture.clone();
        metadata.retained = true;
        metadata.data_url.clear();
        let encoded = serde_json::to_vec(&metadata).map_err(std::io::Error::other)?;
        let mut file = files::create_new(&meta)?;
        file.write_all(&encoded)?;
        file.sync_all()
    };
    write().map_err(|_| {
        invalid("The selected capture could not be retained; inspect app artifact cleanup status")
    })
}
pub fn list(data: &Path, app: &str, run: &str) -> AppResult<Vec<PreviewCaptureDto>> {
    let _operation = ARTIFACT_OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = root(data, app, run)?;
    if !root.exists() {
        return Ok(vec![]);
    }
    let _pin =
        files::pin_directory(&root).map_err(|_| invalid("Artifact directory is unavailable"))?;
    let mut output = vec![];
    for entry in std::fs::read_dir(&root).map_err(|_| invalid("Artifacts unavailable"))? {
        let path = entry
            .map_err(|_| invalid("Artifact entry unavailable"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let mut bytes = vec![];
        files::for_read(&path)
            .map_err(|_| invalid("Artifact metadata unavailable"))?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid("Artifact metadata unreadable"))?;
        if bytes.len() > 4096 {
            return Err(invalid("Artifact metadata exceeds its limit"));
        }
        let mut capture: PreviewCaptureDto =
            serde_json::from_slice(&bytes).map_err(|_| invalid("Invalid artifact metadata"))?;
        crate::owner_workspace::validation::uuid(&capture.id, "capture id")?;
        if path.file_stem().and_then(|v| v.to_str()) != Some(capture.id.as_str())
            || !capture.retained
            || !capture.data_url.is_empty()
            || capture.expires_at > crate::domain::now_ms() + RETENTION
        {
            return Err(invalid(
                "Artifact metadata does not match its reviewed capture",
            ));
        }
        if capture.expires_at <= crate::domain::now_ms() {
            remove_unlocked(data, app, run, &capture.id)?;
            continue;
        }
        let mut image = vec![];
        files::for_read(&root.join(format!("{}.png", capture.id)))
            .map_err(|_| invalid("Artifact image unavailable"))?
            .take(MAX_IMAGE as u64 + 1)
            .read_to_end(&mut image)
            .map_err(|_| invalid("Artifact image unreadable"))?;
        capture.data_url = encode(&image);
        decode(&capture.data_url)?;
        output.push(capture);
        if output.len() > 50 {
            return Err(invalid("Too many capture artifacts for one run"));
        }
    }
    Ok(output)
}
/// Run at startup and before each browser session, including orphaned raw files.
pub fn expire(data: &Path) -> AppResult<()> {
    let _operation = ARTIFACT_OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let base = data.join("owner-artifacts");
    if !base.exists() {
        return Ok(());
    }
    let _pin =
        files::pin_directory(&base).map_err(|_| invalid("Artifact expiry root unavailable"))?;
    for app in std::fs::read_dir(&base).map_err(|_| invalid("Artifact expiry unavailable"))? {
        let app = app.map_err(|_| invalid("Artifact expiry unavailable"))?;
        let app_id = app.file_name().to_string_lossy().into_owned();
        crate::owner_workspace::validation::uuid(&app_id, "artifact app")?;
        let _app = files::pin_directory(&app.path())
            .map_err(|_| invalid("Artifact app directory unavailable"))?;
        for run in
            std::fs::read_dir(app.path()).map_err(|_| invalid("Artifact runs unavailable"))?
        {
            let run = run.map_err(|_| invalid("Artifact run unavailable"))?;
            let run_id = run.file_name().to_string_lossy().into_owned();
            crate::owner_workspace::validation::uuid(&run_id, "artifact run")?;
            let _run = files::pin_directory(&run.path())
                .map_err(|_| invalid("Artifact run directory unavailable"))?;
            for entry in std::fs::read_dir(run.path())
                .map_err(|_| invalid("Artifact capture unavailable"))?
            {
                let path = entry
                    .map_err(|_| invalid("Artifact capture unavailable"))?
                    .path();
                let id = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| invalid("Artifact name unavailable"))?;
                crate::owner_workspace::validation::uuid(id, "artifact capture")?;
                if path.extension().and_then(|value| value.to_str()) == Some("png")
                    && !path.with_extension("json").exists()
                {
                    remove_unlocked(data, &app_id, &run_id, id)?;
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let mut bytes = vec![];
                files::for_read(&path)
                    .map_err(|_| invalid("Artifact expiry metadata unavailable"))?
                    .take(4097)
                    .read_to_end(&mut bytes)
                    .map_err(|_| invalid("Artifact expiry metadata unreadable"))?;
                let metadata = serde_json::from_slice::<PreviewCaptureDto>(&bytes);
                if metadata.is_err()
                    || metadata.as_ref().is_ok_and(|value| {
                        value.expires_at <= crate::domain::now_ms()
                            || value.expires_at > crate::domain::now_ms() + RETENTION
                    })
                {
                    remove_unlocked(data, &app_id, &run_id, id)?;
                }
            }
        }
    }
    Ok(())
}
pub(super) fn remove(data: &Path, app: &str, run: &str, id: &str) -> AppResult<()> {
    let _operation = ARTIFACT_OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    remove_unlocked(data, app, run, id)
}
fn remove_unlocked(data: &Path, app: &str, run: &str, id: &str) -> AppResult<()> {
    crate::owner_workspace::validation::uuid(id, "capture id")?;
    let root = root(data, app, run)?;
    if !root.exists() {
        return Ok(());
    }
    let _pin =
        files::pin_directory(&root).map_err(|_| invalid("Artifact directory is unavailable"))?;
    for extension in ["png", "json"] {
        let path = root.join(format!("{id}.{extension}"));
        match files::for_read(&path) {
            Ok(file) => {
                drop(file);
                std::fs::remove_file(&path).map_err(|_| invalid("Capture could not be deleted"))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(invalid("Capture could not be safely inspected")),
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn image_encoding_is_bounded_and_strict() {
        for suffix in [b"".as_slice(), b"a", b"ab", b"abc"] {
            let image = [b"\x89PNG\r\n\x1a\n".as_slice(), suffix].concat();
            assert_eq!(decode(&encode(&image)).unwrap(), image)
        }
        for invalid in [
            "data:text/html;base64,AAAA",
            "data:image/png;base64,=====",
            "data:image/png;base64,AB==",
        ] {
            assert!(decode(invalid).is_err())
        }
    }
    #[test]
    #[cfg(windows)]
    fn retained_capture_survives_reload_and_expired_or_rejected_capture_is_removed() {
        let data = crate::testing::temp_dir("preview-capture-retention");
        let app = uuid::Uuid::new_v4().to_string();
        let run = uuid::Uuid::new_v4().to_string();
        let capture = pending(
            uuid::Uuid::new_v4().to_string(),
            "staff".into(),
            encode(b"\x89PNG\r\n\x1a\nsynthetic"),
        )
        .unwrap();
        assert!(list(&data, &app, &run).unwrap().is_empty());
        retain(&data, &app, &run, &capture).unwrap();
        let reloaded = list(&data, &app, &run).unwrap();
        assert_eq!(reloaded.len(), 1);
        assert!(reloaded[0].retained);
        assert_eq!(reloaded[0].data_url, capture.data_url);
        let mut expired = capture.clone();
        expired.id = uuid::Uuid::new_v4().to_string();
        expired.expires_at = 1;
        retain(&data, &app, &run, &expired).unwrap();
        expire(&data).unwrap();
        assert!(!root(&data, &app, &run)
            .unwrap()
            .join(format!("{}.png", expired.id))
            .exists());
        remove(&data, &app, &run, &capture.id).unwrap();
        assert!(list(&data, &app, &run).unwrap().is_empty());
    }
}
