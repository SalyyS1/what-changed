//! Turning one listener difference into an event and the graph evidence for it.
//!
//! The port event is the evidence, and both directions of the process-to-port
//! edge are attached to it, so the analysis engine can walk from "this process
//! restarted" to "that port stopped listening" and back without guessing which
//! direction the relationship was recorded in.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::collectors::listeners::diff::{Change, TrackedSocket};
use crate::collectors::listeners::ID;
use crate::collectors::process_map::{ProcessIdentity, ProcessMap};
use crate::collectors::stateful::CommitCapacity;
use crate::collectors::{
    EntityIdentity, ObservationKind, RawEntity, RawObservation, RawRelationshipSpec,
};
use crate::domain::{
    Category, ClassifierContext, Confidence, EventType, RelationshipKind, Severity,
};
use crate::error::{AppError, AppResult};

/// One change, priced: its event, the graph evidence its owners support, and
/// how much of a commit that evidence costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The port event.
    pub observation: RawObservation,
    /// Both directions of every owner edge, bound to the event above.
    pub relationships: Vec<RawRelationshipSpec>,
    /// Process facts the edges need stored beside the event.
    pub related_entities: usize,
}

/// Price as many leading changes as one atomic commit can carry.
///
/// A change is admitted whole or not at all, so a burst of new listeners arrives
/// over several cycles rather than being truncated or retried at a size storage
/// has already refused. The bound that bites first is usually the relationship
/// count, since each resolved owner costs two edges.
pub fn plan(
    changes: &[Change],
    map: &ProcessMap,
    now_ms: i64,
    uncertain: bool,
) -> AppResult<Vec<Planned>> {
    plan_with_identities(changes, &map.snapshot(), now_ms, uncertain)
}

/// Price changes against the process identities captured with their provider
/// target. A deferred port change must not resolve the same pid again on a
/// later cycle, after Windows may have reused it for another process.
pub fn plan_with_identities(
    changes: &[Change],
    identities: &BTreeMap<u32, ProcessIdentity>,
    now_ms: i64,
    uncertain: bool,
) -> AppResult<Vec<Planned>> {
    let mut capacity = CommitCapacity::full();
    let mut planned = Vec::new();
    for change in changes {
        let owners = resolve_owners(change.evidence_owners(), identities);
        let relationships = owners.confirmed.len().saturating_mul(2);
        if !capacity.spend(1, relationships, owners.confirmed.len()) {
            if planned.is_empty() {
                return Err(AppError::InvalidInput {
                    what: "listener transition",
                    reason: "one endpoint carries more graph evidence than an atomic commit"
                        .to_owned(),
                });
            }
            break;
        }
        let evidence_observation = planned.len();
        planned.push(Planned {
            observation: observation(
                change,
                &owners.named,
                owners.all_raw_pids_resolved,
                now_ms,
                uncertain,
            ),
            relationships: edges(change.socket(), &owners.confirmed, evidence_observation),
            related_entities: owners.confirmed.len(),
        });
    }
    Ok(planned)
}

/// The identities behind an owner set, deduplicated by entity.
///
/// Two pids running the same executable are one entity in the graph, and
/// storage refuses an observation that assigns one edge twice — so the
/// deduplication is a correctness requirement rather than a tidy-up.
struct ResolvedOwners {
    /// Every name the event may display, deduplicated by entity.
    named: Vec<ProcessIdentity>,
    /// Identities strong enough to support durable graph evidence.
    confirmed: Vec<ProcessIdentity>,
    /// Whether every pid in the socket table had a corresponding process row.
    all_raw_pids_resolved: bool,
}

