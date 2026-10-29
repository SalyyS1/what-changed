//! Fixed conservative disk admission; no cache deletion or settings mutation.

use std::path::Path;

use rusqlite::Connection;

const WAL_TEMP_RESERVE: u64 = 512 * 1024 * 1024;
const BUILD_HEADROOM: u64 = 256 * 1024 * 1024;

fn required_bytes(rows: usize) -> u64 {
    assert!(matches!(rows, 10_000 | 1_000_000));
    // Historical 1M DB + WAL were 795,389,952 + 90,594,712 bytes.
    // This more conservative declared allowance is NOT a physical upper bound.
    u64::try_from(rows).unwrap() * 2_048 + WAL_TEMP_RESERVE + BUILD_HEADROOM
}

#[cfg(windows)]
fn free_bytes(path: &Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut available = 0_u64;
    // The canonical directory's UTF-16 buffer and output pointer stay alive
    // through this synchronous native call; no file or volume is mutated.
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(path.as_ptr()),
            Some(&mut available as *mut u64),
            None,
            None,
        )
        .expect("query available bytes on the owned fixture volume");
    }
    available
}

#[cfg(not(windows))]
fn free_bytes(_path: &Path) -> u64 {
    panic!("fixed seed-pair disk admission requires the reviewed Windows native API")
}

pub(super) fn preflight(rows: usize, route: &str) {
    assert!(
        std::env::var_os("WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS").is_none(),
        "investigation stage instrumentation must be disabled for the fixed pair"
    );
    let workspace =
        std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap();
    let available = free_bytes(&workspace);
    let required = required_bytes(rows);
    println!("seed grouping disk admission: events={rows} route={route} available_bytes={available} required_bytes={required} bytes_per_input=2048 wal_temp_reserve_bytes={WAL_TEMP_RESERVE} build_headroom_bytes={BUILD_HEADROOM}");
    assert!(available >= required, "insufficient fixture disk allowance: available_bytes={available} required_bytes={required}");
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct FrozenSettings {
    journal: String,
    numeric: Vec<(&'static str, i64)>,
}

pub(super) fn settings(conn: &Connection) -> FrozenSettings {
    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal.to_ascii_lowercase(), "wal");
    let expected = [
        ("synchronous", 1),
        ("foreign_keys", 1),
        ("temp_store", 2),
        ("cache_size", -8000),
        ("wal_autocheckpoint", 1000),
        ("busy_timeout", 5000),
        ("auto_vacuum", 2),
    ];
    let mut numeric = Vec::with_capacity(expected.len() + 1);
    for (name, expected) in expected {
        let actual: i64 = conn
            .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, expected, "frozen SQLite setting changed: {name}");
        numeric.push((name, actual));
    }
    let page_size = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap();
    numeric.push(("page_size", page_size));
    FrozenSettings { journal, numeric }
}

impl FrozenSettings {
    pub fn print(&self, rows: usize, route: &str) {
        println!(
            "seed grouping SQLite: events={rows} route={route} version={} journal_mode={}",
            rusqlite::version(),
            self.journal
        );
        for (name, value) in &self.numeric {
            println!("seed grouping SQLite numeric: events={rows} route={route} name={name} value={value}");
        }
    }
}

#[test]
fn disk_admission_has_fixed_reviewed_single_fixture_allowances() {
    assert_eq!(required_bytes(10_000), 825_786_368);
    assert_eq!(required_bytes(1_000_000), 2_853_306_368);
}
