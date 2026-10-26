//! Bounded HTTP/1.1 transport over Docker Desktop's Windows named pipe.
//!
//! Docker requires a duplex handle even though this product only issues GETs.
//! Each request owns a fresh overlapped handle: dropping a failed response
//! closes that handle, and a reconnect can never inherit pending I/O from it.

use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use crate::error::{AppError, AppResult};
use crate::platform::traits::DockerTransport;

const DOCKER_PIPE: &str = r"\\.\pipe\docker_engine";
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(5);
// The current collector trait has no cancellation token. Keep an idle read
// below the runtime's two-second shutdown fence; TimedOut is an Available/Noop,
// not a reconnect or log transition.
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(1);
const CANCEL_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 1024 * 1024;
const READ_BUFFER_BYTES: usize = 16 * 1024;

/// Production Docker transport. Cloneable configuration contains no live
/// handle; every call obtains and exclusively owns a new connection.
#[derive(Debug, Clone)]
pub struct WindowsNamedPipeTransport {
    pipe_name: String,
    connect_timeout: Duration,
    io_timeout: Duration,
    response_limit: usize,
}

impl Default for WindowsNamedPipeTransport {
    fn default() -> Self {
        Self::docker()
    }
}

impl WindowsNamedPipeTransport {
    /// Transport for the one production endpoint Docker Desktop exposes.
    pub fn docker() -> Self {
        Self {
            pipe_name: DOCKER_PIPE.to_owned(),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            io_timeout: DEFAULT_IO_TIMEOUT,
            response_limit: MAX_RESPONSE_BYTES,
        }
    }

    fn execute(
        &self,
        path: &str,
        stream_idle_timeout: Option<Duration>,
    ) -> AppResult<HttpBody<BufReader<PipeIo>>> {
        validate_path(path).map_err(transport_error)?;
        // A finite request receives one budget. Repeated short reads must not
        // turn that budget into an unlimited trickle timeout.
        let deadline = AbsoluteDeadline::after(self.io_timeout);
        let mut pipe = PipeIo::open(&self.pipe_name, self.connect_timeout, deadline)
            .map_err(transport_error)?;
        let request = format!("GET {path} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n");
        pipe.write_all(request.as_bytes())
            .map_err(transport_error)?;
        pipe.flush().map_err(transport_error)?;
        // Keep the raw pipe buffered below both the header and chunk parsers.
        // Their byte-oriented delimiter reads then consume memory, not one
        // overlapped kernel operation per byte.
        let buffered = BufReader::with_capacity(READ_BUFFER_BYTES, pipe);
        let mut response = parse_response(buffered).map_err(transport_error)?;
        ensure_success_status(response.status)?;
        // Event streams are intentionally different: after the complete HTTP
        // handshake, each frame read gets a renewable idle window so a quiet
        // daemon remains connected while shutdown stays bounded.
        if let Some(timeout) = stream_idle_timeout {
            response.body.set_idle_timeout(timeout);
        }
        Ok(response.body)
    }
}

impl DockerTransport for WindowsNamedPipeTransport {
    fn get(&mut self, path: &str) -> AppResult<Vec<u8>> {
        let mut body = self.execute(path, None)?;
        let mut bytes = Vec::new();
        let limit = u64::try_from(self.response_limit).unwrap_or(u64::MAX);
        body.by_ref()
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(transport_error)?;
        if bytes.len() > self.response_limit {
            return Err(transport_error(io::Error::new(
                io::ErrorKind::InvalidData,
                "Docker response exceeded the configured limit",
            )));
        }
        Ok(bytes)
    }

    fn stream(&mut self, path: &str) -> AppResult<Box<dyn BufRead + Send>> {
        // An idle event stream is normal. The collector treats this bounded
        // timeout as an idle Noop and keeps the same connection/state. One
        // second also keeps shutdown inside the runtime's two-second fence.
        let body = self.execute(path, Some(STREAM_IDLE_TIMEOUT))?;
        Ok(Box::new(BufReader::with_capacity(READ_BUFFER_BYTES, body)))
    }
}

fn transport_error(source: io::Error) -> AppError {
    AppError::Platform {
        operation: "communicate with the Docker named pipe",
        // All errors created in this module use fixed diagnostic text and the
        // pipe name/request target are never interpolated. HTTP statuses use a
        // separate typed AppError and never include the untrusted body.
        reason: source.to_string(),
    }
}

