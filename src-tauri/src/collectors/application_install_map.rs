//! Session-local application installation evidence for service relationships.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::collectors::CancelToken;
use crate::domain::ids::canonical_path;

const STALE_AFTER: Duration = Duration::from_secs(10 * 60);

/// The minimum application fact needed to identify a provider of a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationInstallEvidence {
    pub hive: String,
    pub registry_key: String,
    pub display_name: String,
    pub install_location: String,
}

struct Published {
    applications: BTreeMap<(String, String), ApplicationInstallEvidence>,
    published_at: Instant,
    publisher: Option<CancelToken>,
}

#[derive(Clone)]
pub struct ApplicationInstallMap {
    published: Arc<RwLock<Published>>,
    readiness: Arc<Readiness>,
}

#[derive(Default)]
struct Readiness {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl Default for ApplicationInstallMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationInstallMap {
    pub fn new() -> Self {
        Self {
            published: Arc::new(RwLock::new(Published {
                applications: BTreeMap::new(),
                published_at: Instant::now(),
                publisher: None,
            })),
            readiness: Arc::new(Readiness::default()),
        }
    }

    pub fn publish(
        &self,
        applications: impl IntoIterator<Item = ApplicationInstallEvidence>,
        publisher: &CancelToken,
        at: Instant,
    ) {
        {
            let mut published = write(&self.published);
            published.applications = applications
                .into_iter()
                .filter(|application| {
                    !application.hive.is_empty()
                        && !application.registry_key.is_empty()
                        && !application.install_location.is_empty()
                })
                .map(|application| {
                    (
                        (
                            application.hive.to_lowercase(),
                            application.registry_key.to_lowercase(),
                        ),
                        application,
                    )
                })
                .collect();
            published.published_at = at;
            published.publisher = Some(publisher.clone());
        }
        self.mark_ready();
    }

    /// Current applications whose install root contains `binary_path`.
    pub fn providers_of(&self, binary_path: &str) -> Vec<ApplicationInstallEvidence> {
        self.providers_of_at(binary_path, Instant::now())
    }

    fn providers_of_at(&self, binary_path: &str, now: Instant) -> Vec<ApplicationInstallEvidence> {
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
        let mut matches = published
            .applications
            .values()
            .filter(|application| path_is_within(binary_path, &application.install_location))
            .cloned()
            .collect::<Vec<_>>();
        let deepest = matches
            .iter()
            .map(|application| path_depth(&application.install_location))
            .max()
            .unwrap_or(0);
        matches.retain(|application| path_depth(&application.install_location) == deepest);
        matches
    }

    pub fn invalidate(&self, publisher: &CancelToken) {
        let mut published = write(&self.published);
        if !same_publisher(published.publisher.as_ref(), publisher) {
            return;
        }
        published.applications.clear();
        published.publisher = None;
    }

    /// Wait once at startup for the application collector's first complete
    /// publication. An empty application table still counts as ready.
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
}

fn path_is_within(path: &str, root: &str) -> bool {
    let path = canonical_path(path);
    let root = canonical_path(root);
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn path_depth(path: &str) -> usize {
    canonical_path(path)
        .split('\\')
        .filter(|component| !component.is_empty())
        .count()
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

    fn application(location: &str) -> ApplicationInstallEvidence {
        ApplicationInstallEvidence {
            hive: "HKLM64".to_owned(),
            registry_key: "AcmeApi".to_owned(),
            display_name: "Acme API".to_owned(),
            install_location: location.to_owned(),
        }
    }

    #[test]
    fn only_component_bounded_install_roots_match_a_service_binary() {
        let map = ApplicationInstallMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        let mut other = application(r"C:\Apps\Other");
        other.registry_key = "OtherApp".to_owned();
        map.publish([application(r"C:\Apps\Acme"), other], &token, at);
        let providers = map.providers_of_at(r"c:\apps\acme\api.exe", at);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].display_name, "Acme API");
        assert!(map
            .providers_of_at(r"C:\Apps\Acme-old\api.exe", at)
            .is_empty());
    }

    #[test]
    fn the_most_specific_install_root_owns_the_service() {
        let map = ApplicationInstallMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        let broad = application(r"C:\Apps");
        let mut specific = application(r"C:\Apps\Acme");
        specific.registry_key = "AcmeApiSpecific".to_owned();
        map.publish([broad, specific], &token, at);

        let providers = map.providers_of_at(r"C:\Apps\Acme\api.exe", at);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].registry_key, "AcmeApiSpecific");
    }

    #[test]
    fn cancelled_or_stale_publication_cannot_supply_an_edge() {
        let map = ApplicationInstallMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish([application(r"C:\Apps\Acme")], &token, at);
        assert!(map
            .providers_of_at(
                r"C:\Apps\Acme\api.exe",
                at + STALE_AFTER + Duration::from_millis(1),
            )
            .is_empty());
        token.cancel();
        assert!(map.providers_of_at(r"C:\Apps\Acme\api.exe", at).is_empty());
    }
}
