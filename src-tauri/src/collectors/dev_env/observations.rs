use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use super::{DevEnvironmentState, ToolState, ID};
use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

const MAX_GIT_CONTROL_BYTES: u64 = 4 * 1024;

pub(super) fn normalize_environment(input: BTreeMap<String, String>) -> BTreeMap<String, String> {
    input
        .into_iter()
        .map(|(key, value)| (key.to_ascii_uppercase(), value))
        .collect()
}

pub(super) fn between(
    before: &DevEnvironmentState,
    after: &DevEnvironmentState,
    occurred_at: i64,
    occurred_at_is_uncertain: bool,
) -> Vec<RawObservation> {
    let mut events = environment_observations(before, after, occurred_at);
    events.extend(path_observations(before, after, occurred_at));
    events.extend(tool_observations(before, after, occurred_at));
    events.extend(branch_observations(before, after, occurred_at));
    for event in &mut events {
        event.occurred_at_is_uncertain = occurred_at_is_uncertain;
    }
    events
}

fn environment_observations(
    before: &DevEnvironmentState,
    after: &DevEnvironmentState,
    occurred_at: i64,
) -> Vec<RawObservation> {
    let names: BTreeSet<&String> = before
        .environment
        .keys()
        .chain(after.environment.keys())
        .filter(|name| !name.eq_ignore_ascii_case("PATH"))
        .collect();
    names
        .into_iter()
        .filter_map(|name| {
            let old = before.environment.get(name);
            let new = after.environment.get(name);
            if old == new || (!before.environment_values_known && old.is_some() && new.is_some()) {
                // The durable baseline contains no value or value-derived
                // marker. On restart, presence is provable but equality is
                // unknown; do not invent a value-change event from redaction.
                return None;
            }
            let (event_type, verb) = match (old, new) {
                (None, Some(_)) => (EventType::Created, "was added"),
                (Some(_), None) => (EventType::Deleted, "was removed"),
                (Some(_), Some(_)) => (EventType::ConfigurationChanged, "changed"),
                (None, None) => return None,
            };
            Some(system_observation(SystemObservation {
                category: Category::Configuration,
                event_type,
                user_summary: format!("Environment variable {name} {verb}"),
                technical_summary: format!("environment variable {name} {verb}"),
                before: Some(named_value(name, old)),
                after: Some(named_value(name, new)),
                field: Some(name.clone()),
                occurred_at,
            }))
        })
        .collect()
}

fn named_value(name: &str, value: Option<&String>) -> Value {
    let mut object = Map::new();
    object.insert(
        name.to_owned(),
        value.map_or(Value::Null, |_| {
            Value::String(crate::privacy::REDACTED.to_owned())
        }),
    );
    Value::Object(object)
}

fn path_observations(
    before: &DevEnvironmentState,
    after: &DevEnvironmentState,
    occurred_at: i64,
) -> Vec<RawObservation> {
    let old = canonical_path_entries(&before.path_entries);
    let new = canonical_path_entries(&after.path_entries);
    let mut events = Vec::new();
    for key in old.keys().filter(|key| !new.contains_key(*key)) {
        let entry = &old[key];
        events.push(system_observation(SystemObservation {
            category: Category::Configuration,
            event_type: EventType::Deleted,
            user_summary: format!("{entry} was removed from PATH"),
            technical_summary: format!("PATH entry removed: {entry}"),
            before: Some(json!(entry)),
            after: None,
            field: Some("PATH".to_owned()),
            occurred_at,
        }));
    }
    for key in new.keys().filter(|key| !old.contains_key(*key)) {
        let entry = &new[key];
        events.push(system_observation(SystemObservation {
            category: Category::Configuration,
            event_type: EventType::Created,
            user_summary: format!("{entry} was added to PATH"),
            technical_summary: format!("PATH entry added: {entry}"),
            before: None,
            after: Some(json!(entry)),
            field: Some("PATH".to_owned()),
            occurred_at,
        }));
    }
    events
}

fn canonical_path_entries(entries: &[String]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|entry| (entry.replace('/', "\\").to_ascii_lowercase(), entry.clone()))
        .collect()
}

fn tool_observations(
    before: &DevEnvironmentState,
    after: &DevEnvironmentState,
    occurred_at: i64,
) -> Vec<RawObservation> {
    let names: BTreeSet<&String> = before.tools.keys().chain(after.tools.keys()).collect();
    names
        .into_iter()
        .filter_map(|name| {
            let old = before.tools.get(name);
            let new = after.tools.get(name);
            if old == new {
                return None;
            }
            let display = crate::domain::tool::definition(name)
                .map(|tool| tool.display_name)
                .unwrap_or(name);
            let (event_type, summary) = match (old, new) {
                (None, Some(current)) => (
                    EventType::Created,
                    format!(
                        "{display} {} was added at {}",
                        version(current),
                        current.path
                    ),
                ),
                (Some(previous), None) => (
                    EventType::Deleted,
                    format!("{display} was removed from {}", previous.path),
                ),
                (Some(previous), Some(current)) => (
                    EventType::VersionChanged,
                    format!(
                        "{display} changed {} → {}",
                        version(previous),
                        version(current)
                    ),
                ),
                (None, None) => return None,
            };
            Some(system_observation(SystemObservation {
                category: Category::Runtime,
                event_type,
                user_summary: summary.clone(),
                technical_summary: summary,
                before: old.map(|state| json!(state)),
                after: new.map(|state| json!(state)),
                field: Some(name.clone()),
                occurred_at,
            }))
        })
        .collect()
}

