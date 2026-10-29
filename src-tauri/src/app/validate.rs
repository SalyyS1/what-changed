//! Input validation for everything a command accepts.
//!
//! The frontend is untrusted, including for values Rust itself sent it a
//! moment earlier, so every path and every bounded number is re-checked here.
//! One module rather than a check per command, because a rule that lives in
//! eight places is a rule that is eventually applied in seven.
//!
//! The path rules exist for one attack: a watched folder that is really a
//! junction pointing at `C:\Windows`, another user's profile, or this app's own
//! data directory, turning a folder watcher into a system-wide reader. A
//! reparse point is therefore rejected outright rather than resolved and
//! re-checked — resolving it and comparing is a race, and the honest answer to
//! "watch this junction" is no.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// How deep below a watched root the file collector may descend.
pub const MAX_WATCH_DEPTH: u8 = 6;

/// How many folders may be watched at once. A watcher per root costs a handle
/// and a thread's attention; twenty is far past what a person can keep in
/// their head, and it bounds the cost.
pub const MAX_WATCHED_ROOTS: usize = 20;

/// Longest free-text field a command accepts. Anything longer is a paste, and
/// a paste is how a log file becomes a denial of service.
pub const MAX_TEXT: usize = 2_000;

/// Longest symptom description. Long enough for a paragraph, short enough that
/// the analysis engine's term extraction stays bounded.
pub const MAX_PROBLEM: usize = 500;

fn reject(what: &'static str, reason: impl Into<String>) -> AppError {
    AppError::InvalidInput {
        what,
        reason: reason.into(),
    }
}

/// Trim a string, reject it if it is empty or longer than `limit`.
///
/// Returns the trimmed value, because a name with trailing whitespace is a
/// name that will not match itself later.
pub fn text(what: &'static str, value: &str, limit: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(reject(what, "it is empty"));
    }
    // Count characters, not bytes: a limit in bytes rejects a shorter piece of
    // text for being in the wrong language.
    if trimmed.chars().count() > limit {
        return Err(reject(
            what,
            format!("it is longer than {limit} characters"),
        ));
    }
    Ok(trimmed.to_owned())
}

/// Reject a number outside `min..=max`.
pub fn in_range<T>(what: &'static str, value: T, min: T, max: T) -> AppResult<T>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    if value < min || value > max {
        return Err(reject(what, format!("it must be between {min} and {max}")));
    }
    Ok(value)
}

/// Reject a time range that is backwards or absurd.
pub fn time_range(from: i64, to: i64) -> AppResult<(i64, i64)> {
    if from > to {
        return Err(reject("time range", "it ends before it starts"));
    }
    Ok((from, to))
}

/// Strip Windows' verbatim prefix so a canonical path reads like a path.
///
/// `fs::canonicalize` returns `\\?\C:\Users\…` on Windows. That form is
/// correct and unusable: it is what the user would be shown, what a
/// `starts_with` comparison against an environment variable would fail on, and
/// what would be stored in the database forever.
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
}

/// Whether the entry at `path` is a reparse point: a symlink, a junction, or
/// any other redirection.
///
/// Checked with `symlink_metadata`, which does not follow the link, so the
/// answer is about this entry rather than about wherever it points.
#[cfg(windows)]
pub(crate) fn is_reparse_point(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    /// `FILE_ATTRIBUTE_REPARSE_POINT`. Covers junctions, which are not
    /// symlinks and which `FileType::is_symlink` is not required to report.
    const REPARSE: u32 = 0x0000_0400;
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_attributes() & REPARSE != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub(crate) fn is_reparse_point(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

/// Re-check a persisted root immediately before registering or traversing it.
/// Persisted validation is not proof against a later directory replacement.
pub(crate) fn watched_root_still_safe(path: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_dir() || is_reparse_point(path) {
        return false;
    }
    let Ok(canonical) = std::fs::canonicalize(path) else {
        return false;
    };
    display_path(&canonical).eq_ignore_ascii_case(&display_path(path))
}

/// Prove that no existing component below `root` redirects traversal.
/// A missing final component is permitted for remove events and sweep deletes;
/// missing or unreadable ancestors are not.
pub(crate) fn watched_path_still_safe(root: &Path, candidate: &Path) -> bool {
    watched_path_components_safe(root, candidate, false)
}

/// Variant for persisted entries known to have been removed while the watcher
/// was down. Once the first missing component is reached there is nothing
/// left that can redirect traversal; permission and I/O failures remain unsafe.
pub(crate) fn watched_removed_path_still_safe(root: &Path, candidate: &Path) -> bool {
    watched_path_components_safe(root, candidate, true)
}

fn watched_path_components_safe(root: &Path, candidate: &Path, allow_missing_suffix: bool) -> bool {
    if !watched_root_still_safe(root) || !candidate.starts_with(root) {
        return false;
    }
    let Ok(relative) = candidate.strip_prefix(root) else {
        return false;
    };
    let components = relative.components().collect::<Vec<_>>();
    let mut current = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(_) if is_reparse_point(&current) => return false,
            Ok(_) => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && (allow_missing_suffix || index + 1 == components.len()) =>
            {
                return true;
            }
            Err(_) => return false,
        }
    }
    true
}

