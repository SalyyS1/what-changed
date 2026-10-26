//! Session-local service binary evidence shared with relationship producers.
//!
//! A service path is evidence only while the publishing service collector is
//! alive and recent. Consumers get owned rows so no collector holds this lock
//! while it constructs or commits observations.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::collectors::CancelToken;
use crate::domain::ids::canonical_path;

const STALE_AFTER: Duration = Duration::from_secs(30);

/// The minimum service fact needed to prove structural relationships.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEvidence {
    pub name: String,
    pub display_name: String,
    pub binary_path: String,
}

struct Published {
    services: BTreeMap<String, ServiceEvidence>,
    published_at: Instant,
    publisher: Option<CancelToken>,
}

/// Read/write handle shared by services, applications and watched files.
#[derive(Clone)]
pub struct ServiceEvidenceMap {
    published: Arc<RwLock<Published>>,
    readiness: Arc<Readiness>,
}

#[derive(Default)]
struct Readiness {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl Default for ServiceEvidenceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceEvidenceMap {
    pub fn new() -> Self {
        Self {
            published: Arc::new(RwLock::new(Published {
                services: BTreeMap::new(),
                published_at: Instant::now(),
                publisher: None,
            })),
            readiness: Arc::new(Readiness::default()),
        }
    }

    /// Replace the complete service view atomically.
    pub fn publish(
        &self,
        services: impl IntoIterator<Item = ServiceEvidence>,
        publisher: &CancelToken,
        at: Instant,
    ) {
        {
            let mut published = write(&self.published);
            published.services = services
                .into_iter()
                .filter(|service| !service.name.is_empty() && !service.binary_path.is_empty())
                .map(|service| (service.name.to_lowercase(), service))
                .collect();
            published.published_at = at;
            published.publisher = Some(publisher.clone());
        }
        self.mark_ready();
    }

    /// Current services whose executable is inside `root`.
    pub fn under_root(&self, root: &str) -> Vec<ServiceEvidence> {
        self.under_root_at(root, Instant::now())
    }

    fn under_root_at(&self, root: &str, now: Instant) -> Vec<ServiceEvidence> {
        self.current_at(now)
            .into_iter()
            .filter(|service| path_is_within(&service.binary_path, root))
            .collect()
    }

    /// Current services supplied by an application's installation directory.
    pub fn under_install_location(&self, location: &str) -> Vec<ServiceEvidence> {
        self.current_at(Instant::now())
            .into_iter()
            .filter(|service| path_is_within(&service.binary_path, location))
            .collect()
    }

    pub fn invalidate(&self, publisher: &CancelToken) {
        let mut published = write(&self.published);
        if !same_publisher(published.publisher.as_ref(), publisher) {
            return;
        }
        published.services.clear();
        published.publisher = None;
    }

    /// Wait once at startup for the service collector's first complete
    /// publication. An empty service table still counts as ready.
    pub fn wait_for_current(&self, timeout: Duration) -> bool {
        if self.has_current() {
            return true;
        }
        let generation = mutex_lock(&self.readiness.generation);
        let observed = *generation;
        let (generation, _) = self
            .readiness
            .changed
            .wait_timeout_while(generation, timeout, |generation| *generation == observed)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(generation);
        self.has_current()
    }

    fn has_current(&self) -> bool {
        let published = read(&self.published);
        let Some(publisher) = published.publisher.as_ref() else {
            return false;
        };
        !publisher.is_cancelled()
            && Instant::now()
                .checked_duration_since(published.published_at)
                .unwrap_or_default()
                <= STALE_AFTER
    }

    fn mark_ready(&self) {
        let mut generation = mutex_lock(&self.readiness.generation);
        *generation = generation.wrapping_add(1);
        self.readiness.changed.notify_all();
    }

    fn current_at(&self, now: Instant) -> Vec<ServiceEvidence> {
        let published = read(&self.published);
        let Some(publisher) = published.publisher.as_ref() else {
            return Vec::new();
        };
        let age = now
            .checked_duration_since(published.published_at)
            .unwrap_or_default();
        if publisher.is_cancelled() || age > STALE_AFTER {
            return Vec::new();
        }
        published.services.values().cloned().collect()
    }
}

fn path_is_within(path: &str, root: &str) -> bool {
    let path = canonical_path(path);
    let root = canonical_path(root);
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn same_publisher(current: Option<&CancelToken>, candidate: &CancelToken) -> bool {
    current.is_some_and(|current| Arc::ptr_eq(&current.0, &candidate.0))
}

fn read(lock: &RwLock<Published>) -> std::sync::RwLockReadGuard<'_, Published> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write(lock: &RwLock<Published>) -> std::sync::RwLockWriteGuard<'_, Published> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn mutex_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn service(name: &str, binary_path: &str) -> ServiceEvidence {
        ServiceEvidence {
            name: name.to_owned(),
            display_name: name.to_owned(),
            binary_path: binary_path.to_owned(),
        }
    }

    #[test]
    fn containment_is_case_insensitive_and_component_bounded() {
        let map = ServiceEvidenceMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish(
            [
                service("api", r"C:\Work\API\bin\api.exe"),
                service("lookalike", r"C:\Work\API-old\api.exe"),
            ],
            &token,
            at,
        );
        assert_eq!(map.under_root_at(r"c:\work\api", at).len(), 1);
    }

    #[test]
    fn replacement_cancellation_and_staleness_remove_evidence() {
        let map = ServiceEvidenceMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish([service("api", r"C:\Apps\Api\api.exe")], &token, at);
        assert_eq!(map.under_root_at(r"C:\Apps", at).len(), 1);
        assert!(map
            .under_root_at(r"C:\Apps", at + STALE_AFTER + Duration::from_millis(1))
            .is_empty());

        map.publish([service("api", r"C:\Apps\Api\api.exe")], &token, at);
        token.cancel();
        assert!(map.under_root_at(r"C:\Apps", at).is_empty());

        let replacement = CancelToken::default();
        map.publish([service("other", r"C:\Other\other.exe")], &replacement, at);
        map.invalidate(&token);
        assert_eq!(map.under_root_at(r"C:\Other", at).len(), 1);
        map.invalidate(&replacement);
        assert!(map.under_root_at(r"C:\Other", at).is_empty());
    }
}
