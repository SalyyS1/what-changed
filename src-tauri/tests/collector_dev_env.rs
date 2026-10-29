#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use what_changed_lib::collectors::dev_env::{
    catalog_descriptor, DevEnvironmentCollector, DevEnvironmentNotificationGuard,
    DevEnvironmentProvider, ToolScan, ToolState, ID,
};
use what_changed_lib::collectors::testing::{live_context, LiveContext};
use what_changed_lib::collectors::{Collector, CollectorSnapshot, RawObservation, SnapshotContext};
use what_changed_lib::domain::EventType;
use what_changed_lib::platform::windows::registry::WindowsDevEnvironmentProvider;
use what_changed_lib::privacy::patterns::REDACTED;
use what_changed_lib::storage::{watched_location_repository, CollectorCommitReceipt};

#[derive(Clone, Default)]
struct ScriptedProvider {
    environment: Arc<Mutex<BTreeMap<String, String>>>,
    tools: Arc<Mutex<BTreeMap<String, ToolState>>>,
    probe_count: Arc<AtomicUsize>,
}

struct NoopGuard;
impl DevEnvironmentNotificationGuard for NoopGuard {}

impl DevEnvironmentProvider for ScriptedProvider {
    fn environment_variables(
        &self,
    ) -> what_changed_lib::error::AppResult<BTreeMap<String, String>> {
        Ok(self.environment.lock().unwrap().clone())
    }

    fn probe_tools(&self, _path: &str) -> ToolScan {
        self.probe_count.fetch_add(1, Ordering::AcqRel);
        ToolScan {
            tools: self.tools.lock().unwrap().clone(),
            complete: true,
        }
    }

    fn register_notifications(
        &self,
        _wake: what_changed_lib::collectors::WakeHandle,
        _cancel: what_changed_lib::collectors::CancelToken,
    ) -> what_changed_lib::error::AppResult<Box<dyn DevEnvironmentNotificationGuard>> {
        Ok(Box::new(NoopGuard))
    }
}

fn persist_cycle(
    collector: &mut DevEnvironmentCollector<ScriptedProvider>,
    live: &LiveContext,
    token: u64,
) -> Vec<RawObservation> {
    let context = SnapshotContext::for_devtools(token);
    let snapshot = collector.snapshot_batch(context).unwrap();
    let observations = match &snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations.clone(),
        CollectorSnapshot::Noop => Vec::new(),
        CollectorSnapshot::Legacy(_) => panic!("dev_env must use atomic snapshots"),
    };
    if let Some((commit_token, receipt)) =
        what_changed_lib::collectors::testing::persist_atomic_snapshot(&live.db, ID, snapshot)
            .unwrap()
    {
        collector.commit_succeeded(commit_token, &receipt).unwrap();
    }
    observations
}

fn register_root(live: &LiveContext, root: &std::path::Path) {
    let path = root.to_string_lossy().into_owned();
    live.db
        .write(move |connection| {
            watched_location_repository::insert(connection, &path, 4, 1).map(|_| ())
        })
        .unwrap();
}

fn cycle(
    collector: &mut DevEnvironmentCollector<ScriptedProvider>,
    token: u64,
) -> Vec<RawObservation> {
    let context = SnapshotContext::for_devtools(token);
    match collector.snapshot_batch(context).unwrap() {
        CollectorSnapshot::Atomic { observations, .. } => {
            collector
                .commit_succeeded(
                    context.token(),
                    &CollectorCommitReceipt { event_ids: vec![] },
                )
                .unwrap();
            observations
        }
        CollectorSnapshot::Noop => Vec::new(),
        CollectorSnapshot::Legacy(_) => panic!("dev_env must use atomic snapshots"),
    }
}

