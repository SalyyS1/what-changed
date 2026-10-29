#[cfg(test)]
mod regression {
    #![allow(clippy::expect_used, clippy::panic)]

    use std::collections::VecDeque;
    use std::io::{self, BufReader, Cursor, Read};
    use std::time::Instant;

    use super::super::*;

    fn event(index: usize) -> String {
        format!(
        "{{\"Type\":\"volume\",\"Action\":\"create\",\"Actor\":{{\"ID\":\"volume-{index}\",\"Attributes\":{{\"name\":\"volume-{index}\"}}}},\"time\":1725000100}}\n"
    )
    }

    fn coalescing_wake() -> (WakeHandle, mpsc::Receiver<()>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        let wake = WakeHandle::new(move || {
            let _ = sender.try_send(());
            Ok(())
        });
        (wake, receiver)
    }

    fn await_finished(reader: &StreamReader) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !reader.finished.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(
            reader.finished.load(Ordering::Acquire),
            "reader reached EOF"
        );
    }

    #[test]
    fn full_drain_wakes_remaining_frames_after_producer_eof() {
        // This is below channel capacity but above the atomic drain cap. All
        // producer/EOF notifications have coalesced before the first snapshot.
        let body = (0..200).map(event).collect::<String>();
        let (wake, scheduled) = coalescing_wake();
        let mut reader = StreamReader::start(Box::new(Cursor::new(body.into_bytes())), wake);
        await_finished(&reader);
        // Join after EOF so even its final wake has completed before clearing the
        // coalesced slot; no producer can accidentally supply the trailing wake.
        reader.stop();

        scheduled
            .recv_timeout(Duration::from_secs(1))
            .expect("initial scheduler wake");
        let (first, disconnected) = reader.drain();
        assert_eq!(first.len(), MAX_EVENTS_PER_DRAIN);
        assert!(!disconnected);

        // No repeated manual drain until the production queue requests a new
        // scheduler turn. Without the trailing wake this waits for the 60s sweep.
        scheduled
            .recv_timeout(Duration::from_millis(100))
            .expect("bounded drain schedules its remainder even after EOF");
        let (remaining, disconnected) = reader.drain();
        assert_eq!(remaining.len(), 72);
        assert!(disconnected);
        assert_eq!(first[0].display_name, "volume-0");
        assert_eq!(remaining[71].display_name, "volume-199");
    }

    #[test]
    fn partial_frame_survives_repeated_idle_timeouts() {
        let first = event(1);
        let second = event(2);
        let body = IntermittentReader {
            segments: VecDeque::from([
                Some(first.as_bytes()[..17].to_vec()),
                None,
                None,
                Some(first.as_bytes()[17..].to_vec()),
                Some(second.into_bytes()),
            ]),
        };
        let (wake, scheduled) = coalescing_wake();
        let reader = StreamReader::start(Box::new(BufReader::new(body)), wake);
        await_finished(&reader);
        scheduled
            .recv_timeout(Duration::from_secs(1))
            .expect("completed frames schedule collection");
        let (events, disconnected) = reader.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].display_name, "volume-1");
        assert_eq!(events[1].display_name, "volume-2");
        assert!(disconnected);
    }

    #[test]
    fn timeout_cannot_reset_the_partial_frame_memory_limit() {
        use crate::collectors::docker_protocol::MAX_EVENT_LINE_BYTES;

        let body = IntermittentReader {
            segments: VecDeque::from([
                Some(vec![b'x'; MAX_EVENT_LINE_BYTES / 2]),
                None,
                Some(vec![b'x'; MAX_EVENT_LINE_BYTES / 2 + 1]),
            ]),
        };
        let mut source = BufReader::new(body);
        let mut frame = Vec::new();
        assert!(matches!(
            read_event_frame(&mut source, &mut frame),
            Err(EventFrameReadError::Io(error)) if error.kind() == io::ErrorKind::TimedOut
        ));
        assert_eq!(frame.len(), MAX_EVENT_LINE_BYTES / 2);
        assert!(matches!(
            read_event_frame(&mut source, &mut frame),
            Err(EventFrameReadError::LineTooLarge)
        ));
        assert!(frame.len() <= MAX_EVENT_LINE_BYTES);
    }

    struct IntermittentReader {
        segments: VecDeque<Option<Vec<u8>>>,
    }

    impl Read for IntermittentReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() {
                return Ok(0);
            }
            match self.segments.pop_front() {
                Some(Some(bytes)) => {
                    let count = buffer.len().min(bytes.len());
                    buffer[..count].copy_from_slice(&bytes[..count]);
                    if count < bytes.len() {
                        self.segments.push_front(Some(bytes[count..].to_vec()));
                    }
                    Ok(count)
                }
                Some(None) => Err(io::Error::new(io::ErrorKind::TimedOut, "idle")),
                None => Ok(0),
            }
        }
    }
}
