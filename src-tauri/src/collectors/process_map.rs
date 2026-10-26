//! The session-local process identity map.
//!
//! One process enumeration serves two collectors: the process collector
//! publishes what it saw, and the listener collector reads it to name the owner
//! of a socket rather than enumerating the process table a second time.
//!
//! The map is deliberately weaker than a database. It holds only what the
//! current session has seen, it expires, and it stops answering when its
//! publisher stops being live — because attributing a port to a process that
//! was replaced ten minutes ago is a false relationship, and a false
//! relationship is worse than an absent one.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::collectors::CancelToken;
use crate::domain::ids::ProcessId;
use crate::domain::Confidence;

/// How long a published snapshot stays trustworthy without a fresh one.
///
/// The publisher polls every second, so thirty seconds is thirty missed cycles:
/// long enough to ride out one slow commit or a scheduling delay, short enough
/// that a paused or dead publisher stops naming owners.
const STALE_AFTER: Duration = Duration::from_secs(30);

/// Both sides of the publication swap start at normal process-table size, so
/// a once-per-second refresh does not grow a fresh hash table.
const RESERVE_IDENTITIES: usize = 512;

/// A process a socket can be attributed to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessIdentity {
    /// The stable entity id, computed once by the publisher and never
    /// recomputed from text a sanitizer may have rewritten.
    pub entity_id: ProcessId,
    /// The image name a person recognises.
    pub display_name: String,
    /// How sure the publisher is of this identity. A process that denied an
    /// executable-path read is named by image alone, and the analysis engine
    /// refuses to build its strongest link on that.
    pub confidence: Confidence,
}

struct Published {
    rows: HashMap<u32, ProcessIdentity>,
    spare: HashMap<u32, ProcessIdentity>,
    published_at: Instant,
    publisher: Option<CancelToken>,
}

/// A read handle on the shared process snapshot.
///
/// Cloning shares one snapshot; there is no per-collector copy to fall out of
/// agreement with.
#[derive(Clone)]
pub struct ProcessMap {
    published: Arc<RwLock<Published>>,
}

impl Default for ProcessMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessMap {
    /// An empty map with no publisher, which resolves nothing.
    pub fn new() -> Self {
        Self {
            published: Arc::new(RwLock::new(Published {
                rows: HashMap::with_capacity(RESERVE_IDENTITIES),
                spare: HashMap::with_capacity(RESERVE_IDENTITIES),
                published_at: Instant::now(),
                publisher: None,
            })),
        }
    }

    /// Replace the whole snapshot with one complete, successful enumeration.
    ///
    /// Partial state is never published: a chunked persistence drain is a
    /// storage concern, and a reader that saw half a process table would
    /// attribute sockets to owners the publisher has not finished confirming.
    pub fn publish(
        &self,
        rows: BTreeMap<u32, ProcessIdentity>,
        publisher: &CancelToken,
        at: Instant,
    ) {
        self.publish_entries(rows, publisher, at);
    }

    /// Publish identities through two reusable hash tables.
    ///
    /// The process collector feeds this directly from its current tracked
    /// snapshot, avoiding a temporary whole-table map. Filling the spare side
    /// before the swap also means readers see either the old complete table or
    /// the new complete table, never a half-refreshed one.
    pub fn publish_entries(
        &self,
        rows: impl IntoIterator<Item = (u32, ProcessIdentity)>,
        publisher: &CancelToken,
        at: Instant,
    ) {
        let mut published = write(&self.published);
        {
            let Published {
                rows: active,
                spare,
                ..
            } = &mut *published;
            spare.clear();
            spare.extend(rows);
            std::mem::swap(active, spare);
            spare.clear();
        }
        published.published_at = at;
        published.publisher = Some(publisher.clone());
    }

    /// Publish tracked process identities while reusing unchanged owned
    /// strings from the previous publication. Readers still see an atomic
    /// complete table, but a noisy process table no longer clones every
    /// unchanged executable name and entity id on every poll.
    pub fn publish_instances<'a, I>(&self, rows: I, publisher: &CancelToken, at: Instant)
    where
        I: IntoIterator<Item = (u32, &'a ProcessId, &'a str, Confidence)>,
    {
        let mut published = write(&self.published);
        let Published {
            rows: active,
            spare,
            ..
        } = &mut *published;
        std::mem::swap(active, spare);
        active.clear();
        for (pid, entity_id, display_name, confidence) in rows {
            if let Some(existing) = spare.remove(&pid) {
                if existing.entity_id == *entity_id
                    && existing.display_name == display_name
                    && existing.confidence == confidence
                {
                    active.insert(pid, existing);
                    continue;
                }
            }
            active.insert(
                pid,
                ProcessIdentity {
                    entity_id: entity_id.clone(),
                    display_name: display_name.to_owned(),
                    confidence,
                },
            );
        }
        spare.clear();
        published.published_at = at;
        published.publisher = Some(publisher.clone());
    }

