//! The difference between the listeners storage knows about and the ones the
//! machine has now.
//!
//! The key is the endpoint — protocol, address and port — because that is what a
//! person asks about. The owner set rides along as the value, so an endpoint that
//! is still bound but has changed hands is one modification rather than a close
//! and an open that nothing corroborates.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::domain::ids::{self, Protocol};
use crate::platform::traits::SocketRow;

/// One listening endpoint and who holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedSocket {
    /// TCP or UDP.
    pub protocol: Protocol,
    /// Local address in text form.
    pub address: String,
    /// Local port.
    pub port: u16,
    /// Every pid the table attributes to this endpoint.
    ///
    /// Normally one. A socket bound for reuse can have more, and choosing
    /// between them would be a claim the table does not support — so the set is
    /// kept whole and the event names all of it.
    pub owners: BTreeSet<u32>,
}

impl TrackedSocket {
    /// The endpoint's stable id, which is also the key it is tracked under.
    pub fn entity_id(&self) -> String {
        ids::port(self.protocol, &self.address, self.port)
    }

    /// How the endpoint reads in a list: `TCP 0.0.0.0:4000`.
    pub fn display_name(&self) -> String {
        format!(
            "{} {}:{}",
            self.protocol.as_str().to_uppercase(),
            self.address,
            self.port
        )
    }
}

/// Endpoints by id.
pub type Sockets = BTreeMap<String, TrackedSocket>;

/// Reusable, unordered index of the current provider table.
///
/// Durable baselines stay ordered for deterministic serialization. This map is
/// session-local scratch: clearing it retains buckets, so a quiet two-second
/// poll does not allocate another whole endpoint index.
pub type SocketIndex = HashMap<String, TrackedSocket>;

/// This cycle's view of the machine, grouped by endpoint.
///
/// The tables report one row per owning pid, so a shared endpoint arrives
/// several times and is folded into one entry with an owner set. Folding here
/// rather than in the diff is what keeps "the same port, two owners" from
/// reading as two separate listeners that each changed.
pub fn sockets(rows: &[SocketRow]) -> Sockets {
    let mut index = SocketIndex::with_capacity(rows.len());
    socket_index_into(rows, &mut index);
    ordered_sockets(&index)
}

/// Fold provider rows into reusable caller-owned endpoint storage.
pub fn socket_index_into(rows: &[SocketRow], sockets: &mut SocketIndex) {
    sockets.clear();
    sockets.reserve(rows.len());
    for row in rows {
        let key = ids::port(row.protocol, &row.address, row.port);
        let entry = sockets.entry(key).or_insert_with(|| TrackedSocket {
            protocol: row.protocol,
            address: row.address.clone(),
            port: row.port,
            owners: BTreeSet::new(),
        });
        entry.owners.insert(row.owning_pid);
    }
}

/// Materialize an ordered durable target only when a snapshot changed.
pub fn ordered_sockets(index: &SocketIndex) -> Sockets {
    index
        .iter()
        .map(|(key, socket)| (key.clone(), socket.clone()))
        .collect()
}

/// Whether the reusable live index says exactly what the durable view says.
pub fn unchanged(acknowledged: &Sockets, current: &SocketIndex) -> bool {
    acknowledged.len() == current.len()
        && current
            .iter()
            .all(|(key, socket)| acknowledged.get(key) == Some(socket))
}

/// One indivisible difference between two listener snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// An endpoint that is listening and was not.
    Opened(TrackedSocket),
    /// An endpoint that was listening and is not.
    Closed(TrackedSocket),
    /// Still bound, held by a different set of processes.
    Reowned {
        /// The endpoint as storage knows it.
        before: TrackedSocket,
        /// The endpoint as the table reports it now.
        after: TrackedSocket,
    },
}

impl Change {
    /// The endpoint the event is about.
    pub const fn socket(&self) -> &TrackedSocket {
        match self {
            Change::Opened(socket) | Change::Closed(socket) => socket,
            Change::Reowned { after, .. } => after,
        }
    }

    /// The owner set to resolve into graph evidence.
    ///
    /// A close gets none. The process that held the port may already be gone,
    /// and the edge written when the port opened still stands on its own
    /// evidence; re-asserting it against a pid that no longer exists would be a
    /// relationship nobody can check.
    pub const fn evidence_owners(&self) -> Option<&BTreeSet<u32>> {
        match self {
            Change::Opened(socket) | Change::Reowned { after: socket, .. } => Some(&socket.owners),
            Change::Closed(_) => None,
        }
    }