fn ensure_success_status(status: u16) -> AppResult<()> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(AppError::DockerHttpStatus { status })
    }
}

fn validate_path(path: &str) -> io::Result<()> {
    if !path.starts_with('/') || path.bytes().any(|byte| byte <= 0x20 || byte == 0x7f) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Docker request path is not a single HTTP request target",
        ));
    }
    Ok(())
}

struct PipeIo {
    handle: OwnedHandle,
    timeout: IoTimeout,
    health: PipeHealth,
}

impl PipeIo {
    fn open(
        pipe_name: &str,
        connect_timeout: Duration,
        deadline: AbsoluteDeadline,
    ) -> io::Result<Self> {
        let wide = wide(pipe_name);
        let wait_ms = deadline.wait_ms_capped(connect_timeout, Instant::now())?;
        if !unsafe { WaitNamedPipeW(PCWSTR(wide.as_ptr()), wait_ms) }.as_bool() {
            return Err(io::Error::last_os_error());
        }
        deadline.ensure_remaining(Instant::now())?;
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .map_err(io::Error::other)?;
        Ok(Self {
            handle: OwnedHandle(handle),
            timeout: IoTimeout::Deadline(deadline),
            health: PipeHealth::default(),
        })
    }

    fn overlapped<F>(&self, operation: F) -> Result<usize, OverlappedFailure>
    where
        F: FnOnce(*mut OVERLAPPED) -> windows::core::Result<()>,
    {
        self.health.ensure_usable().map_err(OverlappedFailure::Io)?;
        let event = OwnedHandle(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(io::Error::other)
                .map_err(OverlappedFailure::Io)?,
        );
        // Heap allocation keeps the OVERLAPPED address stable. In the
        // pathological case where Windows neither completes nor acknowledges
        // cancellation within the drain bound, this allocation is leaked so
        // the kernel can never write into freed state.
        let mut pending = Box::new(PendingOperation {
            overlapped: OVERLAPPED {
                hEvent: event.get(),
                ..Default::default()
            },
            event,
        });
        match operation(&mut pending.overlapped) {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_IO_PENDING.0) => {}
            Err(error) => return Err(OverlappedFailure::Io(io::Error::other(error))),
        }

        let timeout_ms = match self.timeout.wait_ms(Instant::now()) {
            Ok(timeout_ms) => timeout_ms,
            Err(error) => return self.cancel_and_drain(pending, error),
        };
        let wait = unsafe { WaitForSingleObject(pending.event.get(), timeout_ms) };
        if wait == WAIT_TIMEOUT {
            return self.cancel_and_drain(
                pending,
                io::Error::new(io::ErrorKind::TimedOut, "Docker named-pipe I/O timed out"),
            );
        }
        if wait != WAIT_OBJECT_0 {
            return self.cancel_and_drain(pending, io::Error::last_os_error());
        }
        let mut transferred = 0;
        unsafe {
            GetOverlappedResult(
                self.handle.get(),
                &pending.overlapped,
                &mut transferred,
                false,
            )
        }
        .map_err(io::Error::other)
        .map_err(OverlappedFailure::Io)?;
        usize::try_from(transferred)
            .map_err(io::Error::other)
            .map_err(OverlappedFailure::Io)
    }

    fn cancel_and_drain(
        &self,
        pending: Box<PendingOperation>,
        error: io::Error,
    ) -> Result<usize, OverlappedFailure> {
        let _ = unsafe { CancelIoEx(self.handle.get(), Some(&pending.overlapped)) };
        let drained = unsafe {
            WaitForSingleObject(pending.event.get(), duration_ms_ceil(CANCEL_DRAIN_TIMEOUT))
        };
        if drained != WAIT_OBJECT_0 {
            // Callers also leak their operation buffer for this variant. The
            // pipe handle may close, but every address the kernel was given
            // remains valid for the rest of the process. Poison the connection
            // before returning: an undrained operation makes the byte-stream
            // position unknowable, so even a later successful read on this
            // handle would be unsafe to interpret as the next Docker frame.
            Box::leak(pending);
            self.health.poison();
            return Err(OverlappedFailure::StateLeaked(terminal_connection_error()));
        }
        let mut transferred = 0;
        match unsafe {
            GetOverlappedResult(
                self.handle.get(),
                &pending.overlapped,
                &mut transferred,
                false,
            )
        } {
            // Cancellation can lose the race to normal completion. Those
            // bytes have already advanced the pipe cursor and must be returned
            // to the parser instead of being silently discarded.
            Ok(()) => usize::try_from(transferred)
                .map_err(io::Error::other)
                .map_err(OverlappedFailure::Io),
            Err(cancelled)
                if cancelled.code() == HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) =>
            {
                Err(OverlappedFailure::Io(error))
            }
            Err(failure) => Err(OverlappedFailure::Io(io::Error::other(failure))),
        }
    }

    fn set_idle_timeout(&mut self, timeout: Duration) {
        self.timeout = IoTimeout::Idle(timeout);
    }
}

