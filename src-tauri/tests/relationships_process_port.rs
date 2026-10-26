//! The process-to-port graph edge, end to end.
//!
//! This is the relationship the analysis engine walks to reach its strongest
//! confidence class, so it is asserted the way that engine will read it: from
//! both directions, carrying the event that proved it, after the whole pipeline
//! has run — provider, collector, shared process map, normalizer, sanitizer, one
//! SQLite transaction, and a read back out of the database.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod harness;

use std::time::Duration;

use harness::{opaque_process, process, socket, Rig, ScriptedProcesses, ScriptedSockets};
use what_changed_lib::collectors::listeners;
use what_changed_lib::domain::ids::{self, Protocol};
use what_changed_lib::domain::RelationshipKind;

const API_PID: u32 = 4_242;
const OTHER_PID: u32 = 7;
const PORT: u16 = 4_000;

fn port_id() -> String {
    ids::port(Protocol::Tcp, "0.0.0.0", PORT)
}

fn api_id() -> String {
    ids::process(r"C:\dev\api.exe")
}

/// Run one process cycle, then one listener cycle against a scripted table.
fn open_port(rig: &Rig, processes: &ScriptedProcesses, sockets: &ScriptedSockets) {
    processes.script(vec![vec![process(API_PID, "api.exe", 10)]]);
    rig.cycle();
    sockets.script(vec![vec![socket(Protocol::Tcp, "0.0.0.0", PORT, API_PID)]]);
    rig.cycle();
}

#[test]
fn an_opened_port_is_linked_to_its_process_in_both_directions() {
    let (rig, processes, sockets) = Rig::scripted("relationships-both-directions");
    open_port(&rig, &processes, &sockets);

    let opened = rig.events_from(listeners::ID);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].event_type, "opened");
    assert_eq!(opened[0].category, "network");
    assert_eq!(opened[0].entity_id, port_id());
    assert_eq!(opened[0].confidence, "high");
    // The worked example's own wording, produced from the resolved owner rather
    // than asserted about it.
    assert_eq!(
        opened[0].user_friendly_summary,
        "Port 4000 opened by: api.exe"
    );

    let edges = rig.relationships();
    assert_eq!(edges.len(), 2, "one proven fact, both directions of it");
    let evidence = opened[0].id;
    assert!(
        edges.iter().any(|edge| edge.source_id == api_id()
            && edge.target_id == port_id()
            && edge.kind == RelationshipKind::ListensOn
            && edge.event_id == Some(evidence)
            && edge.recorded_at == opened[0].timestamp),
        "process -> listens_on -> port: {edges:?}"
    );
    assert!(
        edges.iter().any(|edge| edge.source_id == port_id()
            && edge.target_id == api_id()
            && edge.kind == RelationshipKind::OpenedBy
            && edge.event_id == Some(evidence)),
        "port -> opened_by -> process: {edges:?}"
    );
}

#[test]
fn relationship_evidence_preserves_the_worked_examples_three_hundred_milliseconds() {
    let (rig, processes, sockets) = Rig::scripted("relationships-300ms");
    processes.script(vec![vec![process(API_PID, "api.exe", 10)]]);
    rig.cycle();
    let started = rig.events_from("process");
    assert_eq!(started.len(), 1);

    sockets.script(vec![vec![socket(Protocol::Tcp, "0.0.0.0", PORT, API_PID)]]);
    rig.advance(Duration::from_millis(300));

    let opened = rig.events_from(listeners::ID);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].timestamp - started[0].timestamp, 300);
    assert!(rig.relationships().iter().all(|edge| {
        edge.event_id == Some(opened[0].id) && edge.recorded_at == opened[0].timestamp
    }));
}

