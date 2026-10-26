//! Docker state-machine acceptance checks; no live daemon or real sleeps.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::cell::Cell;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Once, OnceLock};
use std::time::{Duration, Instant};

use tracing_subscriber::layer::SubscriberExt;

use what_changed_lib::collectors::docker::{
    catalog_descriptor, reconnect_delay, DockerCollector, UNAVAILABLE_MESSAGE,
};
use what_changed_lib::collectors::docker_protocol::MAX_EVENT_LINE_BYTES;
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{
    Collector, CollectorSnapshot, ObservationClock, RawObservation, RawRelationshipSpec,
    SnapshotContext,
};
use what_changed_lib::domain::{DockerCollectorState, Importance, RelationshipKind};
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::platform::traits::DockerTransport;
use what_changed_lib::storage::CollectorCommitReceipt;

const VERSION: &[u8] = include_bytes!("fixtures/docker/version.json");
const CONTAINERS: &[u8] = include_bytes!("fixtures/docker/containers.json");
const IMAGES: &[u8] = include_bytes!("fixtures/docker/images.json");
const EVENTS: &[u8] = include_bytes!("fixtures/docker/events-stream.ndjson");

enum GetStep {
    Body(Vec<u8>),
    Error(&'static str),
    HttpStatus(u16),
}

enum StreamStep {
    Body(Vec<u8>),
    Timeout,
}

#[derive(Default)]
struct Script {
    gets: VecDeque<GetStep>,
    streams: VecDeque<StreamStep>,
    paths: Vec<String>,
}

#[derive(Clone, Default)]
struct SharedTransport(Arc<Mutex<Script>>);

impl SharedTransport {
    fn scripted(gets: Vec<GetStep>, streams: Vec<StreamStep>) -> Self {
        Self(Arc::new(Mutex::new(Script {
            gets: gets.into(),
            streams: streams.into(),
            paths: Vec::new(),
        })))
    }

    fn paths(&self) -> Vec<String> {
        lock(&self.0).paths.clone()
    }
}

impl DockerTransport for SharedTransport {
    fn get(&mut self, path: &str) -> AppResult<Vec<u8>> {
        let mut script = lock(&self.0);
        script.paths.push(path.to_owned());
        match script.gets.pop_front() {
            Some(GetStep::Body(body)) => Ok(body),
            Some(GetStep::Error(reason)) => Err(AppError::Platform {
                operation: "scripted Docker request",
                reason: reason.to_owned(),
            }),
            Some(GetStep::HttpStatus(status)) => Err(AppError::DockerHttpStatus { status }),
            None => Err(AppError::Platform {
                operation: "scripted Docker request",
                reason: "unavailable".to_owned(),
            }),
        }
    }

    fn stream(&mut self, path: &str) -> AppResult<Box<dyn BufRead + Send>> {
        let mut script = lock(&self.0);
        script.paths.push(path.to_owned());
        match script.streams.pop_front() {
            Some(StreamStep::Body(body)) => Ok(Box::new(Cursor::new(body))),
            Some(StreamStep::Timeout) => Ok(Box::new(BufReader::new(TimeoutReader))),
            None => Err(AppError::Platform {
                operation: "scripted Docker stream",
                reason: "unavailable".to_owned(),
            }),
        }
    }
}

struct TimeoutReader;

impl Read for TimeoutReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::TimedOut, "idle"))
    }
}

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    fn at(value: i64) -> Self {
        Self(Arc::new(AtomicI64::new(value)))
    }

    fn advance(&self, millis: i64) {
        self.0.fetch_add(millis, Ordering::AcqRel);
    }
}

impl ObservationClock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

type TestCollector = DockerCollector<SharedTransport, ManualClock>;

#[derive(Clone, Default)]
struct LogRecorder(Arc<Mutex<Vec<u8>>>);

thread_local! {
    static CAPTURE_TEST_LOGS: Cell<bool> = const { Cell::new(false) };
}

impl Write for LogRecorder {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if CAPTURE_TEST_LOGS.with(Cell::get) {
            lock(&self.0).extend_from_slice(buffer);
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl LogRecorder {
    fn clear(&self) {
        lock(&self.0).clear();
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&lock(&self.0)).into_owned()
    }
}

fn test_log_recorder() -> &'static LogRecorder {
    static RECORDER: OnceLock<LogRecorder> = OnceLock::new();
    RECORDER.get_or_init(LogRecorder::default)
}

