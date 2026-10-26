//! Session-local publication of ports that are currently listening.
//!
//! The file collector uses this map only for exact `PORT`-shaped config
//! values. A publication is trusted while the listener collector instance
//! that produced it is alive and recent; after pause, disable, restart or a
//! prolonged gap it answers with no evidence rather than a stale edge.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::collectors::listeners::diff::TrackedSocket;
use crate::collectors::CancelToken;
use crate::domain::ids::Protocol;

const STALE_AFTER: Duration = Duration::from_secs(30);
const RESERVE_ENDPOINTS: usize = 128;

/// One port entity proved by the listener provider's current complete table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningEndpoint {
    pub protocol: Protocol,
    pub address: String,
    pub port: u16,
}

impl ListeningEndpoint {
    pub fn display_name(&self) -> String {
        format!(
            "{} {}:{}",
            self.protocol.as_str().to_uppercase(),
            self.address,
            self.port
        )
    }
}

struct Published {
    by_port: HashMap<u16, Vec<ListeningEndpoint>>,
    spare: HashMap<u16, Vec<ListeningEndpoint>>,
    published_at: Instant,
    publisher: Option<CancelToken>,
}

/// Read handle shared by the listener and watched-file collectors.
#[derive(Clone)]
pub struct ListeningPortMap {
    published: Arc<RwLock<Published>>,
}

impl Default for ListeningPortMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ListeningPortMap {
    pub fn new() -> Self {
        Self {
            published: Arc::new(RwLock::new(Published {
                by_port: HashMap::with_capacity(RESERVE_ENDPOINTS),
                spare: HashMap::with_capacity(RESERVE_ENDPOINTS),
                published_at: Instant::now(),
                publisher: None,
            })),
        }
    }

    /// Atomically replace the complete listener publication.
    pub fn publish(
        &self,
        sockets: impl IntoIterator<Item = TrackedSocket>,
        publisher: &CancelToken,
        at: Instant,
    ) {
        let mut published = write(&self.published);
        {
            let Published {
                by_port: active,
                spare,
                ..
            } = &mut *published;
            spare.clear();
            for socket in sockets {
                spare
                    .entry(socket.port)
                    .or_default()
                    .push(ListeningEndpoint {
                        protocol: socket.protocol,
                        address: socket.address,
                        port: socket.port,
                    });
            }
            for endpoints in spare.values_mut() {
                endpoints.sort_by(|left, right| {
                    (left.protocol.as_str(), left.address.as_str())
                        .cmp(&(right.protocol.as_str(), right.address.as_str()))
                });
                endpoints.dedup();
            }
            std::mem::swap(active, spare);
            spare.clear();
        }
        published.published_at = at;
        published.publisher = Some(publisher.clone());
    }

    /// Endpoints for an exact numeric port, if the publication is still live.
    pub fn endpoints(&self, port: u16) -> Vec<ListeningEndpoint> {
        self.endpoints_at(port, Instant::now())
    }

    fn endpoints_at(&self, port: u16, now: Instant) -> Vec<ListeningEndpoint> {
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
        published.by_port.get(&port).cloned().unwrap_or_default()
    }

    /// Stop a discarded listener instance from supplying relationship evidence.
    pub fn invalidate(&self, publisher: &CancelToken) {
        let mut published = write(&self.published);
        if !published
            .publisher
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &publisher.0))
        {
            return;
        }
        published.by_port.clear();
        published.spare.clear();
        published.publisher = None;
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        read(&self.published).by_port.values().map(Vec::len).sum()
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
    use super::*;
    use std::collections::BTreeSet;

    fn socket(protocol: Protocol, address: &str, port: u16) -> TrackedSocket {
        TrackedSocket {
            protocol,
            address: address.to_owned(),
            port,
            owners: BTreeSet::new(),
        }
    }

    #[test]
    fn exact_port_lookup_returns_every_current_endpoint_in_stable_order() {
        let map = ListeningPortMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish(
            [
                socket(Protocol::Udp, "0.0.0.0", 4_000),
                socket(Protocol::Tcp, "127.0.0.1", 4_000),
                socket(Protocol::Tcp, "0.0.0.0", 3_000),
            ],
            &token,
            at,
        );
        let endpoints = map.endpoints_at(4_000, at);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].protocol, Protocol::Tcp);
        assert_eq!(endpoints[1].protocol, Protocol::Udp);
        assert!(map.endpoints_at(4_001, at).is_empty());
    }

    #[test]
    fn replacement_is_complete_and_duplicates_are_folded() {
        let map = ListeningPortMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        let repeated = socket(Protocol::Tcp, "0.0.0.0", 3_000);
        map.publish([repeated.clone(), repeated], &token, at);
        assert_eq!(map.len(), 1);
        map.publish([socket(Protocol::Tcp, "0.0.0.0", 4_000)], &token, at);
        assert!(map.endpoints_at(3_000, at).is_empty());
        assert_eq!(map.endpoints_at(4_000, at).len(), 1);
    }

    #[test]
    fn cancellation_invalidation_and_staleness_all_remove_evidence() {
        let map = ListeningPortMap::new();
        let token = CancelToken::default();
        let at = Instant::now();
        map.publish([socket(Protocol::Tcp, "0.0.0.0", 3_000)], &token, at);
        assert_eq!(map.endpoints_at(3_000, at).len(), 1);
        assert!(map
            .endpoints_at(3_000, at + STALE_AFTER + Duration::from_millis(1))
            .is_empty());

        map.publish([socket(Protocol::Tcp, "0.0.0.0", 3_000)], &token, at);
        token.cancel();
        assert!(map.endpoints_at(3_000, at).is_empty());

        let replacement = CancelToken::default();
        map.publish([socket(Protocol::Tcp, "0.0.0.0", 4_000)], &replacement, at);
        map.invalidate(&token);
        assert_eq!(map.endpoints_at(4_000, at).len(), 1);
        map.invalidate(&replacement);
        assert!(map.endpoints_at(4_000, at).is_empty());
    }
}
