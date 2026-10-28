//! Verify packaged files through immutable open handles before launching Node.
use super::invalid;
use crate::error::AppResult;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    schema_version: u32,
    node_version: String,
    playwright_version: String,
    chromium_revision: String,
    browser_executable: String,
    checksum_source: String,
    installed_bytes: f64,
    files: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
}
pub struct Bundle {
    pub root: PathBuf,
    pub digest: String,
    _held: Vec<File>,
}

impl Bundle {
    pub fn open(resource_root: &Path) -> AppResult<Self> {
        let root = resource_root.join("preview-runner");
        Self::open_root(root)
    }
    fn open_root(root: PathBuf) -> AppResult<Self> {
        #[cfg(windows)]
        {
            let system_root = std::env::var_os("SystemRoot")
                .ok_or_else(|| invalid("The Windows system compiler is unavailable"))?;
            if !PathBuf::from(system_root)
                .join("System32/D3DCompiler_47.dll")
                .is_file()
            {
                return Err(invalid(
                    "Preview requires Windows 10/11 with its system D3DCompiler_47.dll component",
                ));
            }
        }
        let _directory=crate::platform::managed_file_lock::pin_directory(&root).map_err(|_|invalid("Packaged runner is unavailable. Prepare its bundle before building this candidate."))?;
        let mut manifest =
            crate::platform::managed_file_lock::for_read(&root.join("bundle-inventory.json"))
                .map_err(|_| invalid("Runner inventory is missing"))?;
        if manifest
            .metadata()
            .map_err(|_| invalid("Runner inventory unavailable"))?
            .len()
            > 2 * 1024 * 1024
        {
            return Err(invalid("Runner inventory is too large"));
        }
        let mut bytes = Vec::new();
        manifest
            .read_to_end(&mut bytes)
            .map_err(|_| invalid("Runner inventory unavailable"))?;
        let inventory: Inventory =
            serde_json::from_slice(&bytes).map_err(|_| invalid("Runner inventory is invalid"))?;
        if inventory.schema_version != 1
            || inventory.checksum_source != "https://nodejs.org/dist/v24.12.0/SHASUMS256.txt"
            || inventory.node_version != "v24.12.0"
            || inventory.playwright_version != "1.62.1"
            || inventory.chromium_revision != "156.0.8078.47-1.1"
            || inventory.browser_executable != "browsers/chromium-156.0.8078.47-1.1/chrome.exe"
            || inventory.files.len() > 10000
        {
            return Err(invalid(
                "Runner runtime versions do not match this application",
            ));
        }
        let mut held = vec![manifest, _directory];
        let mut seen = std::collections::BTreeSet::new();
        let mut directories = std::collections::BTreeSet::new();
        let mut total = 0u64;
        for entry in inventory.files {
            let path = Path::new(&entry.path);
            if entry.path.is_empty()
                || entry.path.contains(['\\', ':'])
                || entry
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || part.ends_with(['.', ' ']))
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                || !seen.insert(entry.path.clone())
            {
                return Err(invalid(
                    "Runner inventory contains an unsafe or duplicate path",
                ));
            }
            // Pin every ancestor against substitution while its files are in use.
            if let Some(parent) = path.parent() {
                let mut current = root.clone();
                for part in parent.components() {
                    current.push(part);
                    if directories.insert(current.clone()) {
                        held.push(
                            crate::platform::managed_file_lock::pin_directory(&current)
                                .map_err(|_| invalid("Runner path is redirected or unavailable"))?,
                        );
                    }
                }
            }
            let mut file = crate::platform::managed_file_lock::for_read(&root.join(path))
                .map_err(|_| invalid("A packaged runner file is unavailable"))?;
            let length = file
                .metadata()
                .map_err(|_| invalid("Runner file metadata unavailable"))?
                .len();
            total = total.saturating_add(length);
            if length != entry.bytes || total > 1536 * 1024 * 1024 {
                return Err(invalid("Runner size inventory mismatch"));
            }
            let mut digest = Sha256::new();
            let mut buffer = [0u8; 65536];
            loop {
                let length = file
                    .read(&mut buffer)
                    .map_err(|_| invalid("Runner file could not be verified"))?;
                if length == 0 {
                    break;
                }
                digest.update(&buffer[..length]);
            }
            if format!("{:x}", digest.finalize()) != entry.sha256 {
                return Err(invalid(
                    "A packaged runner file changed. Reinstall the reviewed candidate.",
                ));
            }
            held.push(file);
        }
        if total as f64 != inventory.installed_bytes {
            return Err(invalid("Runner installed-size inventory mismatch"));
        }
        for required in [
            "node.exe",
            "src/main.mjs",
            "src/protocol.mjs",
            "src/policy.mjs",
            "src/egress-proxy.mjs",
            "src/recipe-validation.mjs",
            "src/execute-recipe.mjs",
            "src/browser-session.mjs",
            "src/record-locator.mjs",
            "src/revision-marker.mjs",
            "src/session-authority.mjs",
            "playwright-core/index.mjs",
            "playwright-core/package.json",
            "browsers/chromium-156.0.8078.47-1.1/chrome.exe",
        ] {
            if !seen.contains(required) {
                return Err(invalid("Runner inventory is incomplete"));
            }
        }
        verify_tree(&root, &root, &seen)?;
        Ok(Self {
            root,
            digest: format!("{:x}", Sha256::digest(&bytes)),
            _held: held,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires explicit OWNVERO_TEST_BUNDLE"]
    fn packaged_inventory_matches_native_loader() {
        let root = std::env::var_os("OWNVERO_TEST_BUNDLE").expect(
            "set OWNVERO_TEST_BUNDLE to the packaged test bundle before invoking this opt-in test",
        );
        let bundle = Bundle::open_root(PathBuf::from(root)).expect(
            "the explicitly selected packaged test bundle must pass native loader validation",
        );
        assert_eq!(bundle.digest.len(), 64);
    }
}

fn verify_tree(
    root: &Path,
    current: &Path,
    seen: &std::collections::BTreeSet<String>,
) -> AppResult<()> {
    for entry in std::fs::read_dir(current).map_err(|_| invalid("Runner directory unavailable"))? {
        let path = entry
            .map_err(|_| invalid("Runner directory entry unavailable"))?
            .path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| invalid("Runner metadata unavailable"))?;
        if metadata.file_type().is_symlink() {
            return Err(invalid("Runner resources contain redirected paths"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(invalid("Runner resources contain redirected paths"));
            }
        }
        if metadata.is_dir() {
            verify_tree(root, &path, seen)?;
        } else {
            let name = path
                .strip_prefix(root)
                .map_err(|_| invalid("Runner resource escaped its root"))?
                .to_string_lossy()
                .replace('\\', "/");
            if name != "bundle-inventory.json" && !seen.contains(&name) {
                return Err(invalid("Runner resources contain an unreviewed file"));
            }
        }
    }
    Ok(())
}
