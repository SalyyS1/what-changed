//! The log sink that scrubs.
//!
//! A subscriber layer can be bypassed by a `println!`, and a formatter can be
//! replaced. The writer cannot: every byte tracing emits is handed to it. So
//! redaction lives here, at the last point before the bytes become a file, and
//! a debug statement that interpolates a raw value still cannot produce an
//! unscrubbed line on disk.
//!
//! Lines are buffered until a newline. A formatter is free to emit a line in
//! several `write` calls, and a secret split across two of them would slip
//! through a per-call scrub.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

use crate::privacy::scrub_line;

/// Wraps a sink so every complete line is scrubbed before it is written.
#[derive(Debug)]
pub struct RedactingWriter<W: Write> {
    inner: Arc<Mutex<W>>,
    pending: Vec<u8>,
}

impl<W: Write> RedactingWriter<W> {
    fn new(inner: Arc<Mutex<W>>) -> Self {
        Self {
            inner,
            pending: Vec::new(),
        }
    }

    /// Scrub and forward every complete line held in the buffer.
    fn flush_lines(&mut self) -> io::Result<()> {
        while let Some(index) = self.pending.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=index).collect();
            let text = String::from_utf8_lossy(&line);
            // The newline travels with the line, so scrubbing sees the same
            // text a reader will and the sink keeps its line structure.
            let scrubbed = scrub_line(text.trim_end_matches('\n'));
            let mut sink = self
                .inner
                .lock()
                .map_err(|_| io::Error::other("log sink poisoned by an earlier panic"))?;
            sink.write_all(scrubbed.as_bytes())?;
            sink.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        self.flush_lines()?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_lines()?;
        // A trailing fragment with no newline is held rather than written: it
        // is an unfinished line, and scrubbing half a secret is not scrubbing.
        let mut sink = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log sink poisoned by an earlier panic"))?;
        sink.flush()
    }
}

impl<W: Write> Drop for RedactingWriter<W> {
    fn drop(&mut self) {
        // A process that ends mid-line should still emit what it had, scrubbed.
        if !self.pending.is_empty() {
            self.pending.push(b'\n');
            let _ = self.flush_lines();
        }
    }
}

/// `MakeWriter` factory: hands tracing a fresh redacting writer per event, all
/// sharing one locked sink.
#[derive(Debug, Clone)]
pub struct RedactingMakeWriter<W: Write> {
    inner: Arc<Mutex<W>>,
}

impl<W: Write> RedactingMakeWriter<W> {
    /// Wrap a sink — a file appender, stdout, or a buffer in a test.
    pub fn new(inner: W) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<'a, W: Write + 'a> MakeWriter<'a> for RedactingMakeWriter<W> {
    type Writer = RedactingWriter<W>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter::new(Arc::clone(&self.inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sink that keeps what was written, so a test can read it back.
    #[derive(Debug, Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<u8>>>);

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self.0.lock() {
                Ok(mut sink) => {
                    sink.extend_from_slice(buf);
                    Ok(buf.len())
                }
                Err(_) => Err(io::Error::other("recorder poisoned")),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Recorder {
        fn text(&self) -> String {
            match self.0.lock() {
                Ok(sink) => String::from_utf8_lossy(&sink).into_owned(),
                Err(_) => String::new(),
            }
        }
    }

    #[test]
    fn scrubs_a_complete_line() -> io::Result<()> {
        let recorder = Recorder::default();
        let factory = RedactingMakeWriter::new(recorder.clone());
        let mut writer = factory.make_writer();
        writer.write_all(b"config DB_PASSWORD=hunter2 loaded\n")?;
        assert_eq!(recorder.text(), "config DB_PASSWORD=[REDACTED] loaded\n");
        Ok(())
    }

    #[test]
    fn scrubs_a_secret_split_across_two_writes() -> io::Result<()> {
        let recorder = Recorder::default();
        let factory = RedactingMakeWriter::new(recorder.clone());
        let mut writer = factory.make_writer();
        writer.write_all(b"token sk-abcdefghij")?;
        assert_eq!(
            recorder.text(),
            "",
            "an unfinished line must not reach the sink"
        );
        writer.write_all(b"klmnopqrstuvwx used\n")?;
        assert_eq!(recorder.text(), "token [REDACTED] used\n");
        Ok(())
    }

    #[test]
    fn keeps_line_boundaries_when_several_arrive_at_once() -> io::Result<()> {
        let recorder = Recorder::default();
        let factory = RedactingMakeWriter::new(recorder.clone());
        let mut writer = factory.make_writer();
        writer.write_all(b"first line\nAPI_PORT=8080\nsecond sk-abcdefghijklmnopqrstuvwx\n")?;
        assert_eq!(
            recorder.text(),
            "first line\nAPI_PORT=8080\nsecond [REDACTED]\n"
        );
        Ok(())
    }

    #[test]
    fn emits_a_trailing_fragment_on_drop() {
        let recorder = Recorder::default();
        let factory = RedactingMakeWriter::new(recorder.clone());
        {
            let mut writer = factory.make_writer();
            let _ = writer.write_all(b"cut off with sk-abcdefghijklmnopqrstuvwx");
        }
        assert_eq!(recorder.text(), "cut off with [REDACTED]\n");
    }
}
