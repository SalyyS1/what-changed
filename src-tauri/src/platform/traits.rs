//! Platform-neutral provider contracts.
//!
//! A collector diffs snapshots; a provider produces them. Keeping the two
//! behind these traits is what lets every collector be unit-tested against a
//! scripted table instead of a machine in a particular state, and it is the
//! only seam through which an operating-system call reaches diff logic.

use std::io::BufRead;

use crate::domain::ids::Protocol;
use crate::error::AppResult;

/// The narrow read-only transport used by the Docker collector.
///
/// The production implementation speaks HTTP over Docker Desktop's named
/// pipe. Scripted implementations keep protocol and reconnect policy tests
/// independent from Docker Desktop and Windows.
pub trait DockerTransport: Send + 'static {
    /// Execute one bounded HTTP GET and return its decoded body.
    fn get(&mut self, path: &str) -> AppResult<Vec<u8>>;

    /// Start one HTTP GET whose decoded body may remain open indefinitely.
    fn stream(&mut self, path: &str) -> AppResult<Box<dyn BufRead + Send>>;
}

/// One running process, as the operating system reports it.
///
/// What is absent here is the point: there is no command line and no
/// environment block, because those are where credentials are passed and no
/// collector has a reason to read either. `tests/architecture.rs` fails the
/// build if a collector or a platform module reaches for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRow {
    /// Process id. Unique among live processes at one instant, and reused by
    /// the operating system afterwards — which is why it is never an identity
    /// on its own.
    pub pid: u32,
    /// Parent process id. The parent may already have exited, so this is a
    /// hint about a subprocess tree, not a live handle.
    pub parent_pid: u32,
    /// The image name the snapshot reports, for example `api.exe`.
    pub image_name: String,
    /// The full executable path, when a non-elevated reader may see it.
    ///
    /// Roughly 46% of running processes deny this read. `None` is normal
    /// operation, not an error, and must not be logged per occurrence.
    pub executable_path: Option<String>,
    /// Creation time as the raw `FILETIME` value, when the process handle could
    /// be opened. This is what separates a live process from a recycled pid.
    pub creation_time: Option<u64>,
}

/// One listening socket, as the operating system reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketRow {
    /// TCP or UDP.
    pub protocol: Protocol,
    /// Local address in text form: a dotted quad or a compressed IPv6.
    pub address: String,
    /// Local port, already converted from network byte order by the provider.
    pub port: u16,
    /// The process that owns the socket, when the table reported one.
    pub owning_pid: u32,
}

/// Enumerates running processes.
pub trait ProcessProvider: Send + Sync + 'static {
    /// The whole process table, or an error.
    ///
    /// A partial table must be an error rather than a short `Vec`: a collector
    /// diffs what it is given, and a truncated enumeration would forge a stop
    /// for every process after the point where reading failed.
    fn enumerate(&self) -> AppResult<Vec<ProcessRow>>;

    /// Fill caller-owned storage with the complete process table.
    ///
    /// The default keeps existing scripted providers source-compatible. A
    /// production provider should override it so the collector's pre-sized
    /// allocation is reused rather than allocating a whole-table `Vec` on
    /// every poll. The destination is empty after any error.
    fn enumerate_into(&self, output: &mut Vec<ProcessRow>) -> AppResult<()> {
        clear_process_rows(output);
        let rows = self.enumerate()?;
        output.extend(rows);
        Ok(())
    }
}

/// Enumerates listening sockets.
pub trait SocketProvider: Send + Sync + 'static {
    /// Every TCP and UDP listener for both IP versions, or an error.
    ///
    /// The same rule as [`ProcessProvider::enumerate`]: an address family that
    /// could not be read fails the cycle instead of silently reporting no
    /// listeners, which would close every socket in that family.
    fn listeners(&self) -> AppResult<Vec<SocketRow>>;

    /// Fill caller-owned storage with all four complete listener tables.
    ///
    /// Existing providers may rely on this default. The Windows provider
    /// overrides it to reuse the collector's allocation. The destination is
    /// empty after any error, so a failed partial read cannot be mistaken for
    /// a complete snapshot by a later caller.
    fn listeners_into(&self, output: &mut Vec<SocketRow>) -> AppResult<()> {
        output.clear();
        let rows = self.listeners()?;
        output.extend(rows);
        Ok(())
    }
}

/// Drop process rows without leaving executable-path strings in reusable
/// collector storage.
///
/// `Vec::clear` drops the strings, but clearing the path cell first also makes
/// the privacy boundary explicit: only the vector allocation survives a poll,
/// never a live path value.
pub(crate) fn clear_process_rows(rows: &mut Vec<ProcessRow>) {
    for row in rows.iter_mut() {
        if let Some(path) = row.executable_path.as_mut() {
            path.clear();
        }
        row.executable_path = None;
    }
    rows.clear();
}