    /// Refresh the trust window when the process collector proved that the
    /// complete live table is unchanged. This does not replace rows or create
    /// evidence; it only keeps a healthy publisher's existing publication
    /// from expiring while a quiet poll avoids cloning it.
    pub fn refresh(&self, publisher: &CancelToken, at: Instant) {
        let mut published = write(&self.published);
        if published
            .publisher
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &publisher.0))
        {
            published.published_at = at;
        }
    }

    /// Stop the current publication from answering immediately.
    ///
    /// Pause and stop are lifecycle facts, not eventual-staleness hints. A
    /// listener cycle that happens before the next process publication must not
    /// attribute a socket from the pre-pause table.
    pub fn invalidate(&self, publisher: &CancelToken) {
        let mut published = write(&self.published);
        if !published
            .publisher
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &publisher.0))
        {
            return;
        }
        published.rows.clear();
        published.spare.clear();
        published.publisher = None;
    }

    /// A complete copy of the currently trustworthy publication.
    ///
    /// Listener burst drains persist this alongside their provider target so
    /// graph evidence is the evidence observed with that target, not whatever a
    /// recycled pid means on a later drain cycle.
    pub fn snapshot(&self) -> BTreeMap<u32, ProcessIdentity> {
        self.snapshot_at(Instant::now())
    }

    /// Copy only identities a listener target actually references.
    ///
    /// Most machines have hundreds of processes and tens of listening owners.
    /// Filtering under one read lock avoids cloning the full process table on
    /// every two-second listener poll.
    pub fn snapshot_for(&self, pids: &BTreeSet<u32>) -> BTreeMap<u32, ProcessIdentity> {
        self.snapshot_for_at(pids, Instant::now())
    }

    fn snapshot_at(&self, now: Instant) -> BTreeMap<u32, ProcessIdentity> {
        let published = read(&self.published);
        let Some(publisher) = published.publisher.as_ref() else {
            return BTreeMap::new();
        };
        let age = now
            .checked_duration_since(published.published_at)
            .unwrap_or_default();
        if publisher.is_cancelled() || age > STALE_AFTER {
            return BTreeMap::new();
        }
        published
            .rows
            .iter()
            .map(|(pid, identity)| (*pid, identity.clone()))
            .collect()
    }

    fn snapshot_for_at(
        &self,
        pids: &BTreeSet<u32>,
        now: Instant,
    ) -> BTreeMap<u32, ProcessIdentity> {
        let published = read(&self.published);
        let Some(publisher) = published.publisher.as_ref() else {
            return BTreeMap::new();
        };
        let age = now
            .checked_duration_since(published.published_at)
            .unwrap_or_default();
        if publisher.is_cancelled() || age > STALE_AFTER {
            return BTreeMap::new();
        }
        pids.iter()
            .filter_map(|pid| {
                published
                    .rows
                    .get(pid)
                    .cloned()
                    .map(|identity| (*pid, identity))
            })
            .collect()
    }

    /// The identity of one pid, if a live publisher has seen it recently.
    pub fn resolve(&self, pid: u32) -> Option<ProcessIdentity> {
        self.resolve_at(pid, Instant::now())
    }

    /// [`resolve`](Self::resolve) against an explicit instant, so expiry is
    /// testable without waiting for it.
    pub fn resolve_at(&self, pid: u32, now: Instant) -> Option<ProcessIdentity> {
        let published = read(&self.published);
        let publisher = published.publisher.as_ref()?;
        if publisher.is_cancelled() {
            return None;
        }
        // A clock that steps backwards reads as age zero rather than as an
        // enormous age: the snapshot is not stale because the wall moved.
        let age = now
            .checked_duration_since(published.published_at)
            .unwrap_or_default();
        if age > STALE_AFTER {
            return None;
        }
        published.rows.get(&pid).cloned()
    }

    /// How many processes the current snapshot names.
    pub fn len(&self) -> usize {
        read(&self.published).rows.len()
    }

    /// Whether the current snapshot names nothing.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn read(lock: &RwLock<Published>) -> std::sync::RwLockReadGuard<'_, Published> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write(lock: &RwLock<Published>) -> std::sync::RwLockWriteGuard<'_, Published> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn identity(name: &str) -> ProcessIdentity {
        ProcessIdentity {
            entity_id: ProcessId::from_executable(name),
            display_name: name.to_owned(),
            confidence: Confidence::High,
        }
    }

    fn published() -> (ProcessMap, CancelToken, Instant) {
        let map = ProcessMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish(BTreeMap::from([(4_242, identity("api.exe"))]), &token, at);
        (map, token, at)
    }

    #[test]
    fn an_unpublished_map_names_no_owner() {
        assert!(ProcessMap::new().resolve(4_242).is_none());
        assert!(ProcessMap::new().is_empty());
    }

    #[test]
    fn a_cancelled_publisher_stops_naming_owners_immediately() {
        let (map, token, _) = published();
        assert!(map.resolve(4_242).is_some());
        token.cancel();
        assert_eq!(
            map.resolve(4_242),
            None,
            "a discarded instance must not keep attributing sockets"
        );
    }

    #[test]
    fn a_snapshot_expires_rather_than_naming_a_replaced_process() {
        let (map, token, at) = published();
        assert!(map.resolve_at(4_242, at + STALE_AFTER).is_some());
        assert_eq!(
            map.resolve_at(4_242, at + STALE_AFTER + Duration::from_millis(1)),
            None
        );
        // A clock that steps backwards reads as age zero, not as an enormous
        // age. `checked_sub` because an `Instant` near the monotonic epoch has
        // no earlier instant to subtract from.
        let earlier = at.checked_sub(Duration::from_secs(1)).unwrap_or(at);
        assert!(map.resolve_at(4_242, earlier).is_some());
        assert_eq!(map.len(), 1);
        drop(token);
    }

    #[test]
    fn publishing_replaces_the_whole_snapshot() {
        let (map, token, at) = published();
        map.publish(BTreeMap::from([(7, identity("other.exe"))]), &token, at);
        assert_eq!(map.resolve_at(4_242, at), None);
        assert_eq!(
            map.resolve_at(7, at).map(|row| row.display_name),
            Some("other.exe".to_owned())
        );
    }

    #[test]
    fn refreshing_keeps_rows_and_updates_the_trust_window() {
        let (map, token, at) = published();
        let later = at + Duration::from_secs(1);
        map.refresh(&token, later);
        assert_eq!(
            map.resolve_at(4_242, later + STALE_AFTER),
            Some(identity("api.exe"))
        );
        assert_eq!(
            map.resolve_at(4_242, later + STALE_AFTER + Duration::from_millis(1)),
            None
        );
    }

    #[test]
    fn publishing_instances_reuses_unchanged_owned_identity_text() {
        let map = ProcessMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        let entity = ProcessId::from_executable("api.exe");
        map.publish_instances([(7, &entity, "api.exe", Confidence::High)], &token, at);
        let before = read(&map.published)
            .rows
            .get(&7)
            .unwrap()
            .display_name
            .as_ptr();
        map.publish_instances(
            [(7, &entity, "api.exe", Confidence::High)],
            &token,
            at + Duration::from_millis(1),
        );
        let after = read(&map.published)
            .rows
            .get(&7)
            .unwrap()
            .display_name
            .as_ptr();
        assert_eq!(before, after);
    }

    #[test]
    fn targeted_snapshot_clones_only_requested_live_identities() {
        let map = ProcessMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish(
            BTreeMap::from([
                (7, identity("api.exe")),
                (9, identity("worker.exe")),
                (11, identity("unrelated.exe")),
            ]),
            &token,
            at,
        );
        let selected = map.snapshot_for_at(&BTreeSet::from([7, 9, 99]), at);
        assert_eq!(selected.len(), 2);
        assert!(selected.contains_key(&7));
        assert!(selected.contains_key(&9));
        assert!(!selected.contains_key(&11));
    }

    #[test]
    fn publication_swaps_two_pre_sized_hash_maps() {
        let map = ProcessMap::new();
        let token = CancelToken::default();
        for pid in 1..=3 {
            map.publish(
                BTreeMap::from([(pid, identity(&format!("app-{pid}.exe")))]),
                &token,
                Instant::now(),
            );
        }
        let published = read(&map.published);
        assert!(published.rows.capacity() >= RESERVE_IDENTITIES);
        assert!(published.spare.capacity() >= RESERVE_IDENTITIES);
        assert_eq!(published.rows.len(), 1);
        assert!(published.spare.is_empty());
    }

    #[test]
    fn invalidating_a_publication_removes_every_pre_pause_identity() {
        let (map, token, _) = published();
        assert!(map.resolve(4_242).is_some());
        map.invalidate(&token);
        assert!(map.resolve(4_242).is_none());
        assert!(map.snapshot().is_empty());
        assert!(map.is_empty());
    }

    #[test]
    fn a_snapshot_obeys_the_same_liveness_rule_as_single_pid_resolution() {
        let (map, token, at) = published();
        assert_eq!(map.snapshot_at(at).len(), 1);
        token.cancel();
        assert!(map.snapshot_at(at).is_empty());
    }

    #[test]
    fn an_old_publisher_cannot_invalidate_its_replacement() {
        let (map, old, at) = published();
        let replacement = CancelToken::default();
        map.publish(
            BTreeMap::from([(7, identity("replacement.exe"))]),
            &replacement,
            at,
        );
        map.invalidate(&old);
        assert_eq!(
            map.resolve_at(7, at).map(|row| row.display_name),
            Some("replacement.exe".to_owned())
        );
    }
}
