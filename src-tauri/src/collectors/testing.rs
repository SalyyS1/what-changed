//! Scripted collectors and live contexts used by the runtime's unit tests.

#![allow(clippy::panic)]

use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use crate::app::Monitoring;
use crate::collectors::{
    CancelToken, Collector, CollectorContext, EmitHandle, MonitorHandle, RawObservation,
    StateReadHandle, StatusDetailsHandle, WakeHandle,
};
use crate::domain::CollectorDescriptor;
use crate::error::{AppError, AppResult};
use crate::platform::traits::DockerTransport;
use crate::storage::CollectorCommitReceipt;
use crate::storage::Db;

pub enum ScriptedStep {
    Emit(Vec<RawObservation>),
    Fail,
    Panic,
}

pub struct ScriptedCollector {
    pub descriptor: CollectorDescriptor,
    pub steps: VecDeque<ScriptedStep>,
}

/// One deterministic Docker transport script used by reconnect and resync tests.
pub enum ScriptedDockerStep {
    Body(Vec<u8>),
    DisconnectAfter(Vec<u8>),
    Unavailable,
}

/// Queue-backed transport with no Docker or Windows dependency.
pub struct ScriptedDockerTransport {
    pub get_steps: VecDeque<ScriptedDockerStep>,
    pub stream_steps: VecDeque<ScriptedDockerStep>,
    pub requested_paths: Vec<String>,
}

impl ScriptedDockerTransport {
    pub fn new(
        get_steps: impl IntoIterator<Item = ScriptedDockerStep>,
        stream_steps: impl IntoIterator<Item = ScriptedDockerStep>,
    ) -> Self {
        Self {
            get_steps: get_steps.into_iter().collect(),
            stream_steps: stream_steps.into_iter().collect(),
            requested_paths: Vec::new(),
        }
    }

    fn next(&mut self, stream: bool) -> AppResult<ScriptedDockerStep> {
        let step = if stream {
            self.stream_steps.pop_front()
        } else {
            self.get_steps.pop_front()
        };
        match step {
            Some(step @ (ScriptedDockerStep::Body(_) | ScriptedDockerStep::DisconnectAfter(_))) => {
                Ok(step)
            }
            Some(ScriptedDockerStep::Unavailable) | None => Err(AppError::Platform {
                operation: "open Docker transport",
                reason: "unavailable".to_owned(),
            }),
        }
    }
}

impl DockerTransport for ScriptedDockerTransport {
    fn get(&mut self, path: &str) -> AppResult<Vec<u8>> {
        self.requested_paths.push(path.to_owned());
        match self.next(false)? {
            ScriptedDockerStep::Body(body) | ScriptedDockerStep::DisconnectAfter(body) => Ok(body),
            ScriptedDockerStep::Unavailable => Err(AppError::Platform {
                operation: "open Docker transport",
                reason: "unavailable".to_owned(),
            }),
        }
    }

    fn stream(&mut self, path: &str) -> AppResult<Box<dyn BufRead + Send>> {
        self.requested_paths.push(path.to_owned());
        match self.next(true)? {
            ScriptedDockerStep::Body(body) => Ok(Box::new(Cursor::new(body))),
            ScriptedDockerStep::DisconnectAfter(body) => {
                Ok(Box::new(BufReader::new(DisconnectingReader {
                    body: Cursor::new(body),
                    disconnected: false,
                })))
            }
            ScriptedDockerStep::Unavailable => Err(AppError::Platform {
                operation: "open Docker transport",
                reason: "unavailable".to_owned(),
            }),
        }
    }
}

struct DisconnectingReader {
    body: Cursor<Vec<u8>>,
    disconnected: bool,
}

impl Read for DisconnectingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.body.read(buffer)?;
        if read > 0 {
            return Ok(read);
        }
        if self.disconnected {
            return Ok(0);
        }
        self.disconnected = true;
        Err(io::Error::new(
            io::ErrorKind::ConnectionReset,
            "scripted disconnect",
        ))
    }
}

impl Collector for ScriptedCollector {
    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, _ctx: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        match self.steps.pop_front() {
            Some(ScriptedStep::Emit(value)) => Ok(value),
            Some(ScriptedStep::Fail) => Err(AppError::InvalidInput {
                what: "scripted collector",
                reason: "scripted failure".to_owned(),
            }),
            Some(ScriptedStep::Panic) => panic!("scripted panic"),
            None => Ok(Vec::new()),
        }
    }
}

/// What [`live_context`] hands back, keeping every part of it alive.
pub struct LiveContext {
    /// The context to start a collector with.
    pub context: CollectorContext,
    /// The database behind its state handle. Held so reads can be asserted on.
    pub db: Arc<Db>,
    /// The directory the database lives in. Held so the files are not removed
    /// from under the collector while a test is still running.
    pub directory: PathBuf,
}

/// A real collector context over a fresh temporary database.
///
/// The state handle is the production one, so a unit test that starts a
/// collector with this exercises real baseline hydration and real reads rather
/// than a stub that always answers `None`. Emission and wake are inert: a unit
/// test drives cycles by hand and asserts on what the collector returns.
pub fn live_context(collector_id: &'static str) -> AppResult<LiveContext> {
    let directory = crate::testing::temp_dir(&format!("collector-context-{collector_id}"));
    let db = Arc::new(Db::open_in(&directory)?);
    Ok(LiveContext {
        context: CollectorContext {
            monitor: MonitorHandle(Arc::new(Monitoring::running())),
            emit: EmitHandle::new(|_observations| Ok(())),
            wake: WakeHandle::new(|| Ok(())),
            status_details: StatusDetailsHandle::new(|_details| Ok(())),
            automatic_checkpoint: crate::collectors::AutomaticCheckpointHandle::noop(),
            state: StateReadHandle::new(Arc::clone(&db), collector_id),
            cancel: CancelToken::default(),
        },
        db,
        directory,
    })
}

/// Persist one atomic collector snapshot through the production normalizer.
#[doc(hidden)]
pub fn persist_atomic_snapshot(
    db: &Db,
    collector_id: &'static str,
    snapshot: crate::collectors::CollectorSnapshot,
) -> AppResult<Option<(crate::collectors::CommitToken, CollectorCommitReceipt)>> {
    let crate::collectors::CollectorSnapshot::Atomic {
        token,
        observations,
        relationships,
        baseline,
        config_snapshots,
    } = snapshot
    else {
        return Ok(None);
    };
    let commit = crate::collectors::normalize::Normalizer::new().prepare_atomic_with_config(
        collector_id,
        observations,
        relationships,
        baseline,
        config_snapshots,
    )?;
    db.commit_collector(commit)
        .map(|receipt| Some((token, receipt)))
}
