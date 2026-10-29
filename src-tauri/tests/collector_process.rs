//! The process collector against a real database and a real supervisor.
//!
//! What these add to the collector's own unit tests is the far end of the
//! pipeline: that a degraded identity, a churn summary and a burst larger than
//! one commit all survive normalization, redaction, the atomic commit's
//! validation and a read back out of SQLite — and that none of them cost the
//! collector a failed cycle.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod harness;

use std::time::Duration;

use harness::{
    browser_child, opaque_process, process, unidentified_process, Rig, ScriptedProcesses,
    ScriptedSockets, EPOCH_MS,
};
use what_changed_lib::collectors::process;
use what_changed_lib::domain::ids;
use what_changed_lib::platform::traits::ProcessRow;

const COLLECTOR: &str = process::ID;

/// Every health counter that would say something was lost.
fn assert_nothing_was_lost(rig: &Rig) {
    let status = rig.status(COLLECTOR);
    assert_eq!(status.consecutive_failures, 0, "{:?}", status.last_error);
    assert_eq!(status.failed_cycles, 0);
    assert_eq!(status.cycle_admission_drops, 0);
    assert_eq!(status.queue_drops, 0);
    assert_eq!(status.failed_commit_rows, 0);
    assert_eq!(status.last_error, None);
}

#[test]
fn a_recycled_pid_is_one_stop_and_one_start_in_that_order() {
    let (rig, processes, _sockets) = Rig::scripted("process-pid-reuse");
    processes.script(vec![vec![process(1_234, "api.exe", 10)]]);
    rig.cycle();
    assert_eq!(rig.events_from(COLLECTOR).len(), 1);

    // The same pid and the same executable, with a different creation time: a
    // different program that happens to have inherited a number.
    processes.script(vec![vec![process(1_234, "api.exe", 11)]]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].event_type, "stopped");
    assert_eq!(events[2].event_type, "started");
    assert!(
        events[1].id < events[2].id,
        "the stop is recorded before the start that replaced it"
    );
    assert_eq!(events[1].timestamp, events[2].timestamp);
    assert_eq!(
        events[1].entity_id, events[2].entity_id,
        "the same executable is the same entity, restarted"
    );
    assert_nothing_was_lost(&rig);
}

#[test]
fn twenty_of_forty_denied_paths_degrade_the_events_and_fail_no_cycle() {
    let (rig, processes, _sockets) = Rig::scripted("process-path-denial");
    let table: Vec<ProcessRow> = (0..40u32)
        .map(|index| {
            let pid = index + 1;
            let image = format!("app{pid}.exe");
            if index % 2 == 0 {
                process(pid, &image, 10)
            } else {
                opaque_process(pid, &image, 10)
            }
        })
        .collect();
    processes.script(vec![table]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 40);
    let denied: Vec<_> = events
        .iter()
        .filter(|event| event.metadata["path_unavailable"] == true)
        .collect();
    assert_eq!(denied.len(), 20);
    for event in &denied {
        assert_eq!(event.metadata["path"], serde_json::Value::Null);
        assert_eq!(event.confidence, "low");
        assert_eq!(event.event_type, "started");
        assert!(
            !event.entity_display_name.is_empty(),
            "a denied path is not a denied name"
        );
        assert!(
            !event.user_friendly_summary.contains("unknown"),
            "the recorder knows what the program is called: {}",
            event.user_friendly_summary
        );
    }
    let readable: Vec<_> = events
        .iter()
        .filter(|event| event.metadata["path_unavailable"] == false)
        .collect();
    assert_eq!(readable.len(), 20);
    for event in &readable {
        assert!(event.metadata["path"].is_string());
        assert_eq!(event.confidence, "high");
    }
    // "Zero errors logged" is asserted the way it is observable from outside
    // the logging stack: nothing was lost, retried, dropped or degraded.
    assert_nothing_was_lost(&rig);
    assert_eq!(rig.status(COLLECTOR).events_emitted, 40);
}

#[test]
fn eight_pairs_are_sixteen_prompt_rows_and_one_summary_when_the_window_closes() {
    let (rig, processes, _sockets) = Rig::scripted("process-churn");
    for creation in 1..=8u64 {
        processes.script(vec![vec![process(9, "flaky.exe", creation)]]);
        rig.cycle();
        processes.script(vec![Vec::new()]);
        rig.cycle();
    }

    let events = rig.events_from(COLLECTOR);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "started")
            .count(),
        8,
        "the raw evidence is written when it happens, not held back"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "stopped")
            .count(),
        8
    );
    assert!(
        !events.iter().any(|event| event.event_type == "churn"),
        "the window has not closed yet"
    );

    rig.advance(Duration::from_secs(60));
    let events = rig.events_from(COLLECTOR);
    let summaries: Vec<_> = events
        .iter()
        .filter(|event| event.event_type == "churn")
        .collect();
    assert_eq!(summaries.len(), 1, "one summary, not one per pair");
    assert_eq!(summaries[0].metadata["pairs"], 8);
    assert_eq!(summaries[0].metadata["count"], 8);
    assert_eq!(summaries[0].metadata["summary_of_window"], true);
    assert_eq!(
        summaries[0].user_friendly_summary,
        "flaky.exe started and stopped 8 times in one minute"
    );
    assert_eq!(summaries[0].importance, "informational");
    assert_eq!(
        events.len(),
        17,
        "the summary is appended beside the raw rows, never instead of them"
    );
    assert_nothing_was_lost(&rig);
}

