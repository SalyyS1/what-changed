//! The writer's batch notices, throttled on their way to the webview.
//!
//! The writer already coalesces inserts into transactions; this coalesces the
//! *notifications* about them. Without it a burst of inserts becomes a burst
//! of IPC messages, each waking the renderer to decide it does not care —
//! which is how a background recorder makes the foreground stutter.
//!
//! The contract is one emit per 500 ms at most. Notices that arrive inside a
//! window are merged rather than dropped: the count adds up, the newest id
//! wins, the categories union, and the highest importance survives. So the
//! frontend learns everything that happened, once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::domain::Importance;
use crate::storage::BatchNotice;

/// The event name the frontend listens on.
pub const EVENT_SYSTEM_EVENT_ADDED: &str = "system-event-added";

/// The event a collector's health change is announced on.
pub const EVENT_COLLECTOR_STATUS_CHANGED: &str = "collector-status-changed";

/// The event a finished investigation is announced on.
pub const EVENT_ANALYSIS_COMPLETED: &str = "analysis-completed";

/// The event a pause or resume is announced on.
pub const EVENT_MONITORING_STATE_CHANGED: &str = "monitoring-state-changed";

/// A durable checkpoint was committed and checkpoint views should refresh.
pub const EVENT_CHECKPOINT_SAVED: &str = "checkpoint-saved";

/// A policy-approved OS notification was delivered. The payload contains the
/// durable event id so the shell can route a click to the matching timeline
/// row when the platform exposes notification actions.
pub const EVENT_NOTIFICATION_EVENT: &str = crate::notify::delivery::EVENT_NOTIFICATION;

/// At most one emit per this window. The number is the contract, not a tuning
/// knob: the frontend's "N new changes" pill is designed around it.
pub const THROTTLE: Duration = Duration::from_millis(500);

/// How often the loop wakes while idle to notice a shutdown.
const IDLE_POLL: Duration = Duration::from_millis(250);

/// Merge `next` into `into`, keeping what each field means.
fn merge(into: &mut BatchNotice, next: BatchNotice) {
    into.count += next.count;
    into.newest_id = into.newest_id.max(next.newest_id);
    into.highest_importance = into.highest_importance.max(next.highest_importance);
    for category in next.categories {
        if !into.categories.contains(&category) {
            into.categories.push(category);
        }
    }
    into.categories.sort_unstable();
    for event_id in next.event_ids {
        if !into.event_ids.contains(&event_id) {
            into.event_ids.push(event_id);
        }
    }
    if into.event_ids.len() > 256 {
        let keep_from = into.event_ids.len() - 256;
        into.event_ids.drain(..keep_from);
    }
}

/// Anything that can deliver a notice to the frontend.
///
/// A trait rather than an `AppHandle` so the coalescing can be tested without
/// a Tauri runtime — the timing rule is the part worth testing, and it has
/// nothing to do with Tauri.
pub trait NoticeSink: Send + 'static {
    /// Deliver one coalesced notice.
    fn emit(&self, notice: &BatchNotice);
}

impl<R: tauri::Runtime> NoticeSink for tauri::AppHandle<R> {
    fn emit(&self, notice: &BatchNotice) {
        use tauri::Emitter;
        if let Err(error) = Emitter::emit(self, EVENT_SYSTEM_EVENT_ADDED, notice) {
            tracing::debug!(%error, "a batch notice could not be delivered to the window");
        }
        crate::notify::delivery::deliver_batch(self, &notice.event_ids);
    }
}

