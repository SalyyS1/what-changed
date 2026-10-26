//! Non-elevated Windows Event Log subscriptions.
//!
//! Each callback renders one event to bounded XML, tries one bounded channel
//! send and requests a collector wake. It never parses, sanitizes or writes a
//! database row on the Event Log service callback thread.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows::Win32::System::EventLog::{
    EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection, EvtRender,
    EvtRenderEventXml, EvtSubscribe, EvtSubscribeActionDeliver, EvtSubscribeActionError,
    EvtSubscribeToFutureEvents, EVT_HANDLE, EVT_SUBSCRIBE_NOTIFY_ACTION,
};

use crate::collectors::eventlog::{EventLogSource, RenderedEvent, SUBSCRIPTIONS};
use crate::collectors::{CancelToken, WakeHandle};
use crate::error::{AppError, AppResult};

// Keep callback memory bounded even when the collector thread is stalled.
// 512 rendered records at 64 KiB each cap the queued XML near 32 MiB while
// admitting the Phase 13 five-hundred-record storm acceptance burst.
const CALLBACK_QUEUE_CAPACITY: usize = 512;
const MAX_RENDERED_XML_BYTES: u32 = 64 * 1024;

pub struct WindowsEventLogSource {
    subscriptions: Vec<Subscription>,
    receiver: Option<Receiver<RenderedEvent>>,
    dropped: Arc<AtomicU64>,
    source_errors: Arc<AtomicU64>,
    wake: Option<WakeHandle>,
}

impl Default for WindowsEventLogSource {
    fn default() -> Self {
        Self {
            subscriptions: Vec::new(),
            receiver: None,
            dropped: Arc::new(AtomicU64::new(0)),
            source_errors: Arc::new(AtomicU64::new(0)),
            wake: None,
        }
    }
}

impl WindowsEventLogSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dropped_callback_events(&self) -> u64 {
        self.dropped.load(Ordering::Acquire)
    }
}

impl EventLogSource for WindowsEventLogSource {
    fn start(&mut self, wake: WakeHandle, cancel: CancelToken) -> AppResult<()> {
        self.stop();
        self.dropped.store(0, Ordering::Release);
        self.source_errors.store(0, Ordering::Release);
        let (sender, receiver) = mpsc::sync_channel(CALLBACK_QUEUE_CAPACITY);
        let mut subscriptions = Vec::with_capacity(SUBSCRIPTIONS.len());
        for (channel, query) in SUBSCRIPTIONS {
            subscriptions.push(Subscription::open(
                channel,
                query,
                sender.clone(),
                wake.clone(),
                cancel.clone(),
                Arc::clone(&self.dropped),
                Arc::clone(&self.source_errors),
            )?);
        }
        // A per-user app starts after Windows emitted the current boot's 6005
        // record. Subscribe first (so there is no query/subscribe gap), then
        // backfill only that one boundary. Ordinary downtime log records stay
        // intentionally unrecovered.
        let boot_boundary = match latest_boot_boundary_xml() {
            Ok(boundary) => boundary,
            Err(error) => {
                tracing::warn!(%error, "current boot boundary backfill unavailable; live Event Log subscriptions remain active");
                None
            }
        };
        if let Some(xml) = boot_boundary {
            match sender.try_send(RenderedEvent {
                channel: "System".to_owned(),
                xml,
            }) {
                Ok(()) => {
                    let _ = wake.request_snapshot();
                }
                Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    let _ = wake.request_snapshot();
                }
            }
        }
        self.wake = Some(wake);
        self.receiver = Some(receiver);
        self.subscriptions = subscriptions;
        Ok(())
    }

    fn drain(&mut self) -> AppResult<Vec<RenderedEvent>> {
        let queued = self
            .receiver
            .as_ref()
            .map(|receiver| receiver.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        if !queued.is_empty() {
            if self.source_errors.load(Ordering::Acquire) > 0
                || self.dropped.load(Ordering::Acquire) > 0
            {
                if let Some(wake) = &self.wake {
                    let _ = wake.request_snapshot();
                }
            }
            return Ok(queued);
        }
        let errors = self.source_errors.swap(0, Ordering::AcqRel);
        if errors > 0 {
            return Err(AppError::Platform {
                operation: "receive Windows Event Log subscription events",
                reason: format!("the native subscription reported {errors} error callback(s)"),
            });
        }
        let dropped = self.dropped.swap(0, Ordering::AcqRel);
        if dropped > 0 {
            return Err(AppError::Platform {
                operation: "receive Windows Event Log subscription events",
                reason: format!(
                    "the bounded callback path dropped {dropped} event(s); collector restart requested"
                ),
            });
        }
        Ok(Vec::new())
    }

    fn stop(&mut self) {
        // Subscription::drop closes each handle and removes its opaque registry
        // key. An in-flight callback owns an Arc; a later callback sees no key.
        self.subscriptions.clear();
        self.receiver = None;
        self.wake = None;
    }
}