fn initialize_test_tracing() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .without_time()
                .with_writer(|| test_log_recorder().clone()),
        );
        tracing::subscriber::set_global_default(subscriber)
            .expect("collector_docker owns this integration-test subscriber");
    });
}

struct ThreadLogCapture;

impl ThreadLogCapture {
    fn start() -> Self {
        initialize_test_tracing();
        test_log_recorder().clear();
        CAPTURE_TEST_LOGS.with(|enabled| enabled.set(true));
        Self
    }
}

impl Drop for ThreadLogCapture {
    fn drop(&mut self) {
        CAPTURE_TEST_LOGS.with(|enabled| enabled.set(false));
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn connection_steps(containers: &[u8], images: &[u8]) -> Vec<GetStep> {
    connection_steps_with_version(containers, images, VERSION)
}

fn connection_steps_with_version(containers: &[u8], images: &[u8], version: &[u8]) -> Vec<GetStep> {
    vec![
        GetStep::Body(b"OK".to_vec()),
        GetStep::Body(version.to_vec()),
        GetStep::Body(containers.to_vec()),
        GetStep::Body(images.to_vec()),
    ]
}

fn event(kind: &str, name: &str, action: &str, at: i64, extra: &str) -> String {
    let image = match name {
        "wc-db" => "postgres:16",
        "wc-api" => "wc-api:dev",
        _ => "api:dev",
    };
    format!(
        r#"{{"Type":"{kind}","Action":"{action}","Actor":{{"ID":"{name}000000000000","Attributes":{{"name":"{name}","image":"{image}"{extra}}}}},"time":{at},"timeNano":0}}"#
    )
}

fn start(transport: SharedTransport) -> (TestCollector, ManualClock) {
    initialize_test_tracing();
    let clock = ManualClock::at(1_725_000_000_000);
    let mut collector =
        DockerCollector::with_clock(transport, catalog_descriptor().unwrap(), clock.clone());
    collector
        .start(live_context("docker").unwrap().context)
        .unwrap();
    (collector, clock)
}

fn snapshot(collector: &mut TestCollector, token: u64) -> CollectorSnapshot {
    collector
        .snapshot_batch(SnapshotContext::for_devtools(token))
        .unwrap()
}

fn acknowledge(
    collector: &mut TestCollector,
    snapshot: CollectorSnapshot,
) -> (Vec<RawObservation>, Vec<RawRelationshipSpec>) {
    match snapshot {
        CollectorSnapshot::Atomic {
            token,
            observations,
            relationships,
            ..
        } => {
            collector
                .commit_succeeded(token, &CollectorCommitReceipt { event_ids: vec![] })
                .unwrap();
            (observations, relationships)
        }
        CollectorSnapshot::Noop => (Vec::new(), Vec::new()),
        CollectorSnapshot::Legacy(_) => panic!("Docker must use atomic snapshots"),
    }
}

fn connect(collector: &mut TestCollector, token: u64) {
    let initial = snapshot(collector, token);
    assert!(matches!(initial, CollectorSnapshot::Atomic { .. }));
    acknowledge(collector, initial);
}

fn wait_for_atomic(collector: &mut TestCollector, token: u64) -> CollectorSnapshot {
    for _ in 0..2_000 {
        let next = snapshot(collector, token);
        if matches!(next, CollectorSnapshot::Atomic { .. }) {
            return next;
        }
        std::thread::yield_now();
    }
    panic!("reader thread did not deliver an atomic batch")
}

fn wait_for_state(collector: &mut TestCollector, state: DockerCollectorState, token: u64) {
    for _ in 0..2_000 {
        if collector.machine_state() == state {
            return;
        }
        let next = snapshot(collector, token);
        if matches!(next, CollectorSnapshot::Atomic { .. }) {
            acknowledge(collector, next);
        }
        std::thread::yield_now();
    }
    panic!("collector did not reach {state}")
}

#[test]
fn five_absent_minutes_are_quiet_and_log_only_one_state_transition() {
    let capture = ThreadLogCapture::start();
    {
        let transport =
            SharedTransport::scripted((0..6).map(|_| GetStep::Error("missing")).collect(), vec![]);
        let (mut collector, clock) = start(transport);
        for token in 1..=6 {
            assert!(matches!(
                snapshot(&mut collector, token),
                CollectorSnapshot::Noop
            ));
            clock.advance(60_000);
        }
        assert_eq!(collector.machine_state(), DockerCollectorState::Unavailable);
        assert_eq!(collector.transition_count(), 1);
    }
    let logs = test_log_recorder().text();
    assert_eq!(logs.matches("Docker collector state changed").count(), 1);
    assert!(!logs.contains(" ERROR "), "logs: {logs}");
    assert_eq!(
        UNAVAILABLE_MESSAGE,
        "Docker isn't running. We'll start recording container changes when it does."
    );
    drop(capture);
}

#[test]
fn version_143_falls_back_to_141_only_for_typed_http_400() {
    let gets = vec![
        GetStep::Body(b"OK".to_vec()),
        GetStep::HttpStatus(400),
        GetStep::Body(VERSION.to_vec()),
        GetStep::Body(CONTAINERS.to_vec()),
        GetStep::Body(IMAGES.to_vec()),
    ];
    let transport = SharedTransport::scripted(gets, vec![StreamStep::Timeout]);
    let probe = transport.clone();
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let paths = probe.paths();
    assert!(paths.contains(&"/v1.43/version".to_owned()));
    assert!(paths.contains(&"/v1.41/version".to_owned()));
    assert!(paths.iter().any(|path| path.starts_with("/v1.41/events")));
}

#[test]
fn complete_frames_survive_disconnect_and_resync_is_silent() {
    let mut gets = connection_steps(CONTAINERS, IMAGES);
    gets.extend(connection_steps(CONTAINERS, IMAGES));
    let stream = format!(
        "{}\n{}\n",
        event("container", "wc-api", "start", 1_725_000_100, ""),
        event(
            "container",
            "wc-api",
            "die",
            1_725_000_101,
            r#", "exitCode":"1""#,
        )
    )
    .replace("wc-api000000000000", "444444444444bbbbbbbb");
    let transport = SharedTransport::scripted(
        gets,
        vec![StreamStep::Body(stream.into_bytes()), StreamStep::Timeout],
    );
    let (mut collector, clock) = start(transport);
    connect(&mut collector, 1);
    let mut delivered = Vec::new();
    for _ in 0..2_000 {
        let pending = snapshot(&mut collector, 2);
        if matches!(pending, CollectorSnapshot::Atomic { .. }) {
            delivered.extend(acknowledge(&mut collector, pending).0);
        }
        if collector.machine_state() == DockerCollectorState::Reconnecting {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(delivered.len(), 2, "both complete frames survive EOF");
    assert_eq!(
        collector.machine_state(),
        DockerCollectorState::Reconnecting
    );

    assert_eq!(collector.retry_not_before_ms(), clock.now_ms() + 2_000);
    assert!(matches!(
        snapshot(&mut collector, 4),
        CollectorSnapshot::Noop
    ));
    clock.advance(2_000);
    assert!(matches!(
        snapshot(&mut collector, 5),
        CollectorSnapshot::Noop
    ));
    assert_eq!(collector.machine_state(), DockerCollectorState::Available);
}

#[test]
fn every_fixture_event_has_the_expected_entity_and_importance() {
    let transport = SharedTransport::scripted(
        connection_steps(CONTAINERS, IMAGES),
        vec![StreamStep::Body(EVENTS.to_vec())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let mut delivered = Vec::new();
    for token in 2..100 {
        let pending = snapshot(&mut collector, token);
        if matches!(pending, CollectorSnapshot::Atomic { .. }) {
            delivered.extend(acknowledge(&mut collector, pending).0);
        }
        if collector.machine_state() == DockerCollectorState::Reconnecting {
            break;
        }
        std::thread::yield_now();
    }
    let normalized = delivered
        .into_iter()
        .map(|event| Normalizer::new().normalize(event))
        .filter(|event| {
            event.get().metadata.as_ref().unwrap()["subtype"] != "docker/compose_project_changed"
        })
        .collect::<Vec<_>>();
    let actual = normalized
        .iter()
        .map(|event| {
            let event = event.get();
            (
                event.metadata.as_ref().unwrap()["subtype"]
                    .as_str()
                    .unwrap(),
                event.entity_id.as_str(),
                event.importance,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (
                "docker/container_created",
                "container:111111111111",
                Importance::Informational
            ),
            (
                "docker/container_started",
                "container:111111111111",
                Importance::Informational
            ),
            (
                "docker/container_stopped",
                "container:111111111111",
                Importance::Informational
            ),
            (
                "docker/container_died",
                "container:111111111111",
                Importance::Important
            ),
            (
                "docker/container_removed",
                "container:111111111111",
                Importance::Informational
            ),
            (
                "docker/image_pulled",
                "image:sha256:222222222222",
                Importance::Informational
            ),
            (
                "docker/image_removed",
                "image:sha256:222222222222",
                Importance::Informational
            ),
            (
                "docker/volume_created",
                "docker_volume:wc-data",
                Importance::Informational
            ),
            (
                "docker/volume_removed",
                "docker_volume:wc-data",
                Importance::Informational
            ),
            (
                "docker/network_created",
                "docker_network:333333333333",
                Importance::Informational
            ),
            (
                "docker/network_removed",
                "docker_network:333333333333",
                Importance::Informational
            ),
        ]
    );
    let died = normalized
        .iter()
        .find(|event| event.get().metadata.as_ref().unwrap()["subtype"] == "docker/container_died")
        .unwrap();
    assert_eq!(died.get().metadata.as_ref().unwrap()["exit_code"], 137);
}

#[test]
fn reconnect_deadlines_follow_real_two_to_thirty_second_backoff() {
    let mut gets = connection_steps(b"[]", b"[]");
    gets.push(GetStep::Error("still unavailable"));
    let transport = SharedTransport::scripted(gets, vec![StreamStep::Body(Vec::new())]);
    let probe = transport.clone();
    let (mut collector, clock) = start(transport);
    connect(&mut collector, 1);
    wait_for_state(&mut collector, DockerCollectorState::Reconnecting, 2);
    let requests = probe.paths().len();

    clock.advance(1_999);
    assert!(matches!(
        snapshot(&mut collector, 3),
        CollectorSnapshot::Noop
    ));
    assert_eq!(probe.paths().len(), requests, "no request before deadline");
    clock.advance(1);
    assert!(matches!(
        snapshot(&mut collector, 4),
        CollectorSnapshot::Noop
    ));
    assert_eq!(collector.next_reconnect_delay(), Duration::from_secs(4));
    assert_eq!(collector.retry_not_before_ms(), clock.now_ms() + 4_000);
    assert_eq!(reconnect_delay(100), Duration::from_secs(30));
}

#[test]
fn repeated_reconnect_failures_return_to_unavailable_state() {
    let mut gets = connection_steps(b"[]", b"[]");
    gets.extend((0..5).map(|_| GetStep::Error("daemon stopped")));
    let transport = SharedTransport::scripted(gets, vec![StreamStep::Body(Vec::new())]);
    let (mut collector, clock) = start(transport);
    connect(&mut collector, 1);
    wait_for_state(&mut collector, DockerCollectorState::Reconnecting, 2);

    for (index, delay_ms) in [2_000, 4_000, 8_000, 16_000, 30_000]
        .into_iter()
        .enumerate()
    {
        clock.advance(delay_ms);
        assert!(matches!(
            snapshot(&mut collector, 10 + u64::try_from(index).unwrap()),
            CollectorSnapshot::Noop
        ));
        if delay_ms < 30_000 {
            assert_eq!(
                collector.machine_state(),
                DockerCollectorState::Reconnecting,
                "the collector must remain reconnecting until a capped 30-second retry fails"
            );
            assert_eq!(
                collector.retry_not_before_ms(),
                clock.now_ms()
                    + i64::try_from(reconnect_delay(u32::try_from(index + 1).unwrap()).as_millis())
                        .unwrap()
            );
        }
    }
    assert_eq!(collector.machine_state(), DockerCollectorState::Unavailable);
    assert_eq!(collector.retry_not_before_ms(), 0);
}

#[test]
fn same_name_with_new_id_is_remove_plus_create_on_resync() {
    let recreated = String::from_utf8(CONTAINERS.to_vec())
        .unwrap()
        .replace("111111111111aaaaaaaa", "999999999999cccccccc");
    let mut gets = connection_steps(CONTAINERS, IMAGES);
    gets.extend(connection_steps(recreated.as_bytes(), IMAGES));
    let transport = SharedTransport::scripted(
        gets,
        vec![StreamStep::Body(Vec::new()), StreamStep::Timeout],
    );
    let (mut collector, clock) = start(transport);
    connect(&mut collector, 1);
    wait_for_state(&mut collector, DockerCollectorState::Reconnecting, 2);
    clock.advance(2_000);
    let pending = snapshot(&mut collector, 3);
    let changed = acknowledge(&mut collector, pending).0;
    let subtypes: Vec<_> = changed
        .iter()
        .map(|row| row.details.as_ref().unwrap()["subtype"].as_str().unwrap())
        .collect();
    assert!(subtypes.contains(&"docker/container_removed"));
    assert!(subtypes.contains(&"docker/container_created"));
}

#[test]
fn daemon_version_change_is_recorded_from_the_resync_baseline() {
    let changed_version = String::from_utf8(VERSION.to_vec())
        .unwrap()
        .replace("27.2.0", "28.0.0");
    let mut gets = connection_steps(CONTAINERS, IMAGES);
    gets.extend(connection_steps_with_version(
        CONTAINERS,
        IMAGES,
        changed_version.as_bytes(),
    ));
    let transport = SharedTransport::scripted(
        gets,
        vec![StreamStep::Body(Vec::new()), StreamStep::Timeout],
    );
    let (mut collector, clock) = start(transport);
    connect(&mut collector, 1);
    wait_for_state(&mut collector, DockerCollectorState::Reconnecting, 2);
    clock.advance(2_000);
    let pending = snapshot(&mut collector, 3);
    let changed = acknowledge(&mut collector, pending).0;
    assert!(changed.iter().any(|row| {
        row.details.as_ref().unwrap()["subtype"] == "docker/daemon_changed"
            && row.before.as_ref().unwrap() == "27.2.0"
            && row.after.as_ref().unwrap() == "28.0.0"
    }));
}

#[test]
fn lifecycle_event_without_ports_preserves_authoritative_baseline_ports() {
    let mut gets = connection_steps(CONTAINERS, IMAGES);
    gets.extend(connection_steps(CONTAINERS, IMAGES));
    let stream = format!(
        "{}\n",
        event("container", "wc-db", "start", 1_725_000_100, "")
    )
    .replace("wc-db000000000000", "111111111111aaaaaaaa");
    let transport = SharedTransport::scripted(
        gets,
        vec![StreamStep::Body(stream.into_bytes()), StreamStep::Timeout],
    );
    let (mut collector, clock) = start(transport);
    connect(&mut collector, 1);
    let pending = wait_for_atomic(&mut collector, 2);
    acknowledge(&mut collector, pending);
    wait_for_state(&mut collector, DockerCollectorState::Reconnecting, 3);
    clock.advance(2_000);
    assert!(matches!(
        snapshot(&mut collector, 4),
        CollectorSnapshot::Noop
    ));
}

#[test]
fn failed_atomic_commit_replays_volume_network_and_compose_exactly() {
    let stream = format!(
        "{}\n{}\n{}\n{}\n",
        event("volume", "data", "create", 1_725_000_100, ""),
        event("network", "private", "create", 1_725_000_101, ""),
        event(
            "container",
            "api-1",
            "start",
            1_725_000_102,
            r#", "com.docker.compose.project":"demo""#,
        ),
        event(
            "container",
            "api-2",
            "start",
            1_725_000_103,
            r#", "com.docker.compose.project":"demo""#,
        )
    );
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(stream.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let mut all = Vec::new();
    let mut replayed_volume = false;
    let mut replayed_compose = false;
    for attempt in 0..2_000 {
        let token = 2 + attempt;
        let pending = snapshot(&mut collector, token);
        let kinds = subtypes(&pending);
        if kinds.is_empty() {
            std::thread::yield_now();
            continue;
        }
        let should_replay = (!replayed_volume
            && kinds.iter().any(|kind| kind == "docker/volume_created"))
            || (!replayed_compose
                && kinds
                    .iter()
                    .any(|kind| kind == "docker/compose_project_changed"));
        if should_replay {
            let pending_token = match &pending {
                CollectorSnapshot::Atomic { token, .. } => *token,
                _ => unreachable!(),
            };
            collector.commit_failed(pending_token).unwrap();
            let replay = snapshot(&mut collector, token + 100);
            assert_eq!(subtypes(&replay), kinds);
            replayed_volume |= kinds.iter().any(|kind| kind == "docker/volume_created");
            replayed_compose |= kinds
                .iter()
                .any(|kind| kind == "docker/compose_project_changed");
            all.extend(acknowledge(&mut collector, replay).0);
        } else {
            all.extend(acknowledge(&mut collector, pending).0);
        }
        if replayed_volume
            && replayed_compose
            && all
                .iter()
                .any(|row| row.details.as_ref().unwrap()["subtype"] == "docker/network_created")
        {
            break;
        }
    }
    let all_subtypes: Vec<_> = all
        .iter()
        .map(|row| {
            row.details.as_ref().unwrap()["subtype"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert!(replayed_volume, "volume batch was replayed");
    assert!(replayed_compose, "compose batch was replayed");
    assert!(all_subtypes.contains(&"docker/network_created".to_owned()));
}

#[test]
fn six_distinct_compose_containers_emit_exactly_one_project_summary() {
    let stream = (0..6)
        .map(|index| {
            event(
                "container",
                &format!("api-{index}"),
                "start",
                1_725_000_100 + index,
                r#", "com.docker.compose.project":"demo""#,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(stream.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);

    let mut delivered = Vec::new();
    for token in 2..100 {
        let pending = snapshot(&mut collector, token);
        if matches!(pending, CollectorSnapshot::Atomic { .. }) {
            delivered.extend(acknowledge(&mut collector, pending).0);
        }
        if collector.machine_state() == DockerCollectorState::Reconnecting {
            break;
        }
        std::thread::yield_now();
    }
    let subtypes = delivered
        .iter()
        .map(|row| row.details.as_ref().unwrap()["subtype"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        subtypes
            .iter()
            .filter(|subtype| **subtype == "docker/compose_project_changed")
            .count(),
        1
    );
    assert_eq!(
        subtypes
            .iter()
            .filter(|subtype| **subtype == "docker/container_started")
            .count(),
        6
    );
    assert!(delivered
        .into_iter()
        .filter(|row| row.details.as_ref().unwrap()["subtype"] == "docker/container_started")
        .map(|row| Normalizer::new().normalize(row))
        .all(|row| row.get().importance == Importance::Informational));
}

#[test]
fn duplicate_container_identity_does_not_trigger_compose_summary() {
    let line = event(
        "container",
        "api-1",
        "start",
        1_725_000_100,
        r#", "com.docker.compose.project":"demo""#,
    );
    let stream = format!("{line}\n{line}\n");
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(stream.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let pending = wait_for_atomic(&mut collector, 2);
    let subtypes = subtypes(&pending);
    acknowledge(&mut collector, pending);
    assert!(!subtypes
        .iter()
        .any(|subtype| subtype == "docker/compose_project_changed"));
}

#[test]
fn oversized_live_frame_disconnects_without_emitting_an_event() {
    let oversized = vec![b'x'; MAX_EVENT_LINE_BYTES + 1];
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(oversized)],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    wait_for_state(&mut collector, DockerCollectorState::Reconnecting, 2);
    assert!(matches!(
        snapshot(&mut collector, 3),
        CollectorSnapshot::Noop
    ));
}

#[test]
fn busy_volume_and_network_stream_applies_backpressure_without_losing_frames() {
    const EVENT_COUNT: usize = 600;
    let stream = (0..EVENT_COUNT)
        .map(|index| {
            let (kind, prefix) = if index % 2 == 0 {
                ("volume", "volume")
            } else {
                ("network", "network")
            };
            event(
                kind,
                &format!("{prefix}-{index}"),
                "create",
                1_725_000_100 + i64::try_from(index).unwrap(),
                "",
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(stream.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let mut delivered = 0;
    let mut widest = 0;
    for token in 2..2_000 {
        let pending = snapshot(&mut collector, token);
        if let CollectorSnapshot::Atomic { observations, .. } = &pending {
            widest = widest.max(observations.len());
            delivered += observations.len();
            acknowledge(&mut collector, pending);
        }
        if delivered == EVENT_COUNT {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(
        delivered, EVENT_COUNT,
        "bounded backpressure must preserve lifecycle frames beyond queue capacity"
    );
    assert!(widest <= 128, "each atomic batch must remain committable");
}

#[test]
fn shutdown_cancels_a_producer_blocked_by_backpressure() {
    let stream = (0..600)
        .map(|index| {
            event(
                "volume",
                &format!("blocked-{index}"),
                "create",
                1_725_000_100 + index,
                "",
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(stream.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    std::thread::sleep(Duration::from_millis(20));

    let started = Instant::now();
    collector.stop();
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "bounded backpressure must remain cancellation-aware"
    );
}

#[test]
fn nonzero_die_is_important_and_publication_is_structural_evidence() {
    let died = format!(
        "{}\n",
        event(
            "container",
            "wc-db",
            "die",
            1_725_000_200,
            r#", "exitCode":"137", "host_ip":"127.0.0.1", "host_port":"5432", "container_port":"5432", "protocol":"tcp""#,
        )
    )
    .replace("wc-db000000000000", "111111111111aaaaaaaa");
    let transport = SharedTransport::scripted(
        connection_steps(CONTAINERS, IMAGES),
        vec![StreamStep::Body(died.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let pending = wait_for_atomic(&mut collector, 2);
    let (observations, relationships) = acknowledge(&mut collector, pending);
    let normalized = Normalizer::new().normalize(observations[0].clone());
    assert_eq!(normalized.get().importance, Importance::Important);
    assert_eq!(
        normalized.get().metadata.as_ref().unwrap()["exit_code"],
        137
    );
    assert_eq!(relationships[0].kind, RelationshipKind::Publishes);
}

#[test]
fn a_reserved_user_label_cannot_spoof_exit_severity() {
    let containers = br#"[{"Id":"111111111111aaaaaaaa","Names":["/wc-db"],"Image":"postgres:16","State":"running","Status":"Up","Labels":{"exitCode":"0"},"Ports":[]}]"#;
    let died = format!(
        "{}\n",
        event(
            "container",
            "wc-db",
            "die",
            1_725_000_200,
            r#", "exitCode":"137""#,
        )
    )
    .replace("wc-db000000000000", "111111111111aaaaaaaa");
    let transport = SharedTransport::scripted(
        connection_steps(containers, b"[]"),
        vec![StreamStep::Body(died.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let pending = wait_for_atomic(&mut collector, 2);
    let (observations, _) = acknowledge(&mut collector, pending);
    let normalized = Normalizer::new().normalize(observations[0].clone());
    assert_eq!(normalized.get().importance, Importance::Informational);
    assert_eq!(
        normalized.get().metadata.as_ref().unwrap()["exit_code"],
        serde_json::Value::Null
    );
}

#[test]
fn flattened_event_labels_cannot_forge_port_relationships() {
    let started = format!(
        "{}\n",
        event(
            "container",
            "api",
            "start",
            1_725_000_200,
            r#", "host_ip":"127.0.0.1", "host_port":"65535", "container_port":"1", "protocol":"tcp""#,
        )
    );
    let transport = SharedTransport::scripted(
        connection_steps(b"[]", b"[]"),
        vec![StreamStep::Body(started.into_bytes())],
    );
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let pending = wait_for_atomic(&mut collector, 2);
    let (_, relationships) = acknowledge(&mut collector, pending);
    assert!(relationships.is_empty());
}

#[test]
fn shutdown_joins_an_idle_reader_within_the_transport_timeout_bound() {
    let transport =
        SharedTransport::scripted(connection_steps(b"[]", b"[]"), vec![StreamStep::Timeout]);
    let (mut collector, _) = start(transport);
    connect(&mut collector, 1);
    let started = Instant::now();
    collector.stop();
    assert!(started.elapsed() < Duration::from_secs(1));
}

fn subtypes(snapshot: &CollectorSnapshot) -> Vec<String> {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations
            .iter()
            .map(|row| {
                row.details.as_ref().unwrap()["subtype"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect(),
        _ => Vec::new(),
    }
}
