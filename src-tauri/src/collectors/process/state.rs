//! What the process collector remembers between cycles.
//!
//! Three things have to survive a cycle, a restart and a worker recreation
//! without ever inventing a fact: which programs storage has been told about,
//! which identity each running instance was given, and which of those identities
//! were earned by observing a start rather than by finding the program already
//! there.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::collectors::process::churn::Episodes;
use crate::collectors::stateful::{self, PendingTarget};
use crate::domain::ids::ProcessId;
use crate::platform::traits::ProcessRow;
use crate::storage::Baseline;

/// The durable baseline kind this collector owns.
pub const BASELINE_KIND: &str = "process_instances";

/// Browsers whose own subprocesses come and go with every tab.
///
/// The damping rule is deliberately narrower than "anything a browser spawns":
/// it also requires the child's image to match the parent's. A crash handler or
/// a GPU helper with its own executable name is a distinct program and stays
/// undamped, because guessing at helper naming is how a recorder starts hiding
/// the thing someone is looking for.
const BROWSER_IMAGES: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "opera.exe",
    "opera_gx.exe",
    "vivaldi.exe",
    "waterfox.exe",
    "librewolf.exe",
    "zen.exe",
    "arc.exe",
];

/// One live process instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedInstance {
    /// Process id, which identifies one live process at an instant and is
    /// reused by the operating system afterwards.
    pub pid: u32,
    /// Raw `FILETIME`, or `None` when the handle could not be opened. This is
    /// the last authoritative value observed for this live instance and is
    /// what separates it from a recycled pid. It is retained across a
    /// temporary access denial so a later readable snapshot can still detect
    /// pid reuse.
    pub creation: Option<u64>,
    /// Whether `creation` was observable in the current provider snapshot.
    ///
    /// Kept separate from the last authoritative value: identity continuity
    /// needs the latter, while confidence must never borrow evidence from an
    /// older snapshot after access was denied.
    #[serde(default)]
    pub creation_available: bool,
    /// The image name. Also the display name, and the weak identity used when
    /// no path is readable.
    pub image: String,
    /// The stable id, computed once and validated on the way back from storage.
    pub entity_id: ProcessId,
    /// Whether the stable id was derived from an executable path.
    ///
    /// Once an instance starts with only an image-derived id, a later readable
    /// path cannot retroactively make that weaker id globally unique. The bit
    /// therefore stays false for that instance even while current path access
    /// is recorded separately below.
    #[serde(default)]
    pub identity_path_based: bool,
    /// Whether a full executable path was readable. About 46% of processes deny
    /// that read to a non-elevated caller, and the answer is a fact about the
    /// machine worth keeping even when the path itself is not.
    pub path_available: bool,
    /// The full path, held in memory only.
    ///
    /// Never persisted: the id already encodes the path, and a baseline is
    /// exactly the place a redacted path would come back as a different
    /// program. A start event carries it while it is in hand.
    #[serde(skip)]
    pub executable: Option<String>,
    /// A browser's own helper subprocess, recorded and never escalated.
    pub browser_subtree: bool,
    /// Whether storage was told this instance started.
    ///
    /// A program that was already running when recording began has no observed
    /// start, so its eventual stop is not a matched pair and must not count
    /// toward a churn episode nobody watched.
    pub observed_start: bool,
}

/// The collector's whole durable memory, staged and acknowledged as one value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProcessState {
    /// What storage has been told, by pid.
    pub acknowledged: BTreeMap<u32, TrackedInstance>,
    /// The complete provider snapshot being drained after an oversized burst.
    ///
    /// New provider views wait until this target lands. Persisting the whole
    /// target preserves a short-lived process even when it has exited before a
    /// later chunk, while the timestamp and uncertainty remain those of the
    /// cycle that actually observed it.
    #[serde(default)]
    pub pending: Option<PendingTarget<BTreeMap<u32, TrackedInstance>>>,
    /// Open churn episodes.
    pub episodes: Episodes,
    /// Whether the silent first baseline has been taken.
    pub established: bool,
}

impl ProcessState {
    /// Adopt a durable baseline, and say whether the next diff spans a gap.
    ///
    /// An ordinary episode that straddled the gap is dropped: its pairs were
    /// counted partly in time nobody was watching, so publishing the total
    /// would be a claim the recorder cannot support. A durable pending target
    /// is different — every remaining transition was already observed before
    /// the gap — so its episode is retained until that exact backlog drains.
    pub fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        let (mut state, downtime) =
            stateful::hydrate::<Self>(baseline, now_ms, "the process collector's instance table");
        if downtime && state.pending.is_none() {
            state.episodes.clear();
        }
        (state, downtime)
    }
}