    /// The owner set that was there before, for the copy a person reads.
    pub const fn previous_owners(&self) -> Option<&BTreeSet<u32>> {
        match self {
            Change::Reowned { before, .. } => Some(&before.owners),
            Change::Opened(_) | Change::Closed(_) => None,
        }
    }
}

/// Every difference between the acknowledged view and this cycle's snapshot.
///
/// Ordered by endpoint id, so a burst arrives in a stable order and a remainder
/// left over from one cycle resumes in the same order on the next.
pub fn diff(acknowledged: &Sockets, current: &Sockets) -> Vec<Change> {
    let mut changes = Vec::new();
    for (key, socket) in current {
        match acknowledged.get(key) {
            None => changes.push(Change::Opened(socket.clone())),
            Some(previous) if previous.owners == socket.owners => {}
            Some(previous) => changes.push(Change::Reowned {
                before: previous.clone(),
                after: socket.clone(),
            }),
        }
    }
    for (key, socket) in acknowledged {
        if !current.contains_key(key) {
            changes.push(Change::Closed(socket.clone()));
        }
    }
    changes
}

/// Compare one existing endpoint through the same owner-set classifier used by
/// live listener collection.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    let before: TrackedSocket = serde_json::from_value(before.clone()).ok()?;
    let after: TrackedSocket = serde_json::from_value(after.clone()).ok()?;
    let key = "checkpoint".to_owned();
    let before_map = [(key.clone(), before)].into_iter().collect();
    let after_map = [(key, after)].into_iter().collect();
    Some(
        diff(&before_map, &after_map)
            .into_iter()
            .filter_map(|change| match change {
                Change::Reowned { before, after } => Some((
                    "owners".to_owned(),
                    serde_json::json!(before.owners),
                    serde_json::json!(after.owners),
                )),
                Change::Opened(_) | Change::Closed(_) => None,
            })
            .collect(),
    )
}

/// Diff an ordered durable view against the reusable live index.
///
/// This is called only after [`unchanged`] returns false. Sorting the changed
/// keys keeps burst order deterministic without allocating a sorted copy of
/// the entire table on the common quiet path.
pub fn diff_index(acknowledged: &Sockets, current: &SocketIndex) -> Vec<Change> {
    let mut present = Vec::new();
    for (key, socket) in current {
        match acknowledged.get(key) {
            None => present.push((key.clone(), Change::Opened(socket.clone()))),
            Some(previous) if previous.owners == socket.owners => {}
            Some(previous) => present.push((
                key.clone(),
                Change::Reowned {
                    before: previous.clone(),
                    after: socket.clone(),
                },
            )),
        }
    }
    present.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    let mut closed = acknowledged
        .iter()
        .filter(|(key, _)| !current.contains_key(*key))
        .map(|(key, socket)| (key.clone(), Change::Closed(socket.clone())))
        .collect::<Vec<_>>();
    closed.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    present
        .into_iter()
        .chain(closed)
        .map(|(_, change)| change)
        .collect()
}