fn latest_boot_boundary_xml() -> AppResult<Option<String>> {
    let channel = wide("System");
    let query = wide("*[System[EventID=6005]]");
    let result = unsafe {
        EvtQuery(
            None,
            PCWSTR(channel.as_ptr()),
            PCWSTR(query.as_ptr()),
            EvtQueryChannelPath.0 | EvtQueryReverseDirection.0,
        )
    }
    .map_err(|source| platform_error("query the latest Windows boot boundary", source))?;
    let result = OwnedEvtHandle(result);
    let mut handles = [0isize; 1];
    let mut returned = 0u32;
    match unsafe { EvtNext(result.0, &mut handles, 0, 0, &mut returned) } {
        Ok(()) if returned == 1 => {
            let event = OwnedEvtHandle(EVT_HANDLE(handles[0]));
            render_xml(event.0)
                .map(Some)
                .ok_or_else(|| AppError::Platform {
                    operation: "render the latest Windows boot boundary",
                    reason: "the event XML was unavailable or too large".to_owned(),
                })
        }
        Ok(()) => Ok(None),
        Err(source) if source.code() == ERROR_NO_MORE_ITEMS.to_hresult() => Ok(None),
        Err(source) => Err(platform_error(
            "read the latest Windows boot boundary",
            source,
        )),
    }
}

struct OwnedEvtHandle(EVT_HANDLE);

impl Drop for OwnedEvtHandle {
    fn drop(&mut self) {
        let _ = unsafe { EvtClose(self.0) };
    }
}

impl Drop for WindowsEventLogSource {
    fn drop(&mut self) {
        self.stop();
    }
}

struct CallbackContext {
    channel: &'static str,
    sender: SyncSender<RenderedEvent>,
    wake: WakeHandle,
    cancel: CancelToken,
    dropped: Arc<AtomicU64>,
    source_errors: Arc<AtomicU64>,
}

struct Subscription {
    handle: EVT_HANDLE,
    context_id: usize,
}

// Native code receives an opaque integer key, never a Rust pointer. A callback
// clones the Arc from the registry before using it, so EvtClose can race with
// a callback without either a use-after-free or a callback-rundown assumption.
unsafe impl Send for Subscription {}

static CALLBACK_CONTEXTS: OnceLock<Mutex<HashMap<usize, Arc<CallbackContext>>>> = OnceLock::new();
static NEXT_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn callback_contexts() -> &'static Mutex<HashMap<usize, Arc<CallbackContext>>> {
    CALLBACK_CONTEXTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_callback_context(context: CallbackContext) -> usize {
    loop {
        let context_id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
        if context_id == 0 {
            continue;
        }
        callback_contexts()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(context_id, Arc::new(context));
        return context_id;
    }
}

fn remove_callback_context(context_id: usize) {
    callback_contexts()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&context_id);
}