impl Read for PipeIo {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let handle = self.handle.get();
        let mut owned = vec![0; buffer.len()];
        match self.overlapped(|overlapped| unsafe {
            ReadFile(handle, Some(&mut owned), None, Some(overlapped))
        }) {
            Ok(read) => {
                buffer[..read].copy_from_slice(&owned[..read]);
                Ok(read)
            }
            Err(OverlappedFailure::Io(error)) => Err(error),
            Err(OverlappedFailure::StateLeaked(error)) => {
                std::mem::forget(owned);
                Err(error)
            }
        }
    }
}

impl Write for PipeIo {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let handle = self.handle.get();
        let owned = buffer.to_vec();
        match self.overlapped(|overlapped| unsafe {
            WriteFile(handle, Some(&owned), None, Some(overlapped))
        }) {
            Ok(written) => Ok(written),
            Err(OverlappedFailure::Io(error)) => Err(error),
            Err(OverlappedFailure::StateLeaked(error)) => {
                std::mem::forget(owned);
                Err(error)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct AbsoluteDeadline {
    at: Instant,
}

impl AbsoluteDeadline {
    fn after(duration: Duration) -> Self {
        Self {
            at: Instant::now()
                .checked_add(duration)
                .unwrap_or_else(Instant::now),
        }
    }

    fn remaining(self, now: Instant) -> io::Result<Duration> {
        self.at
            .checked_duration_since(now)
            .filter(|value| !value.is_zero())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Docker named-pipe request timed out",
                )
            })
    }

    fn ensure_remaining(self, now: Instant) -> io::Result<()> {
        self.remaining(now).map(|_| ())
    }

    fn wait_ms(self, now: Instant) -> io::Result<u32> {
        self.remaining(now).map(duration_ms_ceil)
    }

    fn wait_ms_capped(self, cap: Duration, now: Instant) -> io::Result<u32> {
        self.remaining(now)
            .map(|remaining| duration_ms_ceil(remaining.min(cap)))
    }
}

#[derive(Debug, Clone, Copy)]
enum IoTimeout {
    Deadline(AbsoluteDeadline),
    Idle(Duration),
}

impl IoTimeout {
    fn wait_ms(self, now: Instant) -> io::Result<u32> {
        match self {
            Self::Deadline(deadline) => deadline.wait_ms(now),
            Self::Idle(timeout) => Ok(duration_ms_ceil(timeout)),
        }
    }
}

struct PendingOperation {
    // Keep the event alive at least as long as OVERLAPPED may reference it.
    overlapped: OVERLAPPED,
    event: OwnedHandle,
}

enum OverlappedFailure {
    Io(io::Error),
    StateLeaked(io::Error),
}

#[derive(Default)]
struct PipeHealth {
    poisoned: AtomicBool,
}

impl PipeHealth {
    fn ensure_usable(&self) -> io::Result<()> {
        if self.poisoned.load(Ordering::Acquire) {
            Err(terminal_connection_error())
        } else {
            Ok(())
        }
    }

    fn poison(&self) {
        self.poisoned.store(true, Ordering::Release);
    }
}

fn terminal_connection_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "Docker named-pipe cancellation did not drain; connection is unusable",
    )
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    const fn get(&self) -> HANDLE {
        self.0
    }
}

// Kernel handles may be moved to the collector's blocking stream thread. This
// wrapper owns the only close and PipeIo never performs concurrent operations.
unsafe impl Send for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

struct ParsedResponse<R> {
    status: u16,
    body: HttpBody<R>,
}