fn resolve_owners(
    owners: Option<&BTreeSet<u32>>,
    identities: &BTreeMap<u32, ProcessIdentity>,
) -> ResolvedOwners {
    let Some(owners) = owners else {
        return ResolvedOwners {
            named: Vec::new(),
            confirmed: Vec::new(),
            all_raw_pids_resolved: true,
        };
    };
    let mut named: BTreeMap<String, ProcessIdentity> = BTreeMap::new();
    let mut confirmed: BTreeMap<String, ProcessIdentity> = BTreeMap::new();
    let mut all_raw_pids_resolved = true;
    for pid in owners {
        if let Some(identity) = identities.get(pid).cloned() {
            let id = identity.entity_id.as_str().to_owned();
            if identity.confidence == Confidence::High {
                confirmed
                    .entry(id.clone())
                    .or_insert_with(|| identity.clone());
            }
            named
                .entry(id)
                .and_modify(|existing| {
                    if identity.confidence != Confidence::High {
                        existing.confidence = Confidence::Low;
                    }
                })
                .or_insert(identity);
        } else {
            all_raw_pids_resolved = false;
        }
    }
    ResolvedOwners {
        named: named.into_values().collect(),
        confirmed: confirmed.into_values().collect(),
        all_raw_pids_resolved,
    }
}

/// Both directions of the process-to-port edge for every resolved owner.
fn edges(
    socket: &TrackedSocket,
    owners: &[ProcessIdentity],
    evidence_observation: usize,
) -> Vec<RawRelationshipSpec> {
    let port = port_entity(socket);
    owners
        .iter()
        .flat_map(|owner| {
            let process = process_entity(owner);
            [
                RawRelationshipSpec {
                    evidence_observation,
                    source: process.clone(),
                    target: port.clone(),
                    kind: RelationshipKind::ListensOn,
                },
                RawRelationshipSpec {
                    evidence_observation,
                    source: port.clone(),
                    target: process,
                    kind: RelationshipKind::OpenedBy,
                },
            ]
        })
        .collect()
}

/// How sure the event is of the owner it names.
///
/// An endpoint whose owner could not be resolved still records — the port really
/// is listening — but it says so at low confidence, because the analysis engine
/// must not build its strongest link on an owner nobody identified.
fn confidence(
    change: &Change,
    resolved: &[ProcessIdentity],
    all_raw_pids_resolved: bool,
) -> Confidence {
    let Some(owners) = change.evidence_owners() else {
        return Confidence::High;
    };
    if owners.is_empty() {
        return Confidence::High;
    }
    let every_owner_readable = resolved
        .iter()
        .all(|owner| owner.confidence == Confidence::High);
    if all_raw_pids_resolved && every_owner_readable {
        Confidence::High
    } else {
        Confidence::Low
    }
}