#[test]
fn the_graph_walk_finds_the_edge_from_either_end() {
    let (rig, processes, sockets) = Rig::scripted("relationships-walk");
    open_port(&rig, &processes, &sockets);

    // What P15 and the System Map actually call: a walk from one entity,
    // answered by the repository rather than by a query this test wrote.
    let from_process = rig.relationships_from(&api_id());
    assert_eq!(from_process.len(), 1);
    assert_eq!(from_process[0].kind, RelationshipKind::ListensOn);
    assert_eq!(from_process[0].target_id, port_id());

    let to_process = rig.relationships_to(&api_id());
    assert_eq!(to_process.len(), 1);
    assert_eq!(to_process[0].kind, RelationshipKind::OpenedBy);
    assert_eq!(to_process[0].source_id, port_id());

    assert_eq!(
        rig.relationships_from(&port_id())[0].kind,
        RelationshipKind::OpenedBy
    );
    assert_eq!(
        rig.relationships_to(&port_id())[0].kind,
        RelationshipKind::ListensOn
    );
}

#[test]
fn both_endpoints_are_stored_as_entities_so_a_relationship_cannot_dangle() {
    let (rig, processes, sockets) = Rig::scripted("relationships-entities");
    open_port(&rig, &processes, &sockets);

    let port = rig.entity(&port_id()).expect("the port entity is stored");
    assert_eq!(port.display_name, "TCP 0.0.0.0:4000");
    assert_eq!(port.entity_type.as_str(), "port");

    let api = rig.entity(&api_id()).expect("the process entity is stored");
    assert_eq!(api.display_name, "api.exe");
    assert_eq!(api.entity_type.as_str(), "process");
    // The process collector also wrote its own event about the same entity, and
    // both collectors agree on the id — which is the point of hashing the
    // executable in exactly one place.
    assert_eq!(api.id, api_id());
}

#[test]
fn a_port_whose_owner_nobody_identified_still_records_but_proves_no_edge() {
    let (rig, _processes, sockets) = Rig::scripted("relationships-unresolved");
    // The process collector is never given this pid, so the shared map cannot
    // name it. Guessing an owner from the socket table alone would be exactly
    // the unproven relationship the engine is not allowed to trust.
    sockets.script(vec![vec![socket(
        Protocol::Tcp,
        "0.0.0.0",
        PORT,
        OTHER_PID,
    )]]);
    rig.cycle();

    let opened = rig.events_from(listeners::ID);
    assert_eq!(opened.len(), 1, "the port really is listening");
    assert_eq!(opened[0].user_friendly_summary, "Port 4000 opened");
    assert_eq!(opened[0].confidence, "low");
    assert_eq!(opened[0].metadata["owner_unresolved"], true);
    assert!(rig.relationships().is_empty());
}

#[test]
fn a_low_confidence_owner_is_named_but_proves_no_graph_edge() {
    let (rig, processes, sockets) = Rig::scripted("relationships-low-confidence");
    processes.script(vec![vec![opaque_process(API_PID, "api.exe", 10)]]);
    rig.cycle();
    sockets.script(vec![vec![socket(Protocol::Tcp, "0.0.0.0", PORT, API_PID)]]);
    rig.cycle();

    let opened = rig.events_from(listeners::ID);
    assert_eq!(opened.len(), 1);
    assert_eq!(
        opened[0].user_friendly_summary,
        "Port 4000 opened by: api.exe"
    );
    assert_eq!(opened[0].confidence, "low");
    assert_eq!(opened[0].metadata["owner_unresolved"], false);
    assert!(
        rig.relationships().is_empty(),
        "a graph row has no confidence column, so a low-confidence owner cannot become a confirmed edge"
    );
}