fn parse_response<R: Read>(mut source: R) -> io::Result<ParsedResponse<R>> {
    let mut header_and_prefix = Vec::with_capacity(1024);
    let header_end = loop {
        if let Some(position) = find_header_end(&header_and_prefix) {
            break position;
        }
        if header_and_prefix.len() >= MAX_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Docker response headers exceeded the limit",
            ));
        }
        let remaining = MAX_HEADER_BYTES.saturating_sub(header_and_prefix.len());
        let mut chunk = vec![0; remaining.min(4096)];
        let read = source.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Docker response ended before its headers",
            ));
        }
        header_and_prefix.extend_from_slice(&chunk[..read]);
    };

    let headers = &header_and_prefix[..header_end];
    let prefix = header_and_prefix[header_end + 4..].to_vec();
    let text = std::str::from_utf8(headers)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Docker headers were not UTF-8"))?;
    let mut lines = text.split("\r\n");
    let status = parse_status(lines.next().unwrap_or_default())?;
    let mut chunked = false;
    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Docker returned a malformed HTTP header",
            ));
        };
        if name.eq_ignore_ascii_case("transfer-encoding") {
            chunked = value
                .split(',')
                .any(|encoding| encoding.trim().eq_ignore_ascii_case("chunked"));
        } else if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(value.trim().parse::<u64>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Docker content length")
            })?);
        }
    }
    let prefixed = PrefixReader::new(prefix, source);
    let body = if chunked {
        HttpBody::Chunked(ChunkedReader::new(prefixed, MAX_CHUNK_BYTES))
    } else if let Some(length) = content_length {
        HttpBody::Fixed(ExactLengthReader::new(prefixed, length))
    } else {
        HttpBody::UntilEof(prefixed)
    };
    Ok(ParsedResponse { status, body })
}

fn parse_status(line: &str) -> io::Result<u16> {
    let mut pieces = line.split_ascii_whitespace();
    let version = pieces.next().unwrap_or_default();
    let status = pieces.next().unwrap_or_default();
    if !version.starts_with("HTTP/1.") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Docker returned an unsupported response",
        ));
    }
    status.parse::<u16>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Docker returned an invalid status",
        )
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

enum HttpBody<R> {
    Chunked(ChunkedReader<PrefixReader<R>>),
    Fixed(ExactLengthReader<PrefixReader<R>>),
    UntilEof(PrefixReader<R>),
}

impl HttpBody<BufReader<PipeIo>> {
    fn set_idle_timeout(&mut self, timeout: Duration) {
        match self {
            Self::Chunked(reader) => reader.source.source.get_mut().set_idle_timeout(timeout),
            Self::Fixed(reader) => reader.source.source.get_mut().set_idle_timeout(timeout),
            Self::UntilEof(reader) => reader.source.get_mut().set_idle_timeout(timeout),
        }
    }
}

impl<R: Read> Read for HttpBody<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Chunked(reader) => reader.read(buffer),
            Self::Fixed(reader) => reader.read(buffer),
            Self::UntilEof(reader) => reader.read(buffer),
        }
    }
}

struct ExactLengthReader<R> {
    source: R,
    remaining: u64,
}

impl<R> ExactLengthReader<R> {
    const fn new(source: R, remaining: u64) -> Self {
        Self { source, remaining }
    }
}

impl<R: Read> Read for ExactLengthReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.remaining == 0 {
            return Ok(0);
        }
        let available = usize::try_from(self.remaining).unwrap_or(usize::MAX);
        let wanted = buffer.len().min(available);
        let read = self.source.read(&mut buffer[..wanted])?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Docker response ended before its declared content length",
            ));
        }
        self.remaining = self
            .remaining
            .saturating_sub(u64::try_from(read).unwrap_or(u64::MAX));
        Ok(read)
    }
}

struct PrefixReader<R> {
    prefix: Cursor<Vec<u8>>,
    source: R,
}

impl<R> PrefixReader<R> {
    fn new(prefix: Vec<u8>, source: R) -> Self {
        Self {
            prefix: Cursor::new(prefix),
            source,
        }
    }
}

impl<R: Read> Read for PrefixReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let copied = self.prefix.read(buffer)?;
        if copied == 0 {
            self.source.read(buffer)
        } else {
            Ok(copied)
        }
    }
}

struct ChunkedReader<R> {
    source: R,
    maximum_chunk: usize,
    state: ChunkState,
    line: Vec<u8>,
    trailer_bytes: usize,
}

