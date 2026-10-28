#![cfg(test)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::time::Duration;

#[derive(Clone, Default)]
struct Queue(Arc<Mutex<Vec<Vec<u8>>>>);
impl Write for Queue {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().push(bytes.to_vec());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<u8>>>);
impl Write for Recorder {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn frame(epoch: u64, bytes: &[u8]) -> Vec<u8> {
    let mut frame = MAGIC.to_vec();
    frame.extend_from_slice(&epoch.to_le_bytes());
    frame.extend_from_slice(bytes);
    frame
}

#[test]
fn delayed_queued_paused_and_partially_formatted_lines_cannot_revive() {
    let epoch = Arc::new(LogEpoch::default());
    let queue = Queue::default();
    let factory = EpochMakeWriter::new(queue.clone(), Arc::clone(&epoch));
    let mut fragment = factory.make_writer();
    fragment.write_all(b"old partial private ").unwrap();
    factory
        .make_writer()
        .write_all(b"old queued private\n")
        .unwrap();
    let barrier = epoch.pause().unwrap();
    factory
        .make_writer()
        .write_all(b"private during purge\n")
        .unwrap();
    drop(barrier);
    fragment.write_all(b"fragment\n").unwrap();
    factory
        .make_writer()
        .write_all(b"fresh DB_PASSWORD=hunter2\n")
        .unwrap();
    let recorder = Recorder::default();
    let mut sink = EpochSink::new(recorder.clone(), Arc::clone(&epoch));
    for frame in queue.0.lock().unwrap().drain(..) {
        sink.write_all(&frame).unwrap();
    }
    assert_eq!(
        String::from_utf8(recorder.0.lock().unwrap().clone()).unwrap(),
        "fresh DB_PASSWORD=[REDACTED]\n"
    );
    assert_eq!(epoch.state.lock().unwrap().discarded_frames, 3);
    assert!(!epoch.state.lock().unwrap().paused);
}

#[test]
fn accepted_lines_flush_inner_buffers_before_the_privacy_barrier() {
    let epoch = Arc::new(LogEpoch::default());
    let recorder = Recorder::default();
    let buffered = io::BufWriter::with_capacity(4096, recorder.clone());
    let mut sink = EpochSink::new(buffered, Arc::clone(&epoch));
    sink.write_all(&frame(0, b"old line\n")).unwrap();
    assert_eq!(&*recorder.0.lock().unwrap(), b"old line\n");
    let barrier = epoch.pause().unwrap();
    recorder.0.lock().unwrap().clear();
    sink.write_all(&frame(0, b"late old line\n")).unwrap();
    sink.flush().unwrap();
    drop(barrier);
    sink.flush().unwrap();
    assert!(recorder.0.lock().unwrap().is_empty());
    sink.write_all(&frame(2, b"new line\n")).unwrap();
    assert_eq!(&*recorder.0.lock().unwrap(), b"new line\n");
}

#[test]
fn real_nonblocking_file_queue_remains_redacted_and_live_after_truncation() {
    let dir = crate::testing::temp_dir("log-epoch-file-queue");
    let path = dir.join("log.txt");
    let file = std::fs::OpenOptions::new()
        .append(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let epoch = Arc::new(LogEpoch::default());
    let (queue, worker) = tracing_appender::non_blocking::NonBlockingBuilder::default()
        .buffered_lines_limit(16)
        .lossy(false)
        .finish(EpochSink::new(file, Arc::clone(&epoch)));
    let factory = EpochMakeWriter::new(queue, Arc::clone(&epoch));
    let mut delayed = factory.make_writer();
    delayed.write_all(b"old unfinished ").unwrap();
    factory.make_writer().write_all(b"old queued\n").unwrap();
    let barrier = epoch.pause().unwrap();
    factory
        .make_writer()
        .write_all(b"paused private\n")
        .unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap()
        .sync_all()
        .unwrap();
    drop(barrier);
    delayed.write_all(b"line\n").unwrap();
    factory
        .make_writer()
        .write_all(b"ERROR fresh DB_PASSWORD=hunter2\n")
        .unwrap();
    drop(delayed);
    drop(factory);
    drop(worker);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "ERROR fresh DB_PASSWORD=[REDACTED]\n"
    );
}

#[test]
fn pause_waits_for_the_in_flight_file_write_and_flush() {
    struct BlockingSink {
        entered: std::sync::mpsc::SyncSender<()>,
        release: std::sync::mpsc::Receiver<()>,
    }
    impl Write for BlockingSink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let epoch = Arc::new(LogEpoch::default());
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let mut sink = EpochSink::new(
        BlockingSink {
            entered: entered_tx,
            release: release_rx,
        },
        Arc::clone(&epoch),
    );
    let writer = std::thread::spawn(move || sink.write_all(&frame(0, b"line\n")));
    entered_rx.recv().unwrap();
    let (barrier_tx, barrier_rx) = sync_channel(1);
    let pausing = Arc::clone(&epoch);
    let pauser = std::thread::spawn(move || barrier_tx.send(pausing.pause().unwrap()).unwrap());
    let early = barrier_rx.recv_timeout(Duration::from_millis(10));
    release_tx.send(()).unwrap();
    writer.join().unwrap().unwrap();
    assert!(
        early.is_err(),
        "the log barrier overtook an active file write"
    );
    let barrier = barrier_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    pauser.join().unwrap();
    assert!(epoch.state.lock().unwrap().paused);
    drop(barrier);
}

#[test]
fn a_flush_failure_blocks_purge_until_a_successful_flush() {
    struct Fallible(Arc<AtomicBool>);
    impl Write for Fallible {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.0.load(Ordering::Acquire) {
                Err(io::Error::other("injected file flush failure"))
            } else {
                Ok(())
            }
        }
    }
    let failing = Arc::new(AtomicBool::new(true));
    let epoch = Arc::new(LogEpoch::default());
    let mut sink = EpochSink::new(Fallible(Arc::clone(&failing)), Arc::clone(&epoch));
    assert!(sink.write_all(&frame(0, b"line\n")).is_err());
    assert!(epoch.pause().is_err());
    assert!(!epoch.state.lock().unwrap().paused);
    failing.store(false, Ordering::Release);
    sink.flush().unwrap();
    drop(epoch.pause().unwrap());
    sink.write_all(&frame(2, b"fresh line\n")).unwrap();
}

#[test]
fn internal_frame_headers_never_reach_the_file() {
    let recorder = Recorder::default();
    let mut sink = EpochSink::new(recorder.clone(), Arc::new(LogEpoch::default()));
    assert!(sink.write_all(b"unframed").is_err());
    assert!(recorder.0.lock().unwrap().is_empty());
    sink.write_all(&frame(0, b"normal\n")).unwrap();
    assert_eq!(&*recorder.0.lock().unwrap(), b"normal\n");
}