#[test]
fn an_owner_change_repoints_the_edge_instead_of_fabricating_a_close_and_an_open() {
    let (rig, processes, sockets) = Rig::scripted("relationships-reowned");
    open_port(&rig, &processes, &sockets);
    assert_eq!(rig.events_from(listeners::ID).len(), 1);

    processes.script(vec![vec![
        process(API_PID, "api.exe", 10),
        process(OTHER_PID, "other.exe", 20),
    ]]);
    rig.cycle();
    sockets.script(vec![vec![socket(
        Protocol::Tcp,
        "0.0.0.0",
        PORT,
        OTHER_PID,
    )]]);
    rig.cycle();

    let events = rig.events_from(listeners::ID);
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "modified");
    assert_eq!(
        events[1].user_friendly_summary,
        "Port 4000 changed owner: other.exe"
    );
    assert_eq!(events[1].metadata["previous_owning_pids"][0], API_PID);
    assert_eq!(events[1].metadata["owning_pids"][0], OTHER_PID);

    // The endpoint stayed bound throughout, so no close or open was invented.
    assert!(events.iter().all(|event| event.event_type != "closed"));

    // Storage keeps one row per (source, target, kind), so the new owner's edges
    // exist beside the old ones and each names the event that proved it.
    let edges = rig.relationships();
    assert_eq!(edges.len(), 4);
    let other_id = ids::process(r"C:\dev\other.exe");
    assert!(edges.iter().any(|edge| edge.source_id == other_id
        && edge.target_id == port_id()
        && edge.kind == RelationshipKind::ListensOn
        && edge.event_id == Some(events[1].id)));
    assert!(edges.iter().any(|edge| edge.source_id == api_id()
        && edge.kind == RelationshipKind::ListensOn
        && edge.event_id == Some(events[0].id)));
}

#[test]
fn a_closed_port_keeps_the_edge_that_proved_it_opened() {
    let (rig, processes, sockets) = Rig::scripted("relationships-closed");
    open_port(&rig, &processes, &sockets);
    let opened = rig.events_from(listeners::ID)[0].clone();

    sockets.script(vec![Vec::new()]);
    rig.cycle();

    let events = rig.events_from(listeners::ID);
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "closed");
    assert_eq!(
        events[1].user_friendly_summary,
        "Port 4000 stopped listening"
    );
    assert_eq!(events[1].metadata["owning_pids"][0], API_PID);

    // The edge outlives the open event and still names it, which is what lets an
    // investigation say "port 4000 stopped being listened on by api.exe" after
    // the fact rather than only while it was true.
    let edges = rig.relationships();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|edge| edge.event_id == Some(opened.id)));
    assert_eq!(rig.relationships_from(&api_id()).len(), 1);
}

#[test]
fn one_endpoint_with_two_owners_is_one_fact_naming_both() {
    let (rig, processes, sockets) = Rig::scripted("relationships-shared-endpoint");
    processes.script(vec![vec![
        process(API_PID, "api.exe", 10),
        process(OTHER_PID, "other.exe", 20),
    ]]);
    rig.cycle();
    sockets.script(vec![vec![
        socket(Protocol::Tcp, "0.0.0.0", PORT, API_PID),
        socket(Protocol::Tcp, "0.0.0.0", PORT, OTHER_PID),
    ]]);
    rig.cycle();

    let events = rig.events_from(listeners::ID);
    assert_eq!(
        events.len(),
        1,
        "two rows for one endpoint would be two listeners"
    );
    assert_eq!(
        events[0].user_friendly_summary,
        "Port 4000 opened by: api.exe, other.exe"
    );
    assert_eq!(
        events[0].metadata["owning_pids"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        rig.relationships().len(),
        4,
        "both directions for each owner"
    );
}

#[test]
fn two_pids_of_one_executable_are_one_entity_and_not_a_duplicated_edge() {
    let (rig, processes, sockets) = Rig::scripted("relationships-same-image");
    // Storage refuses an observation that assigns one edge twice, so the
    // deduplication is a correctness requirement rather than a tidy-up.
    processes.script(vec![vec![
        process(API_PID, "api.exe", 10),
        process(OTHER_PID, "api.exe", 20),
    ]]);
    rig.cycle();
    sockets.script(vec![vec![
        socket(Protocol::Tcp, "0.0.0.0", PORT, API_PID),
        socket(Protocol::Tcp, "0.0.0.0", PORT, OTHER_PID),
    ]]);
    rig.cycle();

    let events = rig.events_from(listeners::ID);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].confidence, "high");
    assert_eq!(
        events[0].user_friendly_summary,
        "Port 4000 opened by: api.exe"
    );
    assert_eq!(rig.relationships().len(), 2);
    assert_eq!(
        rig.status(listeners::ID).failed_cycles,
        0,
        "a duplicate edge would have failed the whole commit"
    );
}
