//! Listener enumeration through the extended TCP and UDP tables.
//!
//! Each table is fetched with a size probe, a reusable buffer, and a bounded retry,
//! because the table can grow between the probe and the read and the API answers
//! that by reporting the new size. Four family-and-protocol combinations are
//! read, and any one of them failing fails the whole cycle: a collector that
//! reported "no IPv6 listeners" because a read failed would close every IPv6
//! socket in the history, and a fabricated close is worse than a missed one.

use std::ffi::c_void;
use std::mem::{offset_of, size_of};
use std::sync::{Mutex, MutexGuard, OnceLock};

use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NO_DATA, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
    MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDP6TABLE_OWNER_PID,
    MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    UDP_TABLE_OWNER_PID,
};
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};

use crate::error::{AppError, AppResult};
use crate::platform::traits::{SocketProvider, SocketRow};
use crate::platform::windows::socket_rows;

/// Grow-and-retry attempts for one table. The table has to grow three times in
/// the microseconds between calls for this to be exceeded, and when it is the
/// cycle fails rather than reporting a truncated listener set.
const TABLE_ATTEMPTS: usize = 3;

/// Every one of these tables begins with a `u32` row count, so a buffer shorter
/// than that describes nothing.
const COUNT_BYTES: usize = size_of::<u32>();

/// An upper bound on one table, so a bogus size from the API cannot become an
/// allocation. Sixteen megabytes is far more listeners than a machine has; the
/// development machine reports 37 TCP and 67 UDP.
const MAX_TABLE_BYTES: usize = 16 * 1024 * 1024;

const TCP_V4_OPERATION: &str = "read the TCP IPv4 listener table";
const TCP_V6_OPERATION: &str = "read the TCP IPv6 listener table";
const UDP_V4_OPERATION: &str = "read the UDP IPv4 endpoint table";
const UDP_V6_OPERATION: &str = "read the UDP IPv6 endpoint table";

/// Initial caller-owned row capacity. The development machine reports about a
/// hundred rows across all four tables, so this avoids startup growth while
/// remaining tiny next to the byte buffers themselves.
const RESERVE_ROWS: usize = 128;

/// Reads all four listener tables.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowsSocketProvider;

#[derive(Default)]
struct SocketScratch {
    tcp_v4: Vec<u8>,
    tcp_v6: Vec<u8>,
    udp_v4: Vec<u8>,
    udp_v6: Vec<u8>,
}

fn socket_scratch() -> &'static Mutex<SocketScratch> {
    static SCRATCH: OnceLock<Mutex<SocketScratch>> = OnceLock::new();
    SCRATCH.get_or_init(|| Mutex::new(SocketScratch::default()))
}

fn lock_scratch() -> MutexGuard<'static, SocketScratch> {
    socket_scratch()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl SocketProvider for WindowsSocketProvider {
    fn listeners(&self) -> AppResult<Vec<SocketRow>> {
        let mut rows = Vec::with_capacity(RESERVE_ROWS);
        self.listeners_into(&mut rows)?;
        Ok(rows)
    }

    fn listeners_into(&self, output: &mut Vec<SocketRow>) -> AppResult<()> {
        output.clear();
        let mut scratch = lock_scratch();
        let result = (|| {
            tcp_v4_table(&mut scratch.tcp_v4, output)?;
            tcp_v6_table(&mut scratch.tcp_v6, output)?;
            udp_v4_table(&mut scratch.udp_v4, output)?;
            udp_v6_table(&mut scratch.udp_v6, output)?;
            Ok(())
        })();
        if result.is_err() {
            // One failed family invalidates the entire cycle. In particular,
            // never leave the successful IPv4 prefix where it could be read
            // as a complete table and forge IPv6 closes.
            output.clear();
        }
        result
    }
}

