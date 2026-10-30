use super::{crc32, ALLOWED, MAX_ARCHIVE_BYTES};
use crate::error::AppResult;
use crate::owner_workspace::validation;
use std::collections::{BTreeMap, BTreeSet};

fn invalid() -> crate::error::AppError {
    validation::invalid("ZIP bundle", "invalid or unsupported archive; use a stored Ownvero bundle with four regular files and no extra fields")
}
fn range(bytes: &[u8], start: usize, len: usize) -> AppResult<&[u8]> {
    bytes
        .get(start..start.checked_add(len).ok_or_else(invalid)?)
        .ok_or_else(invalid)
}
fn u16_at(bytes: &[u8], at: usize) -> AppResult<u16> {
    let b = range(bytes, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(bytes: &[u8], at: usize) -> AppResult<u32> {
    let b = range(bytes, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn decode(bytes: &[u8]) -> AppResult<BTreeMap<String, Vec<u8>>> {
    if bytes.len() < 22 || bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(invalid());
    }
    let end = bytes.len() - 22;
    if u32_at(bytes, end)? != 0x06054b50
        || u16_at(bytes, end + 4)? != 0
        || u16_at(bytes, end + 6)? != 0
        || u16_at(bytes, end + 8)? != 4
        || u16_at(bytes, end + 10)? != 4
        || u16_at(bytes, end + 20)? != 0
    {
        return Err(invalid());
    }
    let start = u32_at(bytes, end + 16)? as usize;
    if start.checked_add(u32_at(bytes, end + 12)? as usize) != Some(end) {
        return Err(invalid());
    }
    let mut cursor = start;
    let mut next_local = 0;
    let mut names = BTreeSet::new();
    let mut files = BTreeMap::new();
    for _ in 0..4 {
        let h = range(bytes, cursor, 46)?;
        if u32_at(h, 0)? != 0x02014b50
            || u16_at(h, 6)? > 20
            || u16_at(h, 8)? != 0
            || u16_at(h, 10)? != 0
            || u16_at(h, 30)? != 0
            || u16_at(h, 32)? != 0
            || u16_at(h, 34)? != 0
            || u32_at(h, 38)? != 0
        {
            return Err(invalid());
        }
        let len = u16_at(h, 28)? as usize;
        let name = std::str::from_utf8(range(bytes, cursor + 46, len)?).map_err(|_| invalid())?;
        // Exact ASCII allowlist also rejects Windows ADS/device/alias names,
        // both separators, Unicode/case collisions, absolute and parent paths.
        if !ALLOWED.contains(&name) || !names.insert(name.to_ascii_lowercase()) {
            return Err(invalid());
        }
        let compressed = u32_at(h, 20)? as usize;
        let size = u32_at(h, 24)? as usize;
        if compressed != size || size > MAX_ARCHIVE_BYTES || u32_at(h, 42)? as usize != next_local {
            return Err(invalid());
        }
        let local = range(bytes, next_local, 30)?;
        if u32_at(local, 0)? != 0x04034b50
            || u16_at(local, 4)? > 20
            || u16_at(local, 6)? != 0
            || u16_at(local, 8)? != 0
            || u16_at(local, 28)? != 0
            || u16_at(local, 26)? as usize != len
            || u32_at(local, 14)? != u32_at(h, 16)?
            || u32_at(local, 18)? as usize != size
            || u32_at(local, 22)? as usize != size
        {
            return Err(invalid());
        }
        if range(bytes, next_local + 30, len)? != name.as_bytes() {
            return Err(invalid());
        }
        let data_start = next_local.checked_add(30 + len).ok_or_else(invalid)?;
        let data_end = data_start.checked_add(size).ok_or_else(invalid)?;
        if data_end > start {
            return Err(invalid());
        }
        let data = range(bytes, data_start, size)?;
        if crc32(data) != u32_at(h, 16)? {
            return Err(invalid());
        }
        files.insert(name.to_owned(), data.to_vec());
        next_local = data_end;
        cursor = cursor.checked_add(46 + len).ok_or_else(invalid)?;
    }
    if cursor != end || next_local != start {
        return Err(invalid());
    }
    Ok(files)
}