/// Directories no watched folder may be inside, lowercased.
///
/// Built from the environment rather than from literals, because
/// `C:\Program Files` is a default, not a guarantee, and a machine that moved
/// it would otherwise have no protection at all.
fn forbidden_roots(app_data: &Path) -> Vec<String> {
    let mut roots = Vec::new();
    for key in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "ProgramData",
    ] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                roots.push(value.to_lowercase());
            }
        }
    }
    // The app's own folder: watching it would record the recorder, and every
    // write would produce an event that produces a write.
    roots.push(display_path(app_data).to_lowercase());

    // The profiles directory, but only when the other-user check below cannot
    // run. That check is the precise one — it allows *this* user's profile and
    // refuses every other — so it is preferred when it works. When it cannot
    // work, refusing the whole tree is the safe direction, and the cost is
    // that a user with no `USERPROFILE` cannot watch their own Documents.
    if users_dir().is_none() {
        if let Some(fallback) = fallback_users_dir() {
            tracing::warn!(
                "the user profile could not be determined; no folder under the \
                 profiles directory can be watched"
            );
            roots.push(fallback);
        }
    }
    roots
}

/// The directory that holds every user's profile, from this user's own.
///
/// `None` when the profile is unknown or is a drive root, which is what makes
/// the other-user comparison unusable.
fn users_dir() -> Option<String> {
    let profile = std::env::var("USERPROFILE").ok()?;
    if profile.is_empty() {
        return None;
    }
    let profile = profile.to_lowercase();
    let parent = Path::new(&profile).parent()?.to_string_lossy().to_string();
    (!parent.is_empty()).then_some(parent)
}

/// `%SystemDrive%\Users`, for when the profile itself is unavailable.
fn fallback_users_dir() -> Option<String> {
    let drive = std::env::var("SystemDrive").ok()?;
    (!drive.is_empty()).then(|| format!("{}\\users", drive.to_lowercase()))
}

/// Whether `candidate` is inside `root`, comparing whole path components so
/// `C:\Windows2` is not judged to be inside `C:\Windows`.
fn is_inside(candidate: &str, root: &str) -> bool {
    if candidate == root {
        return true;
    }
    let with_separator = if root.ends_with('\\') || root.ends_with('/') {
        root.to_owned()
    } else {
        format!("{root}\\")
    };
    candidate.starts_with(&with_separator)
}

/// Canonicalize a folder the user picked and prove it is safe to watch.
///
/// The order matters: existence and reparse-point checks run before
/// canonicalization, because canonicalizing follows the redirection this is
/// meant to catch.
pub fn watched_folder(raw: &str, app_data: &Path) -> AppResult<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(reject("folder", "no folder was given"));
    }
    let path = Path::new(trimmed);
    if !path.exists() {
        return Err(reject("folder", "it does not exist"));
    }
    if is_reparse_point(path) {
        return Err(reject(
            "folder",
            "it is a shortcut to somewhere else; choose the real folder instead",
        ));
    }
    if !path.is_dir() {
        return Err(reject("folder", "it is a file, not a folder"));
    }

    let canonical = std::fs::canonicalize(path).map_err(|_| {
        reject(
            "folder",
            "it could not be resolved; it may have been removed or be unreadable",
        )
    })?;
    let shown = display_path(&canonical);
    let lowered = shown.to_lowercase();

    // A drive root is not a folder anyone means to watch, and it is every
    // forbidden directory at once.
    if canonical.parent().is_none() {
        return Err(reject("folder", "a whole drive is too broad to watch"));
    }
    for root in forbidden_roots(app_data) {
        if is_inside(&lowered, &root) {
            return Err(reject(
                "folder",
                "system folders and the app's own data folder are never watched",
            ));
        }
    }
    if is_other_users_profile(&lowered) {
        return Err(reject("folder", "another user's folder is never watched"));
    }
    Ok(canonical)
}

/// Whether the path is inside a different user's profile.
///
/// Answers only when this user's own profile is known, because "another
/// user's" is defined relative to it. When it is not known this returns
/// `false` and `forbidden_roots` has already refused the whole profiles
/// directory instead — the two are written to cover each other, and the
/// warning there says which one is in force.
fn is_other_users_profile(lowered: &str) -> bool {
    let Some(users_dir) = users_dir() else {
        return false;
    };
    let Ok(profile) = std::env::var("USERPROFILE") else {
        return false;
    };
    is_inside(lowered, &users_dir) && !is_inside(lowered, &profile.to_lowercase())
}

/// Reject a watch depth outside the supported range.
pub fn watch_depth(depth: u8) -> AppResult<u8> {
    in_range("folder depth", depth, 1, MAX_WATCH_DEPTH)
}