fn tcp_v4_table(buffer: &mut Vec<u8>, rows: &mut Vec<SocketRow>) -> AppResult<()> {
    let Some(written) = fetch_table(buffer, TCP_V4_OPERATION, |table, size| unsafe {
        GetExtendedTcpTable(
            table,
            size,
            false,
            u32::from(AF_INET.0),
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    })?
    else {
        return Ok(());
    };
    append_rows::<MIB_TCPROW_OWNER_PID>(
        &buffer[..written],
        offset_of!(MIB_TCPTABLE_OWNER_PID, table),
        socket_rows::tcp_v4,
        rows,
        TCP_V4_OPERATION,
    )?;
    Ok(())
}

fn tcp_v6_table(buffer: &mut Vec<u8>, rows: &mut Vec<SocketRow>) -> AppResult<()> {
    let Some(written) = fetch_table(buffer, TCP_V6_OPERATION, |table, size| unsafe {
        GetExtendedTcpTable(
            table,
            size,
            false,
            u32::from(AF_INET6.0),
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    })?
    else {
        return Ok(());
    };
    append_rows::<MIB_TCP6ROW_OWNER_PID>(
        &buffer[..written],
        offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
        socket_rows::tcp_v6,
        rows,
        TCP_V6_OPERATION,
    )?;
    Ok(())
}

fn udp_v4_table(buffer: &mut Vec<u8>, rows: &mut Vec<SocketRow>) -> AppResult<()> {
    let Some(written) = fetch_table(buffer, UDP_V4_OPERATION, |table, size| unsafe {
        GetExtendedUdpTable(
            table,
            size,
            false,
            u32::from(AF_INET.0),
            UDP_TABLE_OWNER_PID,
            0,
        )
    })?
    else {
        return Ok(());
    };
    append_rows::<MIB_UDPROW_OWNER_PID>(
        &buffer[..written],
        offset_of!(MIB_UDPTABLE_OWNER_PID, table),
        socket_rows::udp_v4,
        rows,
        UDP_V4_OPERATION,
    )?;
    Ok(())
}

fn udp_v6_table(buffer: &mut Vec<u8>, rows: &mut Vec<SocketRow>) -> AppResult<()> {
    let Some(written) = fetch_table(buffer, UDP_V6_OPERATION, |table, size| unsafe {
        GetExtendedUdpTable(
            table,
            size,
            false,
            u32::from(AF_INET6.0),
            UDP_TABLE_OWNER_PID,
            0,
        )
    })?
    else {
        return Ok(());
    };
    append_rows::<MIB_UDP6ROW_OWNER_PID>(
        &buffer[..written],
        offset_of!(MIB_UDP6TABLE_OWNER_PID, table),
        socket_rows::udp_v6,
        rows,
        UDP_V6_OPERATION,
    )?;
    Ok(())
}

/// Size and read one table into reusable storage, retrying while the platform
/// says the buffer was too small.
///
/// Returns the initialized byte count, or `None` when the platform reports
/// there is no data for that family. The latter is a real answer on a machine
/// with no IPv6 stack and not a failure.
fn fetch_table(
    buffer: &mut Vec<u8>,
    operation: &'static str,
    mut call: impl FnMut(Option<*mut c_void>, *mut u32) -> u32,
) -> AppResult<Option<usize>> {
    let mut size: u32 = 0;
    let mut status = call(None, &mut size);
    for _ in 0..TABLE_ATTEMPTS {
        if status == ERROR_NO_DATA.0 {
            return Ok(None);
        }
        if status != ERROR_INSUFFICIENT_BUFFER.0 && status != ERROR_SUCCESS.0 {
            break;
        }
        let requested = usize::try_from(size).unwrap_or(MAX_TABLE_BYTES + 1);
        if requested < COUNT_BYTES {
            return Err(malformed_table(
                operation,
                format!(
                    "the platform reported {requested} bytes, smaller than the {COUNT_BYTES}-byte count field"
                ),
            ));
        }
        if requested > MAX_TABLE_BYTES {
            return Err(AppError::Platform {
                operation,
                reason: format!("the platform asked for {requested} bytes for one table"),
            });
        }
        buffer.resize(requested, 0);
        status = call(Some(buffer.as_mut_ptr().cast::<c_void>()), &mut size);
        if status == ERROR_SUCCESS.0 {
            let written = usize::try_from(size).unwrap_or(MAX_TABLE_BYTES + 1);
            if written < COUNT_BYTES {
                return Err(malformed_table(
                    operation,
                    format!(
                        "the platform initialized {written} bytes, smaller than the {COUNT_BYTES}-byte count field"
                    ),
                ));
            }
            if written > requested {
                return Err(AppError::Platform {
                    operation,
                    reason: format!(
                        "the platform reported {written} initialized bytes in a {requested}-byte buffer"
                    ),
                });
            }
            return Ok(Some(written));
        }
        // Anything else updated `size`; the next attempt uses it.
    }
    Err(AppError::Platform {
        operation,
        reason: format!("{status:#010x} after {TABLE_ATTEMPTS} attempts"),
    })
}

/// Read the row array the API wrote after the table's count field.
///
/// The payload must contain every row the header declares. Windows may return
/// an oversized size-probe buffer, so unused tail bytes are allowed; treating
/// a missing declared suffix as a shorter valid snapshot would forge closes.
#[cfg(test)]
fn parse_rows<Row>(
    buffer: &[u8],
    row_offset: usize,
    convert: fn(&Row) -> SocketRow,
) -> AppResult<Vec<SocketRow>> {
    let mut rows = Vec::new();
    append_rows(
        buffer,
        row_offset,
        convert,
        &mut rows,
        "parse the test listener table",
    )?;
    Ok(rows)
}

fn append_rows<Row>(
    buffer: &[u8],
    row_offset: usize,
    convert: fn(&Row) -> SocketRow,
    output: &mut Vec<SocketRow>,
    operation: &'static str,
) -> AppResult<()> {
    if buffer.len() < COUNT_BYTES || row_offset < COUNT_BYTES || row_offset > buffer.len() {
        return Err(malformed_table(
            operation,
            "the count field or row offset is truncated",
        ));
    }
    let row_size = size_of::<Row>().max(1);
    let payload_bytes = buffer.len() - row_offset;
    let complete_rows = payload_bytes / row_size;
    let declared = usize::try_from(row_count(buffer)).unwrap_or(usize::MAX);
    if declared > complete_rows {
        return Err(AppError::Platform {
            operation,
            reason: format!(
                "the table declared {declared} rows but only contained {complete_rows} complete rows"
            ),
        });
    }
    // A `Vec<u8>` is not promised to have `Row` alignment. Windows writes the
    // table as bytes, so copy each POD row with an unaligned read before the
    // converter borrows it. The declared-versus-complete-row validation above
    // proves every declared read is within the initialized slice; spare tail
    // bytes in the platform-sized buffer are ignored.
    for index in 0..declared {
        let byte_offset = row_offset + index * row_size;
        // SAFETY: `byte_offset + row_size` is within `buffer` by the complete-
        // row check. The IP Helper row structs contain only integers and byte
        // arrays, and `read_unaligned` removes the alignment premise.
        let row = unsafe {
            buffer
                .as_ptr()
                .add(byte_offset)
                .cast::<Row>()
                .read_unaligned()
        };
        output.push(convert(&row));
    }
    Ok(())
}

fn malformed_table(operation: &'static str, reason: impl Into<String>) -> AppError {
    AppError::Platform {
        operation,
        reason: reason.into(),
    }
}

/// The `dwNumEntries` field every one of these tables starts with.
fn row_count(buffer: &[u8]) -> u32 {
    if buffer.len() < COUNT_BYTES {
        return 0;
    }
    u32::from_ne_bytes([buffer[0], buffer[1], buffer[2], buffer[3]])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::ids::Protocol;

    /// A buffer laid out the way the API lays one out: a count, then rows.
    fn table_buffer(rows: &[MIB_TCPROW_OWNER_PID]) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&(rows.len() as u32).to_ne_bytes());
        // The real struct pads nothing before the row array: every table here
        // is a `u32` count followed by fixed-size `u32`-aligned rows.
        assert_eq!(offset_of!(MIB_TCPTABLE_OWNER_PID, table), COUNT_BYTES);
        for row in rows {
            buffer.extend_from_slice(&row.dwState.to_ne_bytes());
            buffer.extend_from_slice(&row.dwLocalAddr.to_ne_bytes());
            buffer.extend_from_slice(&row.dwLocalPort.to_ne_bytes());
            buffer.extend_from_slice(&row.dwRemoteAddr.to_ne_bytes());
            buffer.extend_from_slice(&row.dwRemotePort.to_ne_bytes());
            buffer.extend_from_slice(&row.dwOwningPid.to_ne_bytes());
        }
        buffer
    }

    fn listener(port: u16, pid: u32) -> MIB_TCPROW_OWNER_PID {
        MIB_TCPROW_OWNER_PID {
            dwState: 2,
            dwLocalAddr: u32::from_ne_bytes([0, 0, 0, 0]),
            dwLocalPort: u32::from_ne_bytes([(port >> 8) as u8, (port & 0xFF) as u8, 0, 0]),
            dwRemoteAddr: 0,
            dwRemotePort: 0,
            dwOwningPid: pid,
        }
    }

    #[test]
    fn a_real_provider_reads_the_listeners_this_machine_has() {
        let provider = WindowsSocketProvider;
        let rows = provider.listeners().unwrap();
        assert!(
            rows.iter().any(|row| row.protocol == Protocol::Tcp),
            "a running Windows machine has at least one TCP listener"
        );
        for row in &rows {
            assert!(!row.address.is_empty());
            assert!(row.owning_pid > 0, "the table always names an owner");
        }
    }

    #[test]
    fn rows_are_read_out_of_the_buffer_the_api_wrote() {
        let buffer = table_buffer(&[listener(4_000, 4_242), listener(53, 7)]);
        let rows = parse_rows(
            &buffer,
            offset_of!(MIB_TCPTABLE_OWNER_PID, table),
            socket_rows::tcp_v4,
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].port, 4_000);
        assert_eq!(rows[0].owning_pid, 4_242);
        assert_eq!(rows[1].port, 53);
        assert_eq!(rows[0].protocol, Protocol::Tcp);
    }

    #[test]
    fn a_count_larger_than_the_buffer_fails_the_whole_table() {
        let mut buffer = table_buffer(&[listener(4_000, 1)]);
        buffer[..COUNT_BYTES].copy_from_slice(&999u32.to_ne_bytes());
        let error = parse_rows(
            &buffer,
            offset_of!(MIB_TCPTABLE_OWNER_PID, table),
            socket_rows::tcp_v4,
        )
        .unwrap_err();
        assert!(matches!(error, AppError::Platform { .. }));
        assert!(error.to_string().contains("declared 999 rows"));
    }

    #[test]
    fn an_empty_or_truncated_buffer_fails_instead_of_forging_closes() {
        let empty: Vec<u8> = Vec::new();
        let offset = offset_of!(MIB_TCPTABLE_OWNER_PID, table);
        assert!(parse_rows(&empty, offset, socket_rows::tcp_v4).is_err());
        assert!(parse_rows(&0u32.to_ne_bytes(), offset, socket_rows::tcp_v4)
            .unwrap()
            .is_empty());
        // A count of one with no row bytes behind it.
        assert!(parse_rows(&1u32.to_ne_bytes(), offset, socket_rows::tcp_v4).is_err());
    }

    #[test]
    fn a_payload_ending_part_way_through_a_row_fails_visibly() {
        let mut buffer = table_buffer(&[listener(4_000, 1)]);
        buffer.pop();
        let error = parse_rows(
            &buffer,
            offset_of!(MIB_TCPTABLE_OWNER_PID, table),
            socket_rows::tcp_v4,
        )
        .unwrap_err();
        assert!(error.to_string().contains("only contained 0 complete rows"));
    }

    #[test]
    fn spare_complete_bytes_are_not_mistaken_for_declared_rows() {
        let mut buffer = table_buffer(&[listener(4_000, 1), listener(5_000, 2)]);
        buffer[..COUNT_BYTES].copy_from_slice(&1u32.to_ne_bytes());
        let rows = parse_rows(
            &buffer,
            offset_of!(MIB_TCPTABLE_OWNER_PID, table),
            socket_rows::tcp_v4,
        )
        .unwrap();
        assert_eq!(rows.len(), 1, "the header is authoritative");
        assert_eq!(rows[0].port, 4_000);
    }

    #[test]
    fn the_size_probe_loop_grows_the_buffer_until_the_platform_is_satisfied() {
        let mut calls = 0usize;
        let mut last_size = 0usize;
        let mut buffer = Vec::new();
        let written = fetch_table(&mut buffer, "read the test table", |table, size| {
            calls += 1;
            match table {
                None => {
                    unsafe { *size = 8 };
                    ERROR_INSUFFICIENT_BUFFER.0
                }
                Some(pointer) => {
                    let room = unsafe { *(size as *const u32) } as usize;
                    last_size = room;
                    if room < 16 {
                        unsafe { *size = 16 };
                        ERROR_INSUFFICIENT_BUFFER.0
                    } else {
                        let slice =
                            unsafe { std::slice::from_raw_parts_mut(pointer.cast::<u8>(), room) };
                        slice[..4].copy_from_slice(&1u32.to_ne_bytes());
                        unsafe { *size = 4 };
                        ERROR_SUCCESS.0
                    }
                }
            }
        })
        .unwrap()
        .unwrap();
        assert_eq!(calls, 3, "one probe and two buffered attempts");
        assert_eq!(last_size, 16);
        assert_eq!(written, 4, "reports exactly what the platform wrote");
        assert_eq!(row_count(&buffer[..written]), 1);
    }

    #[test]
    fn no_data_for_a_family_is_an_answer_rather_than_a_failure() {
        let mut buffer = Vec::new();
        let written = fetch_table(&mut buffer, "read the test table", |_table, _size| {
            ERROR_NO_DATA.0
        })
        .unwrap();
        assert!(written.is_none());
    }

    #[test]
    fn a_sub_header_probe_size_is_malformed_for_every_allowed_status() {
        for status in [ERROR_SUCCESS.0, ERROR_INSUFFICIENT_BUFFER.0] {
            let mut calls = 0usize;
            let mut buffer = Vec::new();
            let error = fetch_table(&mut buffer, "read the test table", |_table, size| {
                calls += 1;
                unsafe { *size = (COUNT_BYTES - 1) as u32 };
                status
            })
            .unwrap_err();
            assert_eq!(calls, 1);
            assert!(matches!(error, AppError::Platform { .. }));
            assert!(error.to_string().contains("smaller than"));
        }
    }

    #[test]
    fn a_successful_read_that_initializes_less_than_a_header_is_malformed() {
        let mut calls = 0usize;
        let mut buffer = Vec::new();
        let error = fetch_table(&mut buffer, "read the test table", |table, size| {
            calls += 1;
            if table.is_none() {
                unsafe { *size = 32 };
                ERROR_INSUFFICIENT_BUFFER.0
            } else {
                unsafe { *size = (COUNT_BYTES - 1) as u32 };
                ERROR_SUCCESS.0
            }
        })
        .unwrap_err();
        assert_eq!(calls, 2);
        assert!(matches!(error, AppError::Platform { .. }));
        assert!(error.to_string().contains("initialized"));
    }

    #[test]
    fn an_impossible_table_size_is_refused_before_it_is_allocated() {
        let mut calls = 0usize;
        let mut buffer = Vec::new();
        let error = fetch_table(&mut buffer, "read the test table", |_table, size| {
            calls += 1;
            unsafe { *size = (MAX_TABLE_BYTES as u32) + 1 };
            ERROR_INSUFFICIENT_BUFFER.0
        })
        .unwrap_err();
        assert_eq!(calls, 1, "the size is checked before the allocation");
        match error {
            AppError::Platform { operation, reason } => {
                assert_eq!(operation, "read the test table");
                assert!(reason.contains("bytes for one table"), "{reason}");
            }
            other => panic!("expected a platform error, got {other:?}"),
        }
    }

    #[test]
    fn a_table_that_keeps_growing_gives_up_after_three_attempts() {
        let mut calls = 0usize;
        let mut buffer = Vec::new();
        let error = fetch_table(&mut buffer, "read the test table", |_table, size| {
            calls += 1;
            unsafe { *size = 64 };
            ERROR_INSUFFICIENT_BUFFER.0
        })
        .unwrap_err();
        assert_eq!(calls, 4, "one probe plus three buffered attempts");
        assert!(matches!(error, AppError::Platform { .. }));
    }

    #[test]
    fn repeated_table_reads_reuse_the_existing_allocation() {
        let mut buffer = Vec::with_capacity(64);
        let original = (buffer.as_ptr(), buffer.capacity());
        for _ in 0..2 {
            let written = fetch_table(&mut buffer, "read the test table", |table, size| {
                if let Some(pointer) = table {
                    let room = unsafe { *size } as usize;
                    let slice =
                        unsafe { std::slice::from_raw_parts_mut(pointer.cast::<u8>(), room) };
                    slice[..COUNT_BYTES].copy_from_slice(&0u32.to_ne_bytes());
                    unsafe { *size = COUNT_BYTES as u32 };
                    ERROR_SUCCESS.0
                } else {
                    unsafe { *size = 32 };
                    ERROR_INSUFFICIENT_BUFFER.0
                }
            })
            .unwrap();
            assert_eq!(written, Some(COUNT_BYTES));
        }
        assert_eq!((buffer.as_ptr(), buffer.capacity()), original);
    }

    #[test]
    fn caller_owned_socket_rows_are_reused_across_polls() {
        let provider = WindowsSocketProvider;
        let mut rows = Vec::with_capacity(RESERVE_ROWS);
        provider.listeners_into(&mut rows).unwrap();
        let first = (rows.as_ptr(), rows.capacity());
        provider.listeners_into(&mut rows).unwrap();
        assert_eq!(
            (rows.as_ptr(), rows.capacity()),
            first,
            "steady listener rows stay in the caller's allocation"
        );
    }
}