enum ChunkState {
    Size,
    Data(usize),
    Delimiter(usize),
    Trailers,
    Complete,
}

impl<R> ChunkedReader<R> {
    const fn new(source: R, maximum_chunk: usize) -> Self {
        Self {
            source,
            maximum_chunk,
            state: ChunkState::Size,
            line: Vec::new(),
            trailer_bytes: 0,
        }
    }
}

impl<R: Read> Read for ChunkedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            match self.state {
                ChunkState::Size => {
                    read_crlf_line(&mut self.source, &mut self.line, 128)?;
                    let size_text = self
                        .line
                        .split(|byte| *byte == b';')
                        .next()
                        .unwrap_or_default();
                    let text = std::str::from_utf8(size_text).map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "chunk size was not ASCII")
                    })?;
                    let size = usize::from_str_radix(text.trim(), 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid Docker chunk size")
                    })?;
                    if size > self.maximum_chunk {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Docker chunk exceeded the 1 MiB limit",
                        ));
                    }
                    self.line.clear();
                    self.state = if size == 0 {
                        ChunkState::Trailers
                    } else {
                        ChunkState::Data(size)
                    };
                }
                ChunkState::Data(remaining) => {
                    let wanted = buffer.len().min(remaining);
                    let read = self.source.read(&mut buffer[..wanted])?;
                    if read == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Docker chunk ended early",
                        ));
                    }
                    self.state = if read == remaining {
                        ChunkState::Delimiter(0)
                    } else {
                        ChunkState::Data(remaining - read)
                    };
                    // Return consumed payload before attempting delimiter I/O.
                    // A timeout after this read must not hide these bytes from
                    // BufReader and the incremental NDJSON frame assembler.
                    return Ok(read);
                }
                ChunkState::Delimiter(position) => {
                    let mut byte = [0];
                    self.source.read_exact(&mut byte)?;
                    if byte[0] != b"\r\n"[position] {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Docker chunk was missing its delimiter",
                        ));
                    }
                    self.state = if position == 0 {
                        ChunkState::Delimiter(1)
                    } else {
                        ChunkState::Size
                    };
                }
                ChunkState::Trailers => {
                    read_crlf_line(
                        &mut self.source,
                        &mut self.line,
                        MAX_HEADER_BYTES.saturating_sub(self.trailer_bytes),
                    )?;
                    self.trailer_bytes = self
                        .trailer_bytes
                        .saturating_add(self.line.len())
                        .saturating_add(2);
                    if self.line.is_empty() {
                        self.state = ChunkState::Complete;
                        return Ok(0);
                    }
                    if self.trailer_bytes >= MAX_HEADER_BYTES || !self.line.contains(&b':') {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Docker response trailers were malformed",
                        ));
                    }
                    self.line.clear();
                }
                ChunkState::Complete => return Ok(0),
            }
        }
    }
}

/// Keep consumed bytes in the caller-owned buffer when idle I/O times out.
fn read_crlf_line<R: Read>(source: &mut R, line: &mut Vec<u8>, limit: usize) -> io::Result<()> {
    while line.len() < limit {
        let mut byte = [0];
        source.read_exact(&mut byte)?;
        line.push(byte[0]);
        if line.ends_with(b"\r\n") {
            line.truncate(line.len().saturating_sub(2));
            return Ok(());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Docker HTTP line exceeded its limit",
    ))
}