/// The coalescing thread. Dropping it stops the thread.
pub struct EventBus {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EventBus {
    /// Consume `notices` and emit at most one message per [`THROTTLE`].
    pub fn spawn(notices: Receiver<BatchNotice>, sink: impl NoticeSink) -> EventBus {
        crate::notify::delivery::initialize();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("event-bus".to_owned())
            .spawn(move || run_loop(notices, sink, &flag))
            .ok();
        EventBus { stop, thread }
    }
}

impl Drop for EventBus {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Wait for a notice, coalesce everything that arrives within the window, emit
/// once, repeat.
fn run_loop(notices: Receiver<BatchNotice>, sink: impl NoticeSink, stop: &AtomicBool) {
    let mut pending: Option<BatchNotice> = None;
    let mut window_opened: Option<Instant> = None;

    loop {
        if stop.load(Ordering::Relaxed) {
            // Deliver what was collected before stopping. The claim above is
            // that notices are merged rather than dropped, and a shutdown
            // inside an open window is the one place that could be untrue.
            if let Some(notice) = pending.take() {
                sink.emit(&notice);
            }
            break;
        }
        // While a window is open the loop waits only for what is left of it,
        // so an emit is never late by more than one scheduling slice.
        let wait = match window_opened {
            Some(opened) => THROTTLE.saturating_sub(opened.elapsed()),
            None => IDLE_POLL,
        };
        match notices.recv_timeout(wait) {
            Ok(notice) => {
                match pending.as_mut() {
                    Some(existing) => merge(existing, notice),
                    None => pending = Some(notice),
                }
                if window_opened.is_none() {
                    window_opened = Some(Instant::now());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            // The database closed. Whatever was collected still deserves to
            // be delivered before the thread ends.
            Err(RecvTimeoutError::Disconnected) => {
                if let Some(notice) = pending.take() {
                    sink.emit(&notice);
                }
                break;
            }
        }

        let due = window_opened.is_some_and(|opened| opened.elapsed() >= THROTTLE);
        if due {
            if let Some(notice) = pending.take() {
                sink.emit(&notice);
            }
            window_opened = None;
        }
    }
}

/// A notice for `count` rows at `newest_id`, for tests and for callers that
/// synthesize one.
pub fn notice(count: usize, newest_id: i64, importance: Importance) -> BatchNotice {
    BatchNotice {
        count,
        newest_id,
        categories: Vec::new(),
        highest_importance: importance,
        event_ids: if count == 0 {
            Vec::new()
        } else {
            vec![newest_id]
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::sync::mpsc::sync_channel;
    use std::sync::Mutex;

    use super::*;
    use crate::domain::Category;

    #[derive(Default)]
    struct Recorder(Arc<Mutex<Vec<BatchNotice>>>);

    impl NoticeSink for Recorder {
        fn emit(&self, notice: &BatchNotice) {
            self.0
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(notice.clone());
        }
    }

    #[test]
    fn a_thousand_notices_in_a_hundred_milliseconds_produce_at_most_two_emits() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = sync_channel(4096);
        let bus = EventBus::spawn(rx, Recorder(Arc::clone(&seen)));

        let started = Instant::now();
        for i in 0..1_000 {
            tx.send(notice(1, i, Importance::Informational)).unwrap();
        }
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "the sender itself must not be the slow part"
        );

        // Two windows' worth of waiting: enough for one emit to land and for a
        // second to be provably absent.
        std::thread::sleep(THROTTLE * 2 + Duration::from_millis(200));
        drop(tx);
        drop(bus);

        let emitted = seen.lock().unwrap_or_else(|p| p.into_inner());
        assert!(
            emitted.len() <= 2,
            "1,000 notices produced {} emits; the contract is at most one per {:?}",
            emitted.len(),
            THROTTLE
        );
        assert!(!emitted.is_empty(), "the notices must not be swallowed");
        let total: usize = emitted.iter().map(|n| n.count).sum();
        assert_eq!(total, 1_000, "every row is accounted for across the emits");
        let newest = emitted.iter().map(|n| n.newest_id).max().unwrap();
        assert_eq!(newest, 999);
    }

    #[test]
    fn merging_keeps_the_meaning_of_every_field() {
        let mut first = BatchNotice {
            count: 3,
            newest_id: 10,
            categories: vec![Category::Service],
            highest_importance: Importance::Notable,
            event_ids: vec![8, 9, 10],
        };
        merge(
            &mut first,
            BatchNotice {
                count: 4,
                newest_id: 7,
                categories: vec![Category::Docker, Category::Service],
                highest_importance: Importance::Critical,
                event_ids: vec![7, 10],
            },
        );
        assert_eq!(first.count, 7, "counts add");
        assert_eq!(
            first.newest_id, 10,
            "the newest id is the highest, not the last"
        );
        assert_eq!(first.highest_importance, Importance::Critical);
        // Declaration order, which is the brief's order and the same order the
        // writer's own `publish` produces — so a merged notice and an
        // unmerged one list their categories the same way.
        assert_eq!(first.categories, vec![Category::Service, Category::Docker]);
        assert_eq!(first.event_ids, vec![8, 9, 10, 7]);
        assert!(
            Category::Service < Category::Docker,
            "the ordering above is the enum's, not the alphabet's"
        );
    }

    #[test]
    fn merging_bounds_event_ids_to_the_newest_window() {
        let mut first = notice(256, 255, Importance::Informational);
        first.event_ids = (0..256).collect();
        let mut next = notice(256, 511, Importance::Informational);
        next.event_ids = (256..512).collect();
        merge(&mut first, next);
        assert_eq!(first.event_ids.len(), 256);
        assert_eq!(first.event_ids.first(), Some(&256));
        assert_eq!(first.event_ids.last(), Some(&511));
    }

    #[test]
    fn a_notice_collected_inside_an_open_window_survives_shutdown() {
        // The module claims notices are merged rather than dropped. Stopping
        // mid-window is the one path where that could be false.
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = sync_channel(8);
        let bus = EventBus::spawn(rx, Recorder(Arc::clone(&seen)));
        tx.send(notice(3, 7, Importance::Notable)).unwrap();
        // Well inside the 500 ms window, so `pending` is still holding it.
        std::thread::sleep(Duration::from_millis(60));
        drop(bus);

        let emitted = seen.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(emitted.len(), 1, "the collected notice must be delivered");
        assert_eq!(emitted[0].count, 3);
        assert_eq!(emitted[0].newest_id, 7);
    }

    #[test]
    fn a_single_notice_is_still_delivered() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = sync_channel(8);
        let bus = EventBus::spawn(rx, Recorder(Arc::clone(&seen)));
        tx.send(notice(1, 42, Importance::Critical)).unwrap();
        std::thread::sleep(THROTTLE + Duration::from_millis(200));
        drop(tx);
        drop(bus);
        let emitted = seen.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].newest_id, 42);
    }
}