/// Whether one pid still names the process it named last cycle.
///
/// Creation time is the authoritative answer when both sides have one. When
/// either side could not open a handle, the image name decides: readability
/// flipping on its own is not a process ending, and treating it as one would
/// forge a stop/start pair for a program that never restarted.
pub fn same_instance(previous: &TrackedInstance, row: &ProcessRow) -> bool {
    match (previous.creation, row.creation_time) {
        (Some(before), Some(now)) => before == now,
        _ => previous.image.eq_ignore_ascii_case(&row.image_name),
    }
}

/// A first identity for a process the collector has not seen before.
fn fresh(row: &ProcessRow, parent_image: Option<&str>) -> TrackedInstance {
    let executable = row.executable_path.clone();
    let identity_source = executable.clone().unwrap_or_else(|| row.image_name.clone());
    TrackedInstance {
        pid: row.pid,
        creation: row.creation_time,
        creation_available: row.creation_time.is_some(),
        image: row.image_name.clone(),
        entity_id: ProcessId::from_executable(&identity_source),
        identity_path_based: executable.is_some(),
        path_available: executable.is_some(),
        executable,
        browser_subtree: is_browser_subprocess(&row.image_name, parent_image),
        observed_start: false,
    }
}

/// Refresh current-snapshot evidence without changing the stable identity.
///
/// A temporary denial must lower confidence immediately, but discarding a
/// creation time already observed for this live instance would make a later
/// pid reuse with the same image impossible to distinguish. Conversely, a
/// creation time that becomes readable after an earlier denial strengthens
/// subsequent reuse checks.
fn refreshed(
    existing: &TrackedInstance,
    row: &ProcessRow,
    keep_executable: bool,
) -> TrackedInstance {
    TrackedInstance {
        creation: row.creation_time.or(existing.creation),
        creation_available: row.creation_time.is_some(),
        image: row.image_name.clone(),
        path_available: row.executable_path.is_some(),
        executable: keep_executable
            .then(|| row.executable_path.clone())
            .flatten(),
        ..existing.clone()
    }
}

/// Whether an image is one of the browsers whose subprocess churn is noise.
fn is_browser_image(image: &str) -> bool {
    BROWSER_IMAGES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(image))
}

/// Whether a process is a browser's own helper: its parent is a known browser
/// and its image matches the parent's.
pub fn is_browser_subprocess(image: &str, parent_image: Option<&str>) -> bool {
    parent_image
        .is_some_and(|parent| is_browser_image(parent) && parent.eq_ignore_ascii_case(image))
}

/// Build this cycle's view of the machine, carrying identities forward.
///
/// Returns the instances and how many of them denied the executable-path read,
/// which the collector aggregates into one hourly line rather than logging per
/// process.
///
/// An executable path is carried only for an instance storage has not been told
/// about, because that is the only kind that can still produce a start event.
/// Keeping one for every running process would hold a string per program for the
/// life of the app and give the path somewhere to outlive the cycle that read it.
pub fn instances(
    rows: &[ProcessRow],
    previous: &ProcessState,
) -> (BTreeMap<u32, TrackedInstance>, u64) {
    let mut row_index = HashMap::with_capacity(rows.len());
    instances_with_index(rows, previous, &mut row_index)
}