impl Subscription {
    fn open(
        channel: &'static str,
        query: &'static str,
        sender: SyncSender<RenderedEvent>,
        wake: WakeHandle,
        cancel: CancelToken,
        dropped: Arc<AtomicU64>,
        source_errors: Arc<AtomicU64>,
    ) -> AppResult<Self> {
        let context_id = register_callback_context(CallbackContext {
            channel,
            sender,
            wake,
            cancel,
            dropped,
            source_errors,
        });
        let channel_wide = wide(channel);
        let query_wide = wide(query);
        let result = unsafe {
            EvtSubscribe(
                None,
                None,
                PCWSTR(channel_wide.as_ptr()),
                PCWSTR(query_wide.as_ptr()),
                None,
                Some(context_id as *const c_void),
                Some(callback),
                EvtSubscribeToFutureEvents.0,
            )
        };
        match result {
            Ok(handle) => Ok(Self { handle, context_id }),
            Err(source) => {
                remove_callback_context(context_id);
                Err(platform_error(
                    "subscribe to a Windows Event Log channel",
                    source,
                ))
            }
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = unsafe { EvtClose(self.handle) };
        // If EvtClose fails, a later native callback still carries only this
        // integer key. Registry removal makes that callback a harmless no-op.
        // A callback already in flight owns an Arc until it returns.
        remove_callback_context(self.context_id);
    }
}

unsafe extern "system" fn callback(
    action: EVT_SUBSCRIBE_NOTIFY_ACTION,
    user_context: *const c_void,
    event: EVT_HANDLE,
) -> u32 {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if user_context.is_null() {
            return;
        }
        let context_id = user_context as usize;
        let Some(context) = callback_contexts()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&context_id)
            .cloned()
        else {
            return;
        };
        if action == EvtSubscribeActionError {
            context.source_errors.fetch_add(1, Ordering::Relaxed);
            let _ = context.wake.request_snapshot();
            return;
        }
        if action != EvtSubscribeActionDeliver {
            return;
        }
        if context.cancel.is_cancelled() {
            return;
        }
        let Some(xml) = render_xml(event) else {
            context.dropped.fetch_add(1, Ordering::Relaxed);
            let _ = context.wake.request_snapshot();
            return;
        };
        match context.sender.try_send(RenderedEvent {
            channel: context.channel.to_owned(),
            xml,
        }) {
            Ok(()) => {
                let _ = context.wake.request_snapshot();
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                context.dropped.fetch_add(1, Ordering::Relaxed);
                let _ = context.wake.request_snapshot();
            }
        }
    }));
    ERROR_SUCCESS.0
}

fn render_xml(event: EVT_HANDLE) -> Option<String> {
    let mut required = 0u32;
    let mut properties = 0u32;
    let measured = unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            0,
            None,
            &mut required,
            &mut properties,
        )
    };
    if let Err(source) = measured {
        if source.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult() {
            return None;
        }
    }
    if required == 0 || required > MAX_RENDERED_XML_BYTES || required % 2 != 0 {
        return None;
    }
    let words = usize::try_from(required / 2).ok()?;
    let mut buffer = vec![0u16; words];
    unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            required,
            Some(buffer.as_mut_ptr().cast()),
            &mut required,
            &mut properties,
        )
        .ok()?;
    }
    let end = buffer
        .iter()
        .position(|word| *word == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end]).ok()
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn platform_error(operation: &'static str, source: windows::core::Error) -> AppError {
    AppError::Platform {
        operation,
        reason: format!("{:#010x}", source.code().0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_failure_wakes_the_collector_and_surfaces_a_health_error() {
        let wake_count = Arc::new(AtomicU64::new(0));
        let counted = Arc::clone(&wake_count);
        let wake = WakeHandle::new(move || {
            counted.fetch_add(1, Ordering::AcqRel);
            Ok(())
        });
        let (sender, _receiver) = mpsc::sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let context_id = register_callback_context(CallbackContext {
            channel: "Application",
            sender,
            wake,
            cancel: CancelToken::default(),
            dropped: Arc::clone(&dropped),
            source_errors: Arc::new(AtomicU64::new(0)),
        });
        unsafe {
            callback(
                EvtSubscribeActionDeliver,
                context_id as *const c_void,
                EVT_HANDLE::default(),
            );
        }
        remove_callback_context(context_id);
        assert_eq!(wake_count.load(Ordering::Acquire), 1);
        assert_eq!(dropped.load(Ordering::Acquire), 1);

        let mut source = WindowsEventLogSource::default();
        source.dropped.store(1, Ordering::Release);
        assert!(source.drain().is_err());
    }
}