fn observation(
    change: &Change,
    resolved: &[ProcessIdentity],
    all_raw_pids_resolved: bool,
    now_ms: i64,
    uncertain: bool,
) -> RawObservation {
    let socket = change.socket();
    let names = resolved
        .iter()
        .map(|owner| owner.display_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    // A close deliberately carries no owner claim, so an empty resolution there
    // is not an unresolved owner and must not be reported as one.
    let owner_unresolved = change
        .evidence_owners()
        .is_some_and(|owners| !owners.is_empty())
        && !all_raw_pids_resolved;
    let (event_type, user_friendly_summary, technical_summary, details) = match change {
        Change::Opened(_) => (
            EventType::Opened,
            opened_summary(socket.port, &names),
            format!(
                "{} listening, owned by {}",
                socket.display_name(),
                pids(socket)
            ),
            details(socket, owner_unresolved, None),
        ),
        Change::Closed(_) => (
            EventType::Closed,
            format!("Port {} stopped listening", socket.port),
            format!(
                "{} no longer listening, last owned by {}",
                socket.display_name(),
                pids(socket)
            ),
            details(socket, owner_unresolved, None),
        ),
        Change::Reowned { before, .. } => (
            EventType::Modified,
            changed_owner_summary(socket.port, &names),
            format!(
                "{} owner changed from {} to {}",
                socket.display_name(),
                pid_list(&before.owners),
                pids(socket)
            ),
            details(socket, owner_unresolved, Some(&before.owners)),
        ),
    };
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Network,
            event_type,
            severity: Severity::Info,
            confidence: confidence(change, resolved, all_raw_pids_resolved),
            user_friendly_summary,
            technical_summary,
            classifier: ClassifierContext::default(),
        },
        entity: port_entity(socket),
        before: None,
        after: None,
        field: None,
        occurred_at: now_ms,
        details: Some(details),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

/// "Port 4000 opened by: api.exe" — the shape the worked example asks for,
/// produced from the resolved owner rather than asserted about it.
fn opened_summary(port: u16, names: &str) -> String {
    if names.is_empty() {
        format!("Port {port} opened")
    } else {
        format!("Port {port} opened by: {names}")
    }
}

fn changed_owner_summary(port: u16, names: &str) -> String {
    if names.is_empty() {
        format!("Port {port} changed owner")
    } else {
        format!("Port {port} changed owner: {names}")
    }
}

fn details(
    socket: &TrackedSocket,
    owner_unresolved: bool,
    previous: Option<&BTreeSet<u32>>,
) -> serde_json::Value {
    let mut details = json!({
        "protocol": socket.protocol.as_str(),
        "address": socket.address,
        "port": socket.port,
        "owning_pids": socket.owners.iter().copied().collect::<Vec<_>>(),
        // Naming the gap is what stops a low-confidence row from reading like a
        // port with no owner at all.
        "owner_unresolved": owner_unresolved,
    });
    if let Some(previous) = previous {
        details["previous_owning_pids"] = json!(previous.iter().copied().collect::<Vec<_>>());
    }
    details
}

fn pids(socket: &TrackedSocket) -> String {
    pid_list(&socket.owners)
}

fn pid_list(owners: &BTreeSet<u32>) -> String {
    if owners.is_empty() {
        "nothing the table named".to_owned()
    } else {
        owners
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn port_entity(socket: &TrackedSocket) -> RawEntity {
    RawEntity {
        identity: EntityIdentity::Port {
            protocol: socket.protocol,
            address: socket.address.clone(),
            port: socket.port,
        },
        display_name: socket.display_name(),
    }
}

fn process_entity(owner: &ProcessIdentity) -> RawEntity {
    RawEntity {
        identity: EntityIdentity::ProcessById {
            id: owner.entity_id.clone(),
        },
        display_name: owner.display_name.clone(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::collectors::CancelToken;
    use crate::domain::ids::{ProcessId, Protocol};
    use crate::storage::MAX_ATOMIC_RELATIONSHIPS;

    fn socket(port: u16, owners: &[u32]) -> TrackedSocket {
        TrackedSocket {
            protocol: Protocol::Tcp,
            address: "0.0.0.0".to_owned(),
            port,
            owners: owners.iter().copied().collect(),
        }
    }

    fn identity(executable: &str, confidence: Confidence) -> ProcessIdentity {
        ProcessIdentity {
            entity_id: ProcessId::from_executable(executable),
            display_name: executable.to_owned(),
            confidence,
        }
    }

    /// A map that resolves every pid it is given, at high confidence.
    ///
    /// The publish token is dropped at the end of the call and the map keeps its
    /// own clone, which is the point: a live publisher is what makes the map
    /// answer, and a test has no worker to hold one.
    fn resolving_map(pids: &[u32]) -> ProcessMap {
        let map = ProcessMap::new();
        let rows = pids
            .iter()
            .map(|pid| (*pid, identity(&format!("app{pid}.exe"), Confidence::High)))
            .collect();
        map.publish(rows, &CancelToken::default(), Instant::now());
        map
    }

    /// A map holding exactly one identity under several pids.
    fn shared_identity_map(pids: &[u32], executable: &str, confidence: Confidence) -> ProcessMap {
        let map = ProcessMap::new();
        let rows = pids
            .iter()
            .map(|pid| (*pid, identity(executable, confidence)))
            .collect();
        map.publish(rows, &CancelToken::default(), Instant::now());
        map
    }

    #[test]
    fn an_opened_port_names_its_owner_the_way_the_worked_example_asks() {
        let map = resolving_map(&[4_242]);
        let planned = plan(
            &[Change::Opened(socket(4_000, &[4_242]))],
            &map,
            1_000,
            false,
        )
        .unwrap();
        assert_eq!(planned.len(), 1);
        assert_eq!(
            planned[0].observation.kind.user_friendly_summary,
            "Port 4000 opened by: app4242.exe"
        );
        assert_eq!(planned[0].observation.kind.event_type, EventType::Opened);
        assert_eq!(planned[0].observation.kind.category, Category::Network);
        assert_eq!(planned[0].observation.kind.confidence, Confidence::High);
        assert_eq!(
            planned[0].observation.entity.display_name,
            "TCP 0.0.0.0:4000"
        );
        assert!(!planned[0].observation.occurred_at_is_uncertain);
    }

    #[test]
    fn each_resolved_owner_earns_both_directions_of_the_edge() {
        let map = resolving_map(&[7]);
        let planned = plan(&[Change::Opened(socket(4_000, &[7]))], &map, 1_000, false).unwrap();
        assert_eq!(planned[0].relationships.len(), 2);
        assert_eq!(planned[0].related_entities, 1);
        let kinds: Vec<_> = planned[0]
            .relationships
            .iter()
            .map(|edge| edge.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![RelationshipKind::ListensOn, RelationshipKind::OpenedBy]
        );
        assert!(planned[0]
            .relationships
            .iter()
            .all(|edge| edge.evidence_observation == 0));

        let first = &planned[0].relationships[0];
        assert!(matches!(
            first.source.identity,
            EntityIdentity::ProcessById { .. }
        ));
        assert!(matches!(first.target.identity, EntityIdentity::Port { .. }));
        let second = &planned[0].relationships[1];
        assert!(matches!(
            second.source.identity,
            EntityIdentity::Port { .. }
        ));
    }

    #[test]
    fn evidence_indices_point_at_the_event_that_proved_each_edge() {
        let map = resolving_map(&[7, 8]);
        let changes = vec![
            Change::Opened(socket(4_000, &[7])),
            Change::Opened(socket(5_000, &[8])),
        ];
        let planned = plan(&changes, &map, 1_000, false).unwrap();
        assert_eq!(planned.len(), 2);
        for (index, row) in planned.iter().enumerate() {
            assert!(row
                .relationships
                .iter()
                .all(|edge| edge.evidence_observation == index));
        }
    }

    #[test]
    fn an_owner_nobody_identified_still_records_at_low_confidence() {
        let map = ProcessMap::new();
        let planned = plan(
            &[Change::Opened(socket(4_000, &[4_242]))],
            &map,
            1_000,
            false,
        )
        .unwrap();
        assert_eq!(
            planned.len(),
            1,
            "the port is listening whether or not we can name who by"
        );
        assert!(planned[0].relationships.is_empty());
        assert_eq!(planned[0].observation.kind.confidence, Confidence::Low);
        assert_eq!(
            planned[0].observation.kind.user_friendly_summary,
            "Port 4000 opened"
        );
        assert_eq!(
            planned[0].observation.details.as_ref().unwrap()["owner_unresolved"],
            true
        );
    }

    #[test]
    fn two_pids_of_one_executable_are_one_entity_and_one_pair_of_edges() {
        // The same image under two pids resolves to one process id, and
        // storage refuses an observation that assigns one edge twice.
        let map = shared_identity_map(&[7, 9], "shared.exe", Confidence::High);
        let planned = plan(
            &[Change::Opened(socket(4_000, &[7, 9]))],
            &map,
            1_000,
            false,
        )
        .unwrap();
        assert_eq!(planned[0].relationships.len(), 2);
        assert_eq!(planned[0].related_entities, 1);
        assert_eq!(planned[0].observation.kind.confidence, Confidence::High);
        assert_eq!(
            planned[0].observation.kind.user_friendly_summary,
            "Port 4000 opened by: shared.exe"
        );
    }

    #[test]
    fn a_low_confidence_owner_keeps_the_event_low_confidence() {
        let map = shared_identity_map(&[7], "hidden.exe", Confidence::Low);
        let planned = plan(&[Change::Opened(socket(4_000, &[7]))], &map, 1_000, false).unwrap();
        assert_eq!(planned[0].observation.kind.confidence, Confidence::Low);
        assert!(planned[0].relationships.is_empty());
        assert_eq!(planned[0].related_entities, 0);
    }

    #[test]
    fn a_mixed_owner_set_only_persists_the_high_confidence_edge() {
        let map = ProcessMap::new();
        map.publish(
            BTreeMap::from([
                (7, identity("api.exe", Confidence::High)),
                (9, identity("hidden.exe", Confidence::Low)),
            ]),
            &CancelToken::default(),
            Instant::now(),
        );
        let planned = plan(
            &[Change::Opened(socket(4_000, &[7, 9]))],
            &map,
            1_000,
            false,
        )
        .unwrap();
        assert_eq!(planned[0].observation.kind.confidence, Confidence::Low);
        assert_eq!(planned[0].relationships.len(), 2);
        assert_eq!(planned[0].related_entities, 1);
        assert!(planned[0].relationships.iter().all(|edge| {
            matches!(
                &edge.source.identity,
                EntityIdentity::ProcessById { id }
                    if id == &ProcessId::from_executable("api.exe")
            ) || matches!(
                &edge.target.identity,
                EntityIdentity::ProcessById { id }
                    if id == &ProcessId::from_executable("api.exe")
            )
        }));
    }

    #[test]
    fn a_close_names_the_port_and_carries_no_owner_claim() {
        let map = resolving_map(&[7]);
        let planned = plan(&[Change::Closed(socket(4_000, &[7]))], &map, 1_000, false).unwrap();
        assert_eq!(
            planned[0].observation.kind.user_friendly_summary,
            "Port 4000 stopped listening"
        );
        assert_eq!(planned[0].observation.kind.event_type, EventType::Closed);
        assert!(planned[0].relationships.is_empty());
        assert_eq!(
            planned[0].observation.details.as_ref().unwrap()["owning_pids"][0],
            7
        );
    }

    #[test]
    fn a_reowned_endpoint_is_one_modification_naming_both_owner_sets() {
        let map = resolving_map(&[9]);
        let change = Change::Reowned {
            before: socket(4_000, &[7]),
            after: socket(4_000, &[9]),
        };
        let planned = plan(&[change], &map, 1_000, false).unwrap();
        assert_eq!(planned[0].observation.kind.event_type, EventType::Modified);
        assert_eq!(
            planned[0].observation.kind.user_friendly_summary,
            "Port 4000 changed owner: app9.exe"
        );
        let details = planned[0].observation.details.as_ref().unwrap();
        assert_eq!(details["owning_pids"][0], 9);
        assert_eq!(details["previous_owning_pids"][0], 7);
        assert_eq!(planned[0].relationships.len(), 2);
    }

    #[test]
    fn a_downtime_diff_marks_its_timestamps_uncertain() {
        let map = ProcessMap::new();
        let planned = plan(&[Change::Opened(socket(4_000, &[]))], &map, 1_000, true).unwrap();
        assert!(planned[0].observation.occurred_at_is_uncertain);
        assert_eq!(planned[0].observation.occurred_at, 1_000);
    }

    #[test]
    fn a_burst_stops_at_the_relationship_bound_and_leaves_the_rest_for_next_cycle() {
        // Two hundred opened ports, each with one owner and therefore two
        // edges, is exactly the relationship bound. The next one must wait.
        let pids: Vec<u32> = (1..=201).collect();
        let map = resolving_map(&pids);
        let changes: Vec<Change> = (0..201u32)
            .map(|index| Change::Opened(socket(1_000 + index as u16, &[index + 1])))
            .collect();
        let planned = plan(&changes, &map, 1_000, false).unwrap();
        assert_eq!(planned.len(), MAX_ATOMIC_RELATIONSHIPS / 2);
        assert_eq!(
            planned
                .iter()
                .map(|row| row.relationships.len())
                .sum::<usize>(),
            MAX_ATOMIC_RELATIONSHIPS
        );
        assert!(
            planned.len() < changes.len(),
            "the remainder waits for the next cycle"
        );
    }

    #[test]
    fn one_transition_larger_than_the_atomic_edge_bound_fails_visibly() {
        let pids: Vec<u32> = (1..=201).collect();
        let map = resolving_map(&pids);
        let error = plan(&[Change::Opened(socket(4_000, &pids))], &map, 1_000, false).unwrap_err();
        assert!(matches!(
            error,
            AppError::InvalidInput {
                what: "listener transition",
                ..
            }
        ));
    }
}