/// Build a snapshot while reusing a caller-owned pid-to-row index.
///
/// The process collector calls this once a second. Indexing row positions
/// instead of cloning every image name keeps parent lookup in one reused hash
/// table. The returned ordered map deliberately owns its values because it can
/// become the collector's serializable atomic target; its measured cost is
/// guarded by the real-machine p95 test.
pub fn instances_with_index(
    rows: &[ProcessRow],
    previous: &ProcessState,
    row_index: &mut HashMap<u32, usize>,
) -> (BTreeMap<u32, TrackedInstance>, u64) {
    row_index.clear();
    row_index.reserve(rows.len());
    for (index, row) in rows.iter().enumerate() {
        row_index.insert(row.pid, index);
    }
    let mut instances = BTreeMap::new();
    let mut denials = 0u64;
    for row in rows {
        let instance = if let Some(existing) = previous
            .acknowledged
            .get(&row.pid)
            .filter(|existing| same_instance(existing, row))
        {
            refreshed(existing, row, false)
        } else if let Some(existing) = previous
            .pending
            .as_ref()
            .and_then(|pending| pending.target.get(&row.pid))
            .filter(|existing| same_instance(existing, row))
        {
            refreshed(existing, row, true)
        } else {
            let parent_image = row_index
                .get(&row.parent_pid)
                .and_then(|index| rows.get(*index))
                .map(|parent| parent.image_name.as_str());
            fresh(row, parent_image)
        };
        // Counted from the live row: the aggregate is a fact about what the
        // machine allowed this cycle.
        if row.executable_path.is_none() {
            denials += 1;
        }
        instances.insert(row.pid, instance);
    }
    (instances, denials)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::collectors::process::churn;

    fn row(pid: u32, image: &str, path: Option<&str>, creation: Option<u64>) -> ProcessRow {
        ProcessRow {
            pid,
            parent_pid: 0,
            image_name: image.to_owned(),
            executable_path: path.map(str::to_owned),
            creation_time: creation,
        }
    }

    #[test]
    fn a_process_that_denies_its_path_is_named_by_image_and_still_tracked() {
        let (instances, denials) = instances(
            &[
                row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(10)),
                row(2, "svchost.exe", None, None),
            ],
            &ProcessState::default(),
        );
        assert_eq!(denials, 1);
        let readable = &instances[&1];
        assert!(readable.path_available);
        assert!(readable.creation_available);
        assert_eq!(
            readable.entity_id,
            ProcessId::from_executable(r"C:\dev\api.exe")
        );
        let denied = &instances[&2];
        assert!(!denied.path_available);
        assert_eq!(denied.entity_id, ProcessId::from_executable("svchost.exe"));
        assert_eq!(denied.image, "svchost.exe");
    }

    #[test]
    fn a_carried_forward_identity_survives_a_path_read_flipping() {
        let first = instances(
            &[row(1, "api.exe", Some(r"C:\dev\api.exe"), None)],
            &ProcessState::default(),
        )
        .0;
        // A committed instance never carries a path — the id already encodes it
        // and committing strips it — so the carry-forward is checked against the
        // state production actually holds rather than a fresh snapshot.
        let acknowledged: BTreeMap<u32, TrackedInstance> = first
            .into_iter()
            .map(|(pid, mut instance)| {
                instance.executable = None;
                (pid, instance)
            })
            .collect();
        let state = ProcessState {
            acknowledged,
            ..Default::default()
        };
        let before = state.acknowledged[&1].entity_id.clone();

        // The handle is denied this cycle. Without a creation time the image
        // name decides continuity, so the identity must not move.
        let (second, denials) = instances(&[row(1, "api.exe", None, None)], &state);
        assert_eq!(denials, 1, "the machine denied the read this cycle");
        assert_eq!(second[&1].entity_id, before);
        assert_eq!(second[&1].executable, None);
        assert!(!second[&1].path_available);
        assert!(!second[&1].creation_available);
    }

    #[test]
    fn current_confidence_changes_without_forgetting_authoritative_creation_time() {
        let first = instances(
            &[row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(10))],
            &ProcessState::default(),
        )
        .0;
        let denied_state = ProcessState {
            acknowledged: first,
            ..Default::default()
        };

        let denied = instances(&[row(1, "api.exe", None, None)], &denied_state).0;
        assert_eq!(denied[&1].creation, Some(10));
        assert!(!denied[&1].creation_available);
        assert!(!denied[&1].path_available);
        assert_eq!(
            denied[&1].entity_id,
            denied_state.acknowledged[&1].entity_id
        );

        let denied_then_readable = ProcessState {
            acknowledged: denied,
            ..Default::default()
        };
        let reused = row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(11));
        assert!(
            !same_instance(&denied_then_readable.acknowledged[&1], &reused),
            "a denial between snapshots cannot hide later pid reuse"
        );
    }

    #[test]
    fn creation_time_that_becomes_readable_refreshes_a_weak_identity() {
        let first = instances(
            &[row(1, "api.exe", Some(r"C:\dev\api.exe"), None)],
            &ProcessState::default(),
        )
        .0;
        let state = ProcessState {
            acknowledged: first,
            ..Default::default()
        };
        let refreshed = instances(
            &[row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(10))],
            &state,
        )
        .0;
        assert_eq!(refreshed[&1].creation, Some(10));
        assert!(refreshed[&1].creation_available);
        assert!(refreshed[&1].path_available);
    }

    #[test]
    fn a_recycled_pid_is_a_different_instance() {
        let first = instances(
            &[row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(10))],
            &ProcessState::default(),
        )
        .0;
        let state = ProcessState {
            acknowledged: first,
            ..Default::default()
        };
        assert!(!same_instance(
            &state.acknowledged[&1],
            &row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(11))
        ));
        assert!(same_instance(
            &state.acknowledged[&1],
            &row(1, "api.exe", Some(r"C:\dev\api.exe"), Some(10))
        ));
    }

    #[test]
    fn only_a_browser_spawning_its_own_image_is_damped() {
        assert!(is_browser_subprocess("chrome.exe", Some("chrome.exe")));
        assert!(is_browser_subprocess("CHROME.EXE", Some("chrome.exe")));
        assert!(!is_browser_subprocess("chrome.exe", Some("explorer.exe")));
        assert!(!is_browser_subprocess(
            "chrome_crashpad_handler.exe",
            Some("chrome.exe")
        ));
        assert!(!is_browser_subprocess("chrome.exe", None));
        assert!(!is_browser_subprocess("notepad.exe", Some("notepad.exe")));
    }

    #[test]
    fn a_browser_helper_is_flagged_when_the_snapshot_is_built() {
        let (instances, _) = instances(
            &[
                ProcessRow {
                    pid: 10,
                    parent_pid: 1,
                    image_name: "chrome.exe".to_owned(),
                    executable_path: None,
                    creation_time: Some(5),
                },
                row(
                    1,
                    "chrome.exe",
                    Some(r"C:\Program Files\chrome.exe"),
                    Some(1),
                ),
            ],
            &ProcessState::default(),
        );
        assert!(instances[&10].browser_subtree);
        assert!(!instances[&1].browser_subtree);
    }

    fn baseline(snapshot: serde_json::Value, captured_at: i64, truncated: bool) -> Baseline {
        Baseline {
            collector_id: "process".to_owned(),
            entity_kind: BASELINE_KIND.to_owned(),
            snapshot,
            captured_at,
            truncated,
        }
    }

    #[test]
    fn a_baseline_that_cannot_be_read_re_baselines_instead_of_failing() {
        let corrupt = baseline(
            serde_json::json!({ "not": "a process baseline" }),
            1_000,
            false,
        );
        assert_eq!(
            ProcessState::hydrate(Some(corrupt), 2_000),
            (ProcessState::default(), false),
            "a corrupt row costs one silent re-baseline, not a parked collector"
        );

        let marker = baseline(serde_json::json!({ "truncated": true }), 1_000, true);
        assert_eq!(
            ProcessState::hydrate(Some(marker), 2_000),
            (ProcessState::default(), false)
        );
        assert_eq!(
            ProcessState::hydrate(None, 2_000),
            (ProcessState::default(), false)
        );
    }

    #[test]
    fn a_hydrated_baseline_spans_downtime_and_drops_its_episodes() {
        let state = ProcessState {
            established: true,
            episodes: BTreeMap::from([(
                ProcessId::from_executable("api.exe"),
                churn::Episode {
                    display_name: "api.exe".to_owned(),
                    anchor_ms: 1_000,
                    pairs: 9,
                    confidence: crate::domain::Confidence::High,
                },
            )]),
            ..Default::default()
        };
        let stored = serde_json::to_value(&state).unwrap();

        let (soon, downtime) = ProcessState::hydrate(
            Some(baseline(stored.clone(), 1_000, false)),
            1_000 + stateful::DOWNTIME_GAP_MS,
        );
        assert!(!downtime);
        assert_eq!(soon.episodes.len(), 1);
        assert!(soon.established);

        let (later, downtime) = ProcessState::hydrate(
            Some(baseline(stored, 1_000, false)),
            1_000 + stateful::DOWNTIME_GAP_MS + 1,
        );
        assert!(downtime);
        assert!(
            later.episodes.is_empty(),
            "pairs nobody watched must not be published as a count"
        );
        assert_eq!(later.acknowledged, soon.acknowledged);
    }

    #[test]
    fn a_stored_entity_id_that_is_not_a_process_id_is_rejected_on_the_way_in() {
        let state = ProcessState {
            established: true,
            ..Default::default()
        };
        let honest = serde_json::to_value(&state).unwrap();
        assert!(serde_json::from_value::<ProcessState>(honest).is_ok());

        let tampered = serde_json::json!({
            "acknowledged": { "1": {
                "pid": 1, "creation": null, "image": "api.exe",
                "entity_id": "process:sk-live-abcdef", "path_available": true,
                "browser_subtree": false, "observed_start": true
            }},
            "pending": null, "episodes": {}, "established": true
        });
        assert!(
            serde_json::from_value::<ProcessState>(tampered).is_err(),
            "a baseline cannot smuggle an arbitrary string in as an identity"
        );
    }
}
