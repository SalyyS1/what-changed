//! The listener collector against a real database and a real supervisor.
//!
//! The graph evidence it produces is covered in `relationships_process_port.rs`.
//! This file covers the endpoint identity itself — the four tables, both IP
//! versions, the byte-order conversion that silently turns 4000 into a different
//! plausible port when it is wrong — plus the same restart and burst behaviour
//! the process collector has to get right.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod harness;

use std::time::Duration;

use harness::{socket, Rig, ScriptedProcesses, ScriptedSockets, EPOCH_MS};
use what_changed_lib::collectors::listeners;
use what_changed_lib::domain::ids::{self, Protocol};
use what_changed_lib::platform::traits::SocketRow;

const COLLECTOR: &str = listeners::ID;

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
fn a_first_snapshot_records_the_listeners_that_already_exist_and_opens_none() {
    let table = vec![
        socket(Protocol::Tcp, "0.0.0.0", 4_000, 7),
        socket(Protocol::Udp, "::", 53, 9),
    ];
    // Constructed with the table already in place, so the establishing cycle
    // happens during `Rig::with` and this asserts what it did.
    let sockets = ScriptedSockets::new(vec![table]);
    let rig = Rig::with(
        ScriptedProcesses::new(vec![Vec::new()]),
        sockets,
        "listeners-establish",
    );

    assert_eq!(
        rig.events_from(COLLECTOR).len(),
        0,
        "a socket that was already bound did not just open"
    );
    let stored = rig
        .baseline(COLLECTOR, listeners::BASELINE_KIND)
        .expect("the establishing cycle persisted a baseline");
    assert_eq!(stored["acknowledged"].as_object().unwrap().len(), 2);
    assert_eq!(stored["established"], true);
    assert_nothing_was_lost(&rig);

    // And the next cycle, reading the same table, still finds nothing to say.
    rig.cycle();
    assert_eq!(rig.events_from(COLLECTOR).len(), 0);
}

#[test]
fn every_transport_and_ip_version_is_a_distinct_endpoint() {
    let (rig, _processes, sockets) = Rig::scripted("listeners-families");
    sockets.script(vec![vec![
        socket(Protocol::Tcp, "0.0.0.0", 4_000, 7),
        socket(Protocol::Tcp, "::", 4_000, 7),
        socket(Protocol::Udp, "0.0.0.0", 4_000, 7),
        socket(Protocol::Udp, "::", 4_000, 7),
        socket(Protocol::Tcp, "127.0.0.1", 4_000, 7),
    ]]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(events.len(), 5, "five endpoints, not one port number");
    let ids: Vec<&str> = events
        .iter()
        .map(|event| event.entity_id.as_str())
        .collect();
    for expected in [
        ids::port(Protocol::Tcp, "0.0.0.0", 4_000),
        ids::port(Protocol::Tcp, "::", 4_000),
        ids::port(Protocol::Udp, "0.0.0.0", 4_000),
        ids::port(Protocol::Udp, "::", 4_000),
        ids::port(Protocol::Tcp, "127.0.0.1", 4_000),
    ] {
        assert!(
            ids.contains(&expected.as_str()),
            "{expected} is missing from {ids:?}"
        );
    }
    assert_eq!(
        ids[0],
        ids::port(Protocol::Tcp, "0.0.0.0", 4_000),
        "the grammar is written in exactly one place"
    );
    assert_nothing_was_lost(&rig);
}

#[test]
fn a_link_local_endpoint_keeps_its_interface_scope_in_its_identity() {
    let (rig, _processes, sockets) = Rig::scripted("listeners-scope");
    sockets.script(vec![vec![
        socket(Protocol::Tcp, "fe80::1%12", 4_000, 7),
        socket(Protocol::Tcp, "fe80::1%15", 4_000, 7),
    ]]);
    rig.cycle();

    let events = rig.events_from(COLLECTOR);
    assert_eq!(
        events.len(),
        2,
        "the same address on two interfaces is two listeners, not one changing owner"
    );
    assert_nothing_was_lost(&rig);
}

#[test]
fn a_burst_beyond_one_commit_lands_whole_over_several_cycles() {
    const BURST: u16 = 500;
    let (rig, _processes, sockets) = Rig::scripted("listeners-burst");
    let table: Vec<SocketRow> = (0..BURST)
        .map(|index| socket(Protocol::Tcp, "0.0.0.0", 1_000 + index, 7))
        .collect();
    sockets.script(vec![table]);

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
    assert_eq!(rig.event_count(), u64::from(BURST));
}

#[test]
fn a_listener_found_across_a_restart_is_recorded_at_its_detection_time() {
    let directory = {
        let (rig, _processes, sockets) = Rig::scripted("listeners-restart");
        sockets.script(vec![vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 7)]]);
        rig.cycle();
        assert_eq!(rig.event_count(), 1);
        let directory = rig.directory.clone();
        rig.stop();
        directory
    };

    // Ten minutes later the old socket is still bound and a new one appeared
    // while nothing was watching.
    let sockets = ScriptedSockets::new(vec![vec![
        socket(Protocol::Tcp, "0.0.0.0", 4_000, 7),
        socket(Protocol::Tcp, "0.0.0.0", 5_000, 8),
    ]]);
    let resumed = Rig::at_elapsed(
        directory,
        ScriptedProcesses::new(vec![Vec::new()]),
        sockets.clone(),
        Duration::from_secs(600),
    );

    let events = resumed.events_from(COLLECTOR);
    assert_eq!(
        events.len(),
        2,
        "one historical open plus one new open; the old socket did not reopen"
    );
    assert_eq!(events[1].entity_id, "port:tcp:0.0.0.0:5000");
    assert!(events[1].occurred_at_is_uncertain);
    assert_eq!(events[1].timestamp, EPOCH_MS + 600_000);
    assert_nothing_was_lost(&resumed);

    sockets.script(vec![vec![
        socket(Protocol::Tcp, "0.0.0.0", 4_000, 7),
        socket(Protocol::Tcp, "0.0.0.0", 5_000, 8),
        socket(Protocol::Tcp, "0.0.0.0", 6_000, 9),
    ]]);
    resumed.cycle();
    let events = resumed.events_from(COLLECTOR);
    assert_eq!(events.len(), 3);
    assert!(!events[2].occurred_at_is_uncertain);
}