/// Record one change in the acknowledged view.
pub fn apply(acknowledged: &mut Sockets, change: &Change) {
    match change {
        Change::Opened(socket) | Change::Reowned { after: socket, .. } => {
            acknowledged.insert(socket.entity_id(), socket.clone());
        }
        Change::Closed(socket) => {
            acknowledged.remove(&socket.entity_id());
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn row(protocol: Protocol, address: &str, port: u16, pid: u32) -> SocketRow {
        SocketRow {
            protocol,
            address: address.to_owned(),
            port,
            owning_pid: pid,
        }
    }

    fn tcp(port: u16, owners: &[u32]) -> TrackedSocket {
        TrackedSocket {
            protocol: Protocol::Tcp,
            address: "0.0.0.0".to_owned(),
            port,
            owners: owners.iter().copied().collect(),
        }
    }

    #[test]
    fn rows_sharing_an_endpoint_fold_into_one_owner_set() {
        let sockets = sockets(&[
            row(Protocol::Tcp, "0.0.0.0", 4_000, 7),
            row(Protocol::Tcp, "0.0.0.0", 4_000, 9),
            row(Protocol::Udp, "0.0.0.0", 4_000, 7),
        ]);
        assert_eq!(sockets.len(), 2, "udp 4000 is a different endpoint");
        assert_eq!(
            sockets["port:tcp:0.0.0.0:4000"].owners,
            BTreeSet::from([7, 9])
        );
        assert_eq!(
            sockets["port:tcp:0.0.0.0:4000"].display_name(),
            "TCP 0.0.0.0:4000"
        );
    }

    #[test]
    fn reusable_index_matches_the_ordered_snapshot_without_losing_capacity() {
        let rows = [
            row(Protocol::Tcp, "0.0.0.0", 4_000, 7),
            row(Protocol::Tcp, "0.0.0.0", 4_000, 9),
            row(Protocol::Udp, "::", 53, 7),
        ];
        let mut index = SocketIndex::with_capacity(16);
        let capacity = index.capacity();
        socket_index_into(&rows, &mut index);
        assert_eq!(ordered_sockets(&index), sockets(&rows));
        assert!(unchanged(&sockets(&rows), &index));

        socket_index_into(&rows[..1], &mut index);
        assert_eq!(index.capacity(), capacity, "clear retains hash buckets");
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn an_unchanged_endpoint_with_the_same_owners_is_not_a_change() {
        let acknowledged = sockets(&[row(Protocol::Tcp, "0.0.0.0", 4_000, 7)]);
        assert!(diff(&acknowledged, &acknowledged).is_empty());
    }

    #[test]
    fn an_owner_change_on_a_still_bound_endpoint_is_one_modification() {
        let acknowledged = sockets(&[row(Protocol::Tcp, "0.0.0.0", 4_000, 7)]);
        let current = sockets(&[row(Protocol::Tcp, "0.0.0.0", 4_000, 9)]);
        let changes = diff(&acknowledged, &current);
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            Change::Reowned { before, after } => {
                assert_eq!(before.owners, BTreeSet::from([7]));
                assert_eq!(after.owners, BTreeSet::from([9]));
            }
            other => panic!("a reowned endpoint must not read as a close and an open: {other:?}"),
        }
        assert_eq!(
            changes[0].previous_owners(),
            Some(&BTreeSet::from([7])),
            "the copy can name what it was before"
        );
    }

    #[test]
    fn opens_and_closes_are_reported_separately_and_in_key_order() {
        let acknowledged = sockets(&[
            row(Protocol::Tcp, "0.0.0.0", 5_000, 1),
            row(Protocol::Tcp, "127.0.0.1", 9_000, 2),
        ]);
        let current = sockets(&[
            row(Protocol::Tcp, "0.0.0.0", 4_000, 3),
            row(Protocol::Tcp, "0.0.0.0", 5_000, 1),
        ]);
        let changes = diff(&acknowledged, &current);
        assert_eq!(changes.len(), 2);
        assert!(matches!(&changes[0], Change::Opened(socket) if socket.port == 4_000));
        assert!(matches!(&changes[1], Change::Closed(socket) if socket.port == 9_000));

        let mut current_index = SocketIndex::new();
        socket_index_into(
            &[
                row(Protocol::Tcp, "0.0.0.0", 4_000, 3),
                row(Protocol::Tcp, "0.0.0.0", 5_000, 1),
            ],
            &mut current_index,
        );
        assert_eq!(diff_index(&acknowledged, &current_index), changes);
    }

    #[test]
    fn applying_changes_makes_the_acknowledged_view_match_the_machine() {
        let mut acknowledged = sockets(&[row(Protocol::Tcp, "0.0.0.0", 9_000, 2)]);
        let changes = diff(
            &acknowledged,
            &sockets(&[
                row(Protocol::Tcp, "0.0.0.0", 4_000, 7),
                row(Protocol::Tcp, "0.0.0.0", 4_000, 9),
            ]),
        );
        for change in &changes {
            apply(&mut acknowledged, change);
        }
        assert_eq!(acknowledged.len(), 1);
        assert_eq!(
            acknowledged["port:tcp:0.0.0.0:4000"].owners,
            BTreeSet::from([7, 9])
        );
    }

    #[test]
    fn a_close_carries_no_owner_evidence_but_a_reowner_does() {
        let closed = Change::Closed(tcp(4_000, &[7]));
        assert_eq!(closed.evidence_owners(), None);
        assert_eq!(closed.socket().port, 4_000);

        let opened = Change::Opened(tcp(4_000, &[7]));
        assert_eq!(opened.evidence_owners(), Some(&BTreeSet::from([7])));
        assert_eq!(opened.previous_owners(), None);

        let reowned = Change::Reowned {
            before: tcp(4_000, &[7]),
            after: tcp(4_000, &[9]),
        };
        assert_eq!(reowned.evidence_owners(), Some(&BTreeSet::from([9])));
        assert_eq!(reowned.socket().owners, BTreeSet::from([9]));
    }

    #[test]
    fn an_endpoint_survives_serialization_so_a_recreation_resumes_it() {
        let sockets = sockets(&[row(Protocol::Udp, "::", 53, 7)]);
        let text = serde_json::to_string(&sockets).unwrap();
        let restored: Sockets = serde_json::from_str(&text).unwrap();
        assert_eq!(restored, sockets);
        assert_eq!(restored["port:udp::::53"].protocol, Protocol::Udp);
    }
}