fn duration_ms_ceil(duration: Duration) -> u32 {
    let nanos = duration.as_nanos();
    let millis = nanos.saturating_add(999_999) / 1_000_000;
    u32::try_from(millis.max(1)).unwrap_or(u32::MAX.saturating_sub(1))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chunked_body_split_across_single_byte_reads() -> io::Result<()> {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWiki\r\n5;kind=text\r\npedia\r\n0\r\nX-End: yes\r\n\r\n";
        let mut parsed = parse_response(OneByteAtATime::new(response))?;
        let mut body = String::new();
        parsed.body.read_to_string(&mut body)?;
        assert_eq!(parsed.status, 200);
        assert_eq!(body, "Wikipedia");
        Ok(())
    }

    #[test]
    fn chunk_headers_delimiters_and_trailers_survive_idle_timeouts() -> io::Result<()> {
        let wire = b"10;kind=text\r\n0123456789abcdef\r\n3\r\nxyz\r\n0\r\nX-End: yes\r\n\r\n";
        let source = TimeoutBetweenBytes {
            bytes: wire,
            position: 0,
            timeout: true,
        };
        let mut reader = ChunkedReader::new(source, MAX_CHUNK_BYTES);
        let mut decoded = Vec::new();
        let mut timeouts = 0;
        loop {
            let mut buffer = [0; 32];
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => decoded.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::TimedOut => timeouts += 1,
                Err(error) => return Err(error),
            }
        }
        assert_eq!(decoded, b"0123456789abcdefxyz");
        assert_eq!(timeouts, wire.len());
        assert!(reader.line.len() <= MAX_HEADER_BYTES);
        Ok(())
    }

    #[test]
    fn payload_is_returned_before_chunk_delimiter_io() -> io::Result<()> {
        let source = Cursor::new(b"3\r\nabc".to_vec());
        let mut reader = ChunkedReader::new(source, MAX_CHUNK_BYTES);
        let mut payload = [0; 3];
        assert_eq!(reader.read(&mut payload)?, 3);
        assert_eq!(&payload, b"abc");
        // A missing/temporarily unavailable delimiter cannot retroactively
        // discard payload bytes already consumed from the source.
        let error = reader
            .read(&mut payload)
            .err()
            .ok_or_else(|| io::Error::other("missing delimiter unexpectedly succeeded"))?;
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        Ok(())
    }

    #[test]
    fn chunk_line_limit_is_preserved_across_idle_timeouts() -> io::Result<()> {
        let bytes = [b'a'; 129];
        let source = TimeoutBetweenBytes {
            bytes: &bytes,
            position: 0,
            timeout: true,
        };
        let mut reader = ChunkedReader::new(source, MAX_CHUNK_BYTES);
        loop {
            match reader.read(&mut [0; 1]) {
                Err(error) if error.kind() == io::ErrorKind::TimedOut => continue,
                Err(error) => {
                    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
                    assert_eq!(reader.line.len(), 128);
                    return Ok(());
                }
                Ok(_) => return Err(io::Error::other("oversized chunk header accepted")),
            }
        }
    }

    #[test]
    fn preserves_body_bytes_read_with_headers() -> io::Result<()> {
        let response = Cursor::new(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello".to_vec());
        let mut parsed = parse_response(response)?;
        let mut body = Vec::new();
        parsed.body.read_to_end(&mut body)?;
        assert_eq!(body, b"hello");
        Ok(())
    }

    #[test]
    fn content_length_does_not_consume_a_following_response() -> io::Result<()> {
        let response = Cursor::new(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nokHTTP/1.1 500 Nope\r\n\r\n".to_vec(),
        );
        let mut parsed = parse_response(response)?;
        let mut body = Vec::new();
        parsed.body.read_to_end(&mut body)?;
        assert_eq!(body, b"ok");
        Ok(())
    }

    #[test]
    fn truncated_valid_json_prefix_fails_declared_content_length() -> io::Result<()> {
        let json = br#"{"ok":true}"#;
        let declared = json.len() + 2;
        let response = Cursor::new(
            format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\n\r\n")
                .into_bytes()
                .into_iter()
                .chain(json.iter().copied())
                .collect::<Vec<_>>(),
        );
        let mut parsed = parse_response(response)?;
        let mut body = Vec::new();
        let error = parsed
            .body
            .read_to_end(&mut body)
            .err()
            .ok_or_else(|| io::Error::other("truncated response unexpectedly succeeded"))?;
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(body, json);
        Ok(())
    }

    #[test]
    fn http_400_maps_to_closed_error_without_a_response_body() {
        let error = ensure_success_status(400).err();
        assert!(matches!(
            error,
            Some(AppError::DockerHttpStatus { status: 400 })
        ));
        assert!(ensure_success_status(204).is_ok());
    }

    #[test]
    fn rejects_oversized_chunk_before_allocating_for_it() -> io::Result<()> {
        let response = Cursor::new(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n100001\r\n".to_vec(),
        );
        let mut parsed = parse_response(response)?;
        let mut byte = [0];
        let error = parsed
            .body
            .read(&mut byte)
            .err()
            .ok_or_else(|| io::Error::other("oversized chunk unexpectedly succeeded"))?;
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        Ok(())
    }

    #[test]
    fn rejects_request_injection() {
        for path in [
            "version",
            "/version\r\nPOST /containers/create",
            "/bad path",
        ] {
            assert!(validate_path(path).is_err());
        }
        assert!(validate_path("/v1.43/events?filters=%7B%7D").is_ok());
    }

    #[test]
    fn rejects_missing_chunk_delimiter() -> io::Result<()> {
        let response = Cursor::new(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabcXX".to_vec(),
        );
        let mut parsed = parse_response(response)?;
        let mut body = Vec::new();
        let error = parsed
            .body
            .read_to_end(&mut body)
            .err()
            .ok_or_else(|| io::Error::other("missing delimiter unexpectedly succeeded"))?;
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        Ok(())
    }

    #[test]
    fn rejects_headers_over_the_limit() {
        let mut response = b"HTTP/1.1 200 OK\r\nX-Fill: ".to_vec();
        response.resize(MAX_HEADER_BYTES + 8, b'a');
        assert!(parse_response(Cursor::new(response)).is_err());
    }

    #[test]
    fn absolute_deadline_spends_one_shared_budget() -> io::Result<()> {
        let started = Instant::now();
        let deadline = AbsoluteDeadline {
            at: started + Duration::from_secs(5),
        };

        assert_eq!(deadline.wait_ms(started)?, 5_000);
        assert_eq!(
            deadline.wait_ms(started + Duration::from_millis(4_250))?,
            750
        );
        assert_eq!(
            deadline.wait_ms(started + Duration::from_micros(4_999_001))?,
            1
        );
        let expired = deadline.wait_ms(started + Duration::from_secs(5));
        assert!(matches!(
            expired,
            Err(error) if error.kind() == io::ErrorKind::TimedOut
        ));
        Ok(())
    }

    #[test]
    fn connect_wait_is_capped_without_renewing_request_deadline() -> io::Result<()> {
        let started = Instant::now();
        let deadline = AbsoluteDeadline {
            at: started + Duration::from_secs(5),
        };

        assert_eq!(
            deadline.wait_ms_capped(Duration::from_secs(2), started)?,
            2_000
        );
        assert_eq!(
            deadline.wait_ms_capped(
                Duration::from_secs(2),
                started + Duration::from_millis(4_600)
            )?,
            400
        );
        Ok(())
    }

    #[test]
    fn stream_idle_timeout_renews_each_operation() -> io::Result<()> {
        let idle = IoTimeout::Idle(Duration::from_secs(1));
        let first = Instant::now();
        assert_eq!(idle.wait_ms(first)?, 1_000);
        assert_eq!(idle.wait_ms(first + Duration::from_secs(30))?, 1_000);
        Ok(())
    }

    #[test]
    fn undrained_cancellation_poison_prevents_second_operation() -> io::Result<()> {
        let health = PipeHealth::default();
        let mut started_operations = 0;

        health.ensure_usable()?;
        started_operations += 1;

        // This is the deterministic state transition performed when
        // CancelIoEx is not acknowledged inside the bounded drain window.
        health.poison();
        let first_error = terminal_connection_error();
        assert_eq!(first_error.kind(), io::ErrorKind::BrokenPipe);

        if health.ensure_usable().is_ok() {
            started_operations += 1;
        }
        let retry_error = health
            .ensure_usable()
            .err()
            .ok_or_else(|| io::Error::other("poisoned pipe unexpectedly became usable"))?;
        assert_eq!(retry_error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(started_operations, 1);
        Ok(())
    }

    struct OneByteAtATime<'a> {
        bytes: &'a [u8],
        position: usize,
    }

    struct TimeoutBetweenBytes<'a> {
        bytes: &'a [u8],
        position: usize,
        timeout: bool,
    }

    impl Read for TimeoutBetweenBytes<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() || self.position == self.bytes.len() {
                return Ok(0);
            }
            if self.timeout {
                self.timeout = false;
                return Err(io::Error::new(io::ErrorKind::TimedOut, "idle"));
            }
            buffer[0] = self.bytes[self.position];
            self.position += 1;
            self.timeout = true;
            Ok(1)
        }
    }

    impl<'a> OneByteAtATime<'a> {
        const fn new(bytes: &'a [u8]) -> Self {
            Self { bytes, position: 0 }
        }
    }

    impl Read for OneByteAtATime<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() || self.position >= self.bytes.len() {
                return Ok(0);
            }
            buffer[0] = self.bytes[self.position];
            self.position += 1;
            Ok(1)
        }
    }
}