#[test]
fn a_quiet_machine_costs_no_writes_at_all() {
    let (rig, _processes, sockets) = Rig::scripted("listeners-quiet");
    sockets.script(vec![vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 7)]]);
    rig.cycle();
    assert_eq!(rig.event_count(), 1);
    let baseline = rig.baseline(COLLECTOR, listeners::BASELINE_KIND);

    // Thirty cycles with nothing changing must not rewrite the baseline once,
    // which is the difference between a recorder and a disk-warming loop.
    for _ in 0..30 {
        rig.cycle();
    }
    assert_eq!(rig.event_count(), 1);
    assert_eq!(
        rig.baseline(COLLECTOR, listeners::BASELINE_KIND),
        baseline,
        "a quiet cycle leaves the durable state alone"
    );
    assert_eq!(rig.status(COLLECTOR).events_emitted, 1);
    assert_nothing_was_lost(&rig);
}

/// Reading the real tables, which is the only way to check the byte-order and
/// address conversions against an operating system rather than against a fixture
/// that was written from the same misunderstanding.
#[cfg(windows)]
mod real_machine {
    use what_changed_lib::domain::ids::Protocol;
    use what_changed_lib::platform::traits::SocketProvider;
    use what_changed_lib::platform::windows::sockets::WindowsSocketProvider;

    #[test]
    fn the_real_tables_report_plausible_endpoints() {
        let rows = WindowsSocketProvider.listeners().unwrap();
        assert!(
            rows.len() > 10,
            "a running Windows machine has more than ten bound endpoints"
        );
        let mut seen_tcp = false;
        let mut seen_udp = false;
        let mut seen_v6 = false;
        for row in &rows {
            seen_tcp |= row.protocol == Protocol::Tcp;
            seen_udp |= row.protocol == Protocol::Udp;
            seen_v6 |= row.address.contains(':');
            assert!(row.owning_pid > 0, "every row names an owner");
            // A port that came back byte-swapped would still be a valid number,
            // so the check is on the shape of the address instead: a conversion
            // bug in the address shows up as a dotted quad with an impossible
            // first octet or as text nobody wrote.
            assert!(
                row.address.parse::<std::net::IpAddr>().is_ok() || row.address.contains('%'),
                "{} is not an address this build can explain",
                row.address
            );
        }
        assert!(seen_tcp && seen_udp, "both transports were read");
        assert!(seen_v6, "the IPv6 tables were read too");

        // Windows may expose multiple reusable UDP sockets with the exact same
        // endpoint and owner pid. Folding those rows belongs to the collector's
        // SocketIndex and is covered by deterministic fixture tests; a live
        // provider test must not reinterpret legitimate kernel multiplicity as
        // corruption.
    }
}