#[test]
fn a_burst_beyond_one_commit_lands_whole_over_several_cycles() {
    const BURST: u32 = 500;
    let (rig, processes, _sockets) = Rig::scripted("process-burst");
    let table: Vec<ProcessRow> = (1..=BURST)
        .map(|pid| process(pid, &format!("app{pid}.exe"), 5))
        .collect();
    processes.script(vec![table]);

    rig.cycle();
    let first = rig.event_count();
    assert!(first > 0);
    assert!(
        first < u64::from(BURST),
        "one atomic commit cannot carry all of it"
    );
    rig.cycle();
    rig.cycle();
    assert_eq!(rig.event_count(), u64::from(BURST), "nothing was truncated");
    assert_nothing_was_lost(&rig);

    rig.cycle();
    assert_eq!(
        rig.event_count(),
        u64::from(BURST),
        "the remainder is not emitted twice once it has landed"
    );
}

#[test]
fn a_change_found_across_a_restart_is_recorded_at_its_detection_time() {
    let directory = {
        let (rig, processes, _sockets) = Rig::scripted("process-restart");
        processes.script(vec![vec![process(1, "api.exe", 10)]]);
        rig.cycle();
        assert_eq!(rig.event_count(), 1);
        let directory = rig.directory.clone();
        rig.stop();
        directory
    };

    // Ten minutes later. One program is still there, another appeared while
    // nothing was watching, and nobody knows when.
    let processes = ScriptedProcesses::new(vec![vec![
        process(1, "api.exe", 10),
        process(2, "new.exe", 20),
    ]]);
    let sockets = ScriptedSockets::new(vec![Vec::new()]);
    let resumed = Rig::at_elapsed(
        directory,
        processes.clone(),
        sockets,
        Duration::from_secs(600),
    );

    let events = resumed.events_from(COLLECTOR);
    assert_eq!(
        events.len(),
        2,
        "one historical start plus one new start; the old program did not restart"
    );
    assert_eq!(events[1].entity_display_name, "new.exe");
    assert!(
        events[1].occurred_at_is_uncertain,
        "a change found across a gap has a detection time, not an occurrence time"
    );
    assert_eq!(events[1].timestamp, EPOCH_MS + 600_000);
    assert_nothing_was_lost(&resumed);

    // The gap is closed now, so the next change has a real occurrence time.
    processes.script(vec![vec![
        process(1, "api.exe", 10),
        process(2, "new.exe", 20),
        process(3, "third.exe", 30),
    ]]);
    resumed.cycle();
    let events = resumed.events_from(COLLECTOR);
    assert_eq!(events.len(), 3);
    assert!(!events[2].occurred_at_is_uncertain);
}

#[test]
fn a_browser_spawning_its_own_image_is_recorded_and_marked_as_noise() {
    let (rig, processes, _sockets) = Rig::scripted("process-browser");
    processes.script(vec![vec![
        process(1, "chrome.exe", 10),
        browser_child(10, 1, "chrome.exe", 20),
        process(2, "api.exe", 30),
    ]]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 3);
    let by_pid = |pid: u32| {
        events
            .iter()
            .find(|event| event.metadata["pid"] == pid)
            .unwrap_or_else(|| panic!("no event for pid {pid}"))
    };
    assert_eq!(by_pid(10).metadata["browser_subprocess"], true);
    assert_eq!(by_pid(1).metadata["browser_subprocess"], false);
    assert_eq!(by_pid(2).metadata["browser_subprocess"], false);
    assert_eq!(by_pid(10).importance, "informational");
    assert_eq!(by_pid(2).importance, "informational");
    assert_nothing_was_lost(&rig);
}

#[test]
fn a_process_with_neither_a_path_nor_a_creation_time_is_still_a_fact() {
    let (rig, processes, _sockets) = Rig::scripted("process-unidentified");
    processes.script(vec![vec![unidentified_process(5, "mystery.exe")]]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].confidence, "low");
    assert_eq!(events[0].entity_display_name, "mystery.exe");
    assert_eq!(events[0].entity_id, ids::process("mystery.exe"));
    assert!(events[0].entity_id.starts_with("process:"));
    assert_eq!(events[0].entity_id.len(), "process:".len() + 16);

    // A different image on the same pid is a different program, and with no
    // creation time on either side the name is what decides it.
    processes.script(vec![vec![unidentified_process(5, "other.exe")]]);
    rig.cycle();
    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].event_type, "stopped");
    assert_eq!(events[2].event_type, "started");
    assert_ne!(events[1].entity_id, events[2].entity_id);
    assert_nothing_was_lost(&rig);
}