/// Reject adding a root beyond the cap.
pub fn watch_capacity(current: usize) -> AppResult<()> {
    if current >= MAX_WATCHED_ROOTS {
        return Err(reject(
            "watched folders",
            format!("at most {MAX_WATCHED_ROOTS} folders can be watched; remove one first"),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed_and_bounded_in_characters() {
        assert_eq!(
            text("name", "  Before update  ", 50).unwrap(),
            "Before update"
        );
        assert!(text("name", "   ", 50).is_err());
        // Ten multi-byte characters are ten characters, not thirty bytes.
        assert!(text("name", &"é".repeat(10), 10).is_ok());
        assert!(text("name", &"é".repeat(11), 10).is_err());
    }

    #[test]
    fn ranges_and_depths_are_closed_intervals() {
        assert!(in_range("limit", 1u16, 1, 200).is_ok());
        assert!(in_range("limit", 200u16, 1, 200).is_ok());
        assert!(in_range("limit", 201u16, 1, 200).is_err());
        assert!(watch_depth(0).is_err());
        assert!(watch_depth(MAX_WATCH_DEPTH).is_ok());
        assert!(watch_depth(MAX_WATCH_DEPTH + 1).is_err());
        assert!(time_range(10, 5).is_err());
        assert_eq!(time_range(5, 10).unwrap(), (5, 10));
    }

    #[test]
    fn the_root_cap_is_reached_rather_than_exceeded() {
        assert!(watch_capacity(MAX_WATCHED_ROOTS - 1).is_ok());
        assert!(watch_capacity(MAX_WATCHED_ROOTS).is_err());
    }

    #[test]
    fn a_neighbouring_directory_is_not_inside_the_forbidden_one() {
        assert!(is_inside(r"c:\windows\system32", r"c:\windows"));
        assert!(is_inside(r"c:\windows", r"c:\windows"));
        assert!(
            !is_inside(r"c:\windows2\thing", r"c:\windows"),
            "a prefix match on the string is not containment on the path"
        );
    }

    #[test]
    fn the_verbatim_prefix_never_reaches_the_user() {
        assert_eq!(
            display_path(Path::new(r"\\?\C:\Users\dev\project")),
            r"C:\Users\dev\project"
        );
        assert_eq!(display_path(Path::new(r"C:\plain")), r"C:\plain");
    }

    #[test]
    fn a_folder_that_does_not_exist_is_refused_before_anything_else() {
        let data = crate::testing::temp_dir("validate-missing");
        let missing = data.join("no-such-folder");
        let error = watched_folder(&missing.to_string_lossy(), &data).unwrap_err();
        assert!(error.to_string().contains("does not exist"));
        assert!(watched_folder("", &data).is_err());
    }

    #[test]
    fn a_real_folder_outside_the_forbidden_roots_is_accepted() {
        let data = crate::testing::temp_dir("validate-ok");
        let watched = crate::testing::temp_dir("validate-watched");
        let resolved = watched_folder(&watched.to_string_lossy(), &data).unwrap();
        assert!(resolved.exists());
    }

    #[test]
    fn the_apps_own_data_folder_is_never_watchable() {
        let data = crate::testing::temp_dir("validate-selfwatch");
        let error = watched_folder(&data.to_string_lossy(), &data).unwrap_err();
        assert!(error.to_string().contains("never watched"));
    }

    #[test]
    fn the_windows_directory_is_refused() {
        let data = crate::testing::temp_dir("validate-windows");
        let Ok(system_root) = std::env::var("SystemRoot") else {
            return;
        };
        let error = watched_folder(&system_root, &data).unwrap_err();
        assert!(error.to_string().contains("system folders"));
    }

    #[test]
    fn a_file_is_not_a_folder() {
        let data = crate::testing::temp_dir("validate-file");
        let file = data.join("note.txt");
        std::fs::write(&file, "x").unwrap();
        let error = watched_folder(&file.to_string_lossy(), &data).unwrap_err();
        assert!(error.to_string().contains("not a folder"));
    }

    #[test]
    fn nested_redirections_are_rejected_at_collection_time() {
        let root = crate::testing::temp_dir("validate-nested-root");
        let target = crate::testing::temp_dir("validate-nested-target");
        let link = root.join("redirect");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&target, &link).is_ok();
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&target, &link).is_ok();
        if !linked {
            return;
        }
        assert!(!watched_path_still_safe(&root, &link.join("config.json")));
    }

    #[test]
    fn a_rejection_never_repeats_the_value_it_rejected() {
        let data = crate::testing::temp_dir("validate-quiet");
        let secret = data.join("AWS_SECRET_ACCESS_KEY-fixture");
        let error = watched_folder(&secret.to_string_lossy(), &data).unwrap_err();
        assert!(
            !error.to_string().contains("AWS_SECRET"),
            "a rejected path must not be echoed back: {error}"
        );
    }
}
