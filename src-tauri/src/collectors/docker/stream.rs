//! Continuous Docker event-stream ownership.

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::collectors::docker_protocol::{
    parse_event_line, read_event_frame, EventFrameReadError, ParsedDockerEvent,
};
use crate::collectors::WakeHandle;

const STREAM_CHANNEL_CAPACITY: usize = 256;
const BACKPRESSURE_POLL_INTERVAL: Duration = Duration::from_millis(5);
// A producer can keep a frame incomplete by returning a sequence of idle
// timeouts.  Bound the *total* age of that frame as well as each individual
// read so collector shutdown cannot wait forever on a slow/malicious peer.
const MAX_PARTIAL_FRAME_AGE: Duration = Duration::from_millis(1_500);
pub(super) const MAX_EVENTS_PER_DRAIN: usize = 128;

/// Owns the only reader thread and joins it during collector shutdown.
///
/// The production pipe has a bounded idle read, so cancellation is observed
/// within that bound even when Docker stays idle.
pub(super) struct StreamReader {
    receiver: mpsc::Receiver<ParsedDockerEvent>,
    cancelled: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    wake: WakeHandle,
}

pub(super) struct RetryTimer {
    cancel: Option<mpsc::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl RetryTimer {
    pub(super) fn start(delay: Duration, wake: WakeHandle) -> Self {
        let (cancel, receiver) = mpsc::channel();
        let failure_wake = wake.clone();
        let join = match std::thread::Builder::new()
            .name("docker-reconnect-timer".to_owned())
            .spawn(move || {
                if receiver.recv_timeout(delay) == Err(mpsc::RecvTimeoutError::Timeout) {
                    let _ = wake.request_snapshot();
                }
            }) {
            Ok(join) => Some(join),
            Err(_) => {
                // Do not strand the state machine in Reconnecting when the OS
                // refuses a helper thread. An immediate retry enters the normal
                // bounded failure/backoff path.
                let _ = failure_wake.request_snapshot();
                None
            }
        };
        Self {
            cancel: Some(cancel),
            join,
        }
    }

    pub(super) fn stop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for RetryTimer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl StreamReader {
    pub(super) fn start(mut stream: Box<dyn BufRead + Send>, wake: WakeHandle) -> Self {
        let (sender, receiver) = mpsc::sync_channel(STREAM_CHANNEL_CAPACITY);
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let finished = Arc::new(AtomicBool::new(false));
        let worker_finished = Arc::clone(&finished);
        let spawn_failed = Arc::clone(&finished);
        let worker_wake = wake.clone();
        let failure_wake = wake.clone();
        let join = match std::thread::Builder::new()
            .name("docker-event-stream".to_owned())
            .spawn(move || {
                let mut line = Vec::new();
                let mut partial_started: Option<Instant> = None;
                while !worker_cancelled.load(Ordering::Acquire) {
                    let read_started = Instant::now();
                    match read_event_frame(&mut stream, &mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let parsed = parse_event_line(&line);
                            line.clear();
                            partial_started = None;
                            match parsed {
                                Ok(Some(event)) => {
                                    // Volume and network list endpoints cannot reconstruct
                                    // removals after an overflow. Keep exactly one parsed frame
                                    // beside the bounded channel and apply backpressure until the
                                    // collector drains it. The short retry interval also keeps
                                    // cancellation and shutdown bounded.
                                    if send_with_backpressure(&sender, event, &worker_cancelled)
                                        .is_err()
                                    {
                                        return;
                                    }
                                    let _ = worker_wake.request_snapshot();
                                }
                                Ok(None) => {}
                                Err(_) => break,
                            }
                        }
                        Err(EventFrameReadError::Io(error))
                            if error.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            if !line.is_empty() {
                                // `read_event_frame` can append bytes and then
                                // time out in the same call. Start the age
                                // clock before that call so a peer that drips
                                // one byte per timeout cannot extend the
                                // lifetime indefinitely.
                                let started = partial_started.get_or_insert(read_started);
                                if started.elapsed() >= MAX_PARTIAL_FRAME_AGE {
                                    break;
                                }
                            }
                            std::thread::yield_now();
                        }
                        Err(_) => break,
                    }
                }
                worker_finished.store(true, Ordering::Release);
                let _ = worker_wake.request_snapshot();
            }) {
            Ok(join) => Some(join),
            Err(_) => {
                // A disconnected receiver plus `finished=true` makes the next
                // drain enter Reconnecting instead of remaining Available with
                // a reader that can never produce data.
                spawn_failed.store(true, Ordering::Release);
                let _ = failure_wake.request_snapshot();
                None
            }
        };
        Self {
            receiver,
            cancelled,
            finished,
            join,
            wake,
        }
    }

    pub(super) fn drain(&self) -> (Vec<ParsedDockerEvent>, bool) {
        let mut events = Vec::new();
        // A wake and its consumer can race by a few instructions. Waiting a
        // tiny bounded interval closes that handoff race without ever waiting
        // for the Docker pipe itself on the collector thread.
        match self.receiver.recv_timeout(Duration::from_millis(5)) {
            Ok(event) => events.push(event),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
        let mut queue_empty = false;
        while events.len() < MAX_EVENTS_PER_DRAIN {
            match self.receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => {
                    queue_empty = true;
                    break;
                }
            }
        }
        let disconnected = self.finished.load(Ordering::Acquire) && queue_empty;
        if events.len() == MAX_EVENTS_PER_DRAIN {
            // Producer wakes may have coalesced before this snapshot started.
            // Preserve a trailing wake even if the producer has already hit EOF,
            // so the bounded remainder does not wait for the 60-second sweep.
            let _ = self.wake.request_snapshot();
        }
        (events, disconnected)
    }

    pub(super) fn stop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn send_with_backpressure(
    sender: &mpsc::SyncSender<ParsedDockerEvent>,
    mut event: ParsedDockerEvent,
    cancelled: &AtomicBool,
) -> Result<(), ()> {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(());
        }
        match sender.try_send(event) {
            Ok(()) => return Ok(()),
            Err(mpsc::TrySendError::Full(returned)) => {
                event = returned;
                std::thread::park_timeout(BACKPRESSURE_POLL_INTERVAL);
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return Err(()),
        }
    }
}

impl Drop for StreamReader {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