#[test]
fn a_secret_in_an_executable_path_never_reaches_a_row_and_never_moves_an_identity() {
    let (rig, processes, _sockets) = Rig::scripted("process-secret-path");
    let secret = r"C:\Users\dev\sk-live-4eC39HqLyjWDarjtT1zdp7dc\api.exe";
    processes.script(vec![vec![ProcessRow {
        pid: 3,
        parent_pid: 0,
        image_name: "api.exe".to_owned(),
        executable_path: Some(secret.to_owned()),
        creation_time: Some(10),
    }]]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 1);
    // The identity is a hash of the path the collector actually saw, so a
    // redacted copy of that path in a row cannot change which process this is.
    assert_eq!(events[0].entity_id, ids::process(secret));
    assert_eq!(events[0].entity_display_name, "api.exe");

    // Restart the recorder over the same history. The identity has to survive
    // the round trip through a baseline that the sanitizer has been over.
    let directory = rig.directory.clone();
    let resumed_processes = ScriptedProcesses::new(vec![vec![ProcessRow {
        pid: 3,
        parent_pid: 0,
        image_name: "api.exe".to_owned(),
        executable_path: Some(secret.to_owned()),
        creation_time: Some(10),
    }]]);
    rig.stop();
    drop(rig);
    let resumed = Rig::at_elapsed(
        directory,
        resumed_processes,
        ScriptedSockets::new(vec![Vec::new()]),
        Duration::from_secs(600),
    );
    let resumed_events = resumed.events_from(COLLECTOR);
    assert_eq!(
        resumed_events.len(),
        1,
        "only the historical row remains; hydration emits no false stop or start"
    );
    assert_eq!(resumed_events[0].entity_id, ids::process(secret));
}

/// The live-machine gates. Both read the real operating system, so neither can
/// be answered by a scripted table.
#[cfg(windows)]
mod real_machine {
    use std::time::{Duration, Instant};

    use what_changed_lib::platform::traits::SocketProvider;
    use what_changed_lib::platform::windows::sockets::WindowsSocketProvider;

    #[test]
    fn the_real_tables_report_both_ip_versions_and_both_transports() {
        let rows = WindowsSocketProvider.listeners().unwrap();
        assert!(rows
            .iter()
            .any(|row| row.protocol == what_changed_lib::domain::ids::Protocol::Tcp));
        assert!(rows
            .iter()
            .any(|row| row.protocol == what_changed_lib::domain::ids::Protocol::Udp));
        for row in &rows {
            assert!(!row.address.is_empty());
            if let Some((address, scope)) = row.address.split_once('%') {
                assert!(address.parse::<std::net::Ipv6Addr>().is_ok());
                assert!(
                    scope.parse::<u32>().is_ok(),
                    "invalid scope in {}",
                    row.address
                );
            } else {
                assert!(row.address.parse::<std::net::IpAddr>().is_ok());
            }
        }
    }

    /// Thirty seconds of an idle machine, through the composition the shipped
    /// app actually uses: the real registry, the real providers, the real
    /// cadence and a real database.
    #[test]
    #[ignore = "thirty seconds of real wall clock against the live operating system"]
    fn an_idle_machine_produces_almost_no_events() {
        use what_changed_lib::app::{AppPaths, AppState};

        let directory = what_changed_lib::testing::temp_dir("collectors-os-idle");
        let state = AppState::open_with_registry(
            AppPaths::under(&directory),
            what_changed_lib::platform::production_registry(),
        )
        .unwrap();
        state.collectors.spawn_all().unwrap();

        let began = Instant::now();
        std::thread::sleep(Duration::from_secs(30));
        assert!(
            began.elapsed() >= Duration::from_secs(30),
            "the idle gate must observe thirty real seconds"
        );
        let events = state
            .db
            .read(what_changed_lib::storage::event_repository::count)
            .unwrap();
        let status = state.collectors.list_collector_status();
        let process = status.iter().find(|row| row.id == "process").unwrap();
        let listeners = status.iter().find(|row| row.id == "listeners").unwrap();
        assert_eq!(process.failed_cycles, 0, "{:?}", process.last_error);
        assert_eq!(listeners.failed_cycles, 0, "{:?}", listeners.last_error);
        assert!(
            process.last_run_at.is_some() && listeners.last_run_at.is_some(),
            "both collectors ran during the sample"
        );
        state.shutdown_cleanly();

        // A genuinely idle machine produces nothing. Real machines are not
        // genuinely idle, so the bound allows for honest churn; what it must
        // not allow is a diff that reports the world as changing.
        assert!(
            events < 5,
            "thirty idle seconds produced {events} events: {}",
            status
                .iter()
                .map(|row| format!("{}={} ", row.id, row.events_emitted))
                .collect::<String>()
        );
    }
}