#[test]
fn env_path_tools_and_branch_changes_are_deterministic_and_redacted() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("my-api");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    register_root(&live, &root);
    assert_eq!(live.context.state.watched_locations().unwrap().len(), 1);

    let provider = ScriptedProvider::default();
    provider.environment.lock().unwrap().extend([
        ("PATH".to_owned(), r"C:\old;C:\keep".to_owned()),
        (
            "AWS_SECRET_ACCESS_KEY".to_owned(),
            "AKIAIOSFODNN7EXAMPLE-old-secret-value".to_owned(),
        ),
    ]);
    provider.tools.lock().unwrap().insert(
        "node".to_owned(),
        ToolState {
            path: r"C:\old\node.exe".to_owned(),
            version: Some("20.11.0".to_owned()),
        },
    );

    let mut collector =
        DevEnvironmentCollector::new(provider.clone(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    assert!(
        cycle(&mut collector, 1).is_empty(),
        "first run establishes a baseline"
    );

    provider
        .environment
        .lock()
        .unwrap()
        .insert("PATH".to_owned(), r"C:\keep;C:\new".to_owned());
    provider.environment.lock().unwrap().insert(
        "AWS_SECRET_ACCESS_KEY".to_owned(),
        "AKIAIOSFODNN7EXAMPLE-new-secret-value".to_owned(),
    );
    provider.tools.lock().unwrap().insert(
        "node".to_owned(),
        ToolState {
            path: r"C:\new\node.exe".to_owned(),
            version: Some("22.6.0".to_owned()),
        },
    );
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/feature/auth\n").unwrap();

    let observations = cycle(&mut collector, 2);
    let path_events = observations
        .iter()
        .filter(|event| event.field.as_deref() == Some("PATH"))
        .collect::<Vec<_>>();
    assert_eq!(path_events.len(), 2);
    assert!(path_events
        .iter()
        .any(|event| event.kind.event_type == EventType::Deleted));
    assert!(path_events
        .iter()
        .any(|event| event.kind.event_type == EventType::Created));
    assert!(path_events
        .iter()
        .any(|event| event.kind.user_friendly_summary == r"C:\old was removed from PATH"));
    assert!(path_events
        .iter()
        .any(|event| event.kind.user_friendly_summary == r"C:\new was added to PATH"));
    assert!(observations.iter().any(|event| {
        event.kind.event_type == EventType::VersionChanged
            && event.kind.user_friendly_summary == "Node.js changed 20.11.0 → 22.6.0"
    }));
    assert!(
        observations.iter().any(|event| {
            event.kind.user_friendly_summary == "Switched branch in my-api: main → feature/auth"
        }),
        "observations: {observations:?}"
    );

    let secrets = observations
        .into_iter()
        .filter(|event| event.field.as_deref() == Some("AWS_SECRET_ACCESS_KEY"))
        .collect::<Vec<_>>();
    assert_eq!(secrets.len(), 1);
    let secret = secrets.into_iter().next().unwrap();
    let normalized = what_changed_lib::collectors::normalize::Normalizer::new().normalize(secret);
    assert_eq!(
        normalized.get().before.as_ref().unwrap()["AWS_SECRET_ACCESS_KEY"],
        REDACTED
    );
    assert_eq!(
        normalized.get().after.as_ref().unwrap()["AWS_SECRET_ACCESS_KEY"],
        REDACTED
    );
}

#[test]
fn unknown_tool_version_is_an_informational_change_not_a_collector_error() {
    let live = live_context(ID).unwrap();
    let provider = ScriptedProvider::default();
    provider
        .environment
        .lock()
        .unwrap()
        .insert("PATH".to_owned(), r"C:\tools".to_owned());
    let mut collector =
        DevEnvironmentCollector::new(provider.clone(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    assert!(cycle(&mut collector, 1).is_empty());
    provider.tools.lock().unwrap().insert(
        "node".to_owned(),
        ToolState {
            path: r"C:\tools\node.exe".to_owned(),
            version: None,
        },
    );
    provider
        .environment
        .lock()
        .unwrap()
        .insert("PATH".to_owned(), r"C:\tools;C:\other".to_owned());
    let observations = cycle(&mut collector, 2);
    let tool_events = observations
        .iter()
        .filter(|event| event.field.as_deref() == Some("node"))
        .collect::<Vec<_>>();
    assert_eq!(tool_events.len(), 1);
    assert_eq!(
        tool_events[0].kind.user_friendly_summary,
        r"Node.js Unknown was added at C:\tools\node.exe"
    );
    assert_eq!(tool_events[0].kind.event_type, EventType::Created);
    assert_eq!(
        tool_events[0].kind.severity,
        what_changed_lib::domain::Severity::Info
    );
}

#[test]
fn environment_registry_read_stays_within_the_twenty_millisecond_budget() {
    let provider = WindowsDevEnvironmentProvider;
    provider.environment_variables().unwrap();
    let mut samples = Vec::with_capacity(20);
    for _ in 0..20 {
        let started = Instant::now();
        provider.environment_variables().unwrap();
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let p95 = samples[18];
    assert!(
        p95 <= Duration::from_millis(20),
        "environment registry read p95 {p95:?} exceeded 20 ms"
    );
}

#[test]
fn no_variables_tools_or_repositories_is_a_quiet_running_collector() {
    let live = live_context(ID).unwrap();
    let provider = ScriptedProvider::default();
    let mut collector = DevEnvironmentCollector::new(provider, catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    assert!(cycle(&mut collector, 1).is_empty());
    assert!(cycle(&mut collector, 2).is_empty());
}

#[test]
fn rejected_tool_scan_is_retried_because_its_clock_is_acknowledgement_safe() {
    let live = live_context(ID).unwrap();
    let provider = ScriptedProvider::default();
    let mut collector =
        DevEnvironmentCollector::new(provider.clone(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let first = SnapshotContext::for_devtools(1);
    let snapshot = collector.snapshot_batch(first).unwrap();
    assert!(matches!(snapshot, CollectorSnapshot::Atomic { .. }));
    collector.commit_failed(first.token()).unwrap();
    let second = SnapshotContext::for_devtools(2);
    let _ = collector.snapshot_batch(second).unwrap();
    assert_eq!(provider.probe_count.load(Ordering::Acquire), 2);
}

#[test]
fn git_file_indirection_tracks_a_worktree_head() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("worktree-api");
    let git_dir = live.directory.join("git-metadata");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(root.join(".git"), "gitdir: ../git-metadata\n").unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    register_root(&live, &root);
    let provider = ScriptedProvider::default();
    let mut collector = DevEnvironmentCollector::new(provider, catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    assert!(cycle(&mut collector, 1).is_empty());
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature/worktree\n").unwrap();
    let observations = cycle(&mut collector, 2);
    assert!(
        observations.iter().any(|event| {
            event.kind.user_friendly_summary
                == "Switched branch in worktree-api: main → feature/worktree"
        }),
        "observations: {observations:?}"
    );
}

#[test]
fn no_environment_value_reaches_raw_database_bytes() {
    let live = live_context(ID).unwrap();
    let provider = ScriptedProvider::default();
    let first = [
        ("EDITOR", "vim"),
        ("INTERNAL_HOST", "build.internal"),
        ("FEATURE_FLAG", "wc-phase18-feature-on-927"),
        ("AWS_SECRET_ACCESS_KEY", "AKIAIOSFODNN7EXAMPLE-first"),
    ];
    provider.environment.lock().unwrap().extend(
        first
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
    );
    let mut collector =
        DevEnvironmentCollector::new(provider.clone(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    assert!(persist_cycle(&mut collector, &live, 1).is_empty());
    let second = [
        ("EDITOR", "nano"),
        ("INTERNAL_HOST", "api.internal"),
        ("FEATURE_FLAG", "wc-phase18-feature-off-314"),
        ("AWS_SECRET_ACCESS_KEY", "AKIAIOSFODNN7EXAMPLE-second"),
    ];
    provider.environment.lock().unwrap().extend(
        second
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
    );
    assert_eq!(persist_cycle(&mut collector, &live, 2).len(), 4);
    live.db.flush().unwrap();
    live.db.close();
    let paths = live.db.paths();
    let mut bytes = std::fs::read(&paths.database).unwrap();
    for companion in [paths.wal(), paths.shm()] {
        if let Ok(more) = std::fs::read(companion) {
            bytes.extend(more);
        }
    }
    let artifact = String::from_utf8_lossy(&bytes);
    assert!(artifact.contains(what_changed_lib::privacy::REDACTED));
    for (_, value) in first.into_iter().chain(second) {
        assert!(
            !artifact.contains(value),
            "environment value leaked: {value}"
        );
    }
}

#[test]
fn restart_preserves_presence_evidence_without_persisting_value_comparison_markers() {
    let live = live_context(ID).unwrap();
    let first_value = "wc-low-entropy-choice-alpha";
    let second_value = "wc-low-entropy-choice-beta";
    let first_provider = ScriptedProvider::default();
    first_provider
        .environment
        .lock()
        .unwrap()
        .insert("EDITOR".to_owned(), first_value.to_owned());
    first_provider
        .environment
        .lock()
        .unwrap()
        .insert("REMOVED_TOOL".to_owned(), "private-old".to_owned());
    let mut first = DevEnvironmentCollector::new(first_provider, catalog_descriptor().unwrap());
    first.start(live.context.clone()).unwrap();
    assert!(persist_cycle(&mut first, &live, 1).is_empty());
    first.stop();

    let baseline = live
        .context
        .state
        .baseline("dev_environment")
        .unwrap()
        .unwrap();
    assert_eq!(baseline.snapshot["environment"]["EDITOR"], REDACTED);
    assert!(baseline.snapshot.get("environment_markers").is_none());
    assert!(baseline.snapshot.get("sealed_material").is_none());

    let second_provider = ScriptedProvider::default();
    second_provider
        .environment
        .lock()
        .unwrap()
        .insert("EDITOR".to_owned(), second_value.to_owned());
    second_provider
        .environment
        .lock()
        .unwrap()
        .insert("ADDED_TOOL".to_owned(), "private-new".to_owned());
    let mut second = DevEnvironmentCollector::new(second_provider, catalog_descriptor().unwrap());
    second.start(live.context.clone()).unwrap();
    let observations = persist_cycle(&mut second, &live, 2);
    assert_eq!(observations.len(), 2, "observations: {observations:?}");
    assert!(observations
        .iter()
        .all(|event| event.field.as_deref() != Some("EDITOR")));
    assert!(observations
        .iter()
        .any(|event| event.field.as_deref() == Some("ADDED_TOOL")
            && event.kind.event_type == EventType::Created));
    assert!(observations
        .iter()
        .any(|event| event.field.as_deref() == Some("REMOVED_TOOL")
            && event.kind.event_type == EventType::Deleted));

    live.db.flush().unwrap();
    live.db.close();
    let paths = live.db.paths();
    let mut bytes = std::fs::read(&paths.database).unwrap();
    for companion in [paths.wal(), paths.shm()] {
        if let Ok(more) = std::fs::read(companion) {
            bytes.extend(more);
        }
    }
    let artifact = String::from_utf8_lossy(&bytes);
    assert!(!artifact.contains(first_value));
    assert!(!artifact.contains(second_value));
}

#[test]
fn large_environment_burst_drains_in_acknowledged_prefixes() {
    let live = live_context(ID).unwrap();
    let provider = ScriptedProvider::default();
    for index in 0..250 {
        provider
            .environment
            .lock()
            .unwrap()
            .insert(format!("BURST_{index:03}"), format!("before-{index}"));
    }
    let mut collector =
        DevEnvironmentCollector::new(provider.clone(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    assert!(cycle(&mut collector, 1).is_empty());

    for index in 0..250 {
        provider
            .environment
            .lock()
            .unwrap()
            .insert(format!("BURST_{index:03}"), format!("after-{index}"));
    }

    let mut all = Vec::new();
    let mut largest = 0;
    for token in 2..=4 {
        let context = SnapshotContext::for_devtools(token);
        let snapshot = collector.snapshot_batch(context).unwrap();
        let observations = match snapshot {
            CollectorSnapshot::Atomic {
                observations,
                token,
                ..
            } => {
                largest = largest.max(observations.len());
                all.extend(observations.iter().cloned());
                collector
                    .commit_succeeded(token, &CollectorCommitReceipt { event_ids: vec![] })
                    .unwrap();
                observations
            }
            CollectorSnapshot::Noop => Vec::new(),
            CollectorSnapshot::Legacy(_) => panic!("dev_env must use atomic snapshots"),
        };
        if observations.is_empty() && !all.is_empty() {
            break;
        }
    }
    assert_eq!(all.len(), 250, "the whole burst must be delivered");
    assert!(largest <= what_changed_lib::storage::MAX_ATOMIC_OBSERVATIONS);
    assert_eq!(
        all.iter()
            .filter_map(|event| event.field.as_deref())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        250,
        "prefix commits must not duplicate or drop fields"
    );
}