fn version(tool: &ToolState) -> &str {
    tool.version.as_deref().unwrap_or("Unknown")
}

fn branch_observations(
    before: &DevEnvironmentState,
    after: &DevEnvironmentState,
    occurred_at: i64,
) -> Vec<RawObservation> {
    after
        .branches
        .iter()
        .filter_map(|(root, branch)| {
            let previous = before.branches.get(root)?;
            if previous == branch {
                return None;
            }
            let display = Path::new(root)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("watched folder");
            let summary = format!("Switched branch in {display}: {previous} → {branch}");
            Some(RawObservation {
                collector_id: ID,
                kind: observation_kind(
                    Category::Git,
                    EventType::ConfigurationChanged,
                    &summary,
                    &summary,
                ),
                entity: RawEntity {
                    identity: EntityIdentity::Project { path: root.clone() },
                    display_name: display.to_owned(),
                },
                before: Some(json!(previous)),
                after: Some(json!(branch)),
                field: Some("branch".to_owned()),
                occurred_at,
                details: None,
                related_entity_ids: Vec::new(),
                occurred_at_is_uncertain: false,
            })
        })
        .collect()
}

struct SystemObservation {
    category: Category,
    event_type: EventType,
    user_summary: String,
    technical_summary: String,
    before: Option<Value>,
    after: Option<Value>,
    field: Option<String>,
    occurred_at: i64,
}

fn system_observation(input: SystemObservation) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: observation_kind(
            input.category,
            input.event_type,
            &input.user_summary,
            &input.technical_summary,
        ),
        entity: RawEntity {
            identity: EntityIdentity::System,
            display_name: "Developer environment".to_owned(),
        },
        before: input.before,
        after: input.after,
        field: input.field,
        occurred_at: input.occurred_at,
        details: None,
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

fn observation_kind(
    category: Category,
    event_type: EventType,
    user_summary: &str,
    technical_summary: &str,
) -> ObservationKind {
    ObservationKind {
        category,
        event_type,
        severity: Severity::Info,
        confidence: Confidence::High,
        user_friendly_summary: user_summary.to_owned(),
        technical_summary: technical_summary.to_owned(),
        classifier: ClassifierContext::default(),
    }
}

pub(super) fn read_branches(roots: &[PathBuf]) -> BTreeMap<String, String> {
    roots
        .iter()
        .filter_map(|root| {
            let branch = read_branch(root)?;
            Some((root.to_string_lossy().into_owned(), branch))
        })
        .collect()
}

pub(super) fn read_branch(root: &Path) -> Option<String> {
    let head_path = git_head_path(root)?;
    let head = read_bounded_text(&head_path)?;
    parse_git_head(&head)
}

pub(super) fn git_head_path(root: &Path) -> Option<PathBuf> {
    let marker = root.join(".git");
    if marker.is_dir() {
        return Some(marker.join("HEAD"));
    }
    let indirection = read_bounded_text(&marker)?;
    let target = indirection.trim().strip_prefix("gitdir:")?.trim();
    if target.is_empty() || target.starts_with(r"\\") {
        return None;
    }
    let target = PathBuf::from(target);
    let git_dir = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    Some(git_dir.join("HEAD"))
}

fn read_bounded_text(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_GIT_CONTROL_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn parse_git_head(value: &str) -> Option<String> {
    let value = value.trim();
    value
        .strip_prefix("ref: refs/heads/")
        .filter(|branch| {
            !branch.is_empty()
                && branch.len() <= crate::app::validate::MAX_TEXT
                && !branch.chars().any(char::is_control)
        })
        .map(str::to_owned)
}

pub(super) fn same_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left.to_string_lossy()
        .replace('/', "\\")
        .eq_ignore_ascii_case(&right.to_string_lossy().replace('/', "\\"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn reads_normal_and_indirected_heads() {
        let root = crate::testing::temp_dir("dev-env-git-heads");
        let normal = root.join("normal");
        std::fs::create_dir_all(normal.join(".git")).unwrap();
        std::fs::write(normal.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        let worktree = root.join("worktree");
        let metadata = root.join("metadata");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::create_dir_all(&metadata).unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: ../metadata\n").unwrap();
        std::fs::write(metadata.join("HEAD"), "ref: refs/heads/feature/test\n").unwrap();
        let branches = read_branches(&[normal.clone(), worktree.clone()]);
        assert_eq!(branches[&normal.to_string_lossy().into_owned()], "main");
        assert_eq!(
            branches[&worktree.to_string_lossy().into_owned()],
            "feature/test"
        );
    }
}
