//! Restricted interoperable ZIP profile: stored regular files, no extraction.
//! Layout follows PKWARE APPNOTE 6.3.10 sections 4.3.7, 4.3.12 and 4.3.16.
//! The importer consumes bounded entry slices in memory; archive names never
//! become OS paths. Compression, extras, links, encryption and ZIP64 fail closed.
mod reader;
use super::validation;
use crate::error::AppResult;
pub use reader::decode;
use std::collections::BTreeMap;

pub const MAX_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
pub const ALLOWED: [&str; 4] = [
    "manifest.json",
    "workspace.json",
    "README.md",
    "recipes.json",
];

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
fn short(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_le_bytes());
}
fn long(out: &mut Vec<u8>, n: u32) {
    out.extend(n.to_le_bytes());
}

pub fn encode(files: &BTreeMap<String, Vec<u8>>) -> AppResult<Vec<u8>> {
    if files.len() != ALLOWED.len() || !ALLOWED.iter().all(|n| files.contains_key(*n)) {
        return Err(validation::invalid(
            "bundle entries",
            "requires the four supported files",
        ));
    }
    encode_ordered(files, &ALLOWED)
}

/// Internal exporters supply their own fixed names. Import keeps its four-file profile.
pub(crate) fn encode_entries(files: &BTreeMap<String, Vec<u8>>) -> AppResult<Vec<u8>> {
    let order = files.keys().map(String::as_str).collect::<Vec<_>>();
    encode_ordered(files, &order)
}

fn encode_ordered(files: &BTreeMap<String, Vec<u8>>, order: &[&str]) -> AppResult<Vec<u8>> {
    if files.is_empty()
        || files.len() > 32
        || files.keys().any(|name| {
            name.len() > 200
                || name.starts_with('/')
                || name.contains(['\\', ':'])
                || name
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
        })
    {
        return Err(validation::invalid(
            "bundle entries",
            "requires bounded relative regular-file names",
        ));
    }
    if files.values().map(Vec::len).sum::<usize>() > MAX_ARCHIVE_BYTES - 1024 {
        return Err(validation::invalid("bundle", "exceeds the 16 MiB limit"));
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    // Manifest is first, with a deterministic order and DOS epoch timestamp.
    for name in order {
        let bytes = &files[*name];
        let offset = out.len() as u32;
        let crc = crc32(bytes);
        long(&mut out, 0x04034b50);
        short(&mut out, 20);
        short(&mut out, 0);
        short(&mut out, 0);
        short(&mut out, 0);
        short(&mut out, 33);
        long(&mut out, crc);
        long(&mut out, bytes.len() as u32);
        long(&mut out, bytes.len() as u32);
        short(&mut out, name.len() as u16);
        short(&mut out, 0);
        out.extend(name.as_bytes());
        out.extend(bytes);
        long(&mut central, 0x02014b50);
        short(&mut central, 20);
        short(&mut central, 20);
        short(&mut central, 0);
        short(&mut central, 0);
        short(&mut central, 0);
        short(&mut central, 33);
        long(&mut central, crc);
        long(&mut central, bytes.len() as u32);
        long(&mut central, bytes.len() as u32);
        short(&mut central, name.len() as u16);
        short(&mut central, 0);
        short(&mut central, 0);
        short(&mut central, 0);
        short(&mut central, 0);
        long(&mut central, 0);
        long(&mut central, offset);
        central.extend(name.as_bytes());
    }
    let offset = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    long(&mut out, 0x06054b50);
    short(&mut out, 0);
    short(&mut out, 0);
    short(&mut out, files.len() as u16);
    short(&mut out, files.len() as u16);
    long(&mut out, size);
    long(&mut out, offset);
    short(&mut out, 0);
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn standard_crc_and_deterministic_round_trip() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        let entries = ALLOWED
            .iter()
            .map(|n| (n.to_string(), n.as_bytes().to_vec()))
            .collect();
        let a = encode(&entries).expect("valid fixture");
        assert_eq!(a, encode(&entries).expect("same fixture"));
        assert_eq!(decode(&a).expect("round trip"), entries);
    }
    #[test]
    fn unsupported_names_and_compressed_or_tampered_entries_fail() {
        let mut entries: BTreeMap<_, _> =
            ALLOWED.iter().map(|n| (n.to_string(), vec![1])).collect();
        entries.insert("../outside".into(), vec![]);
        assert!(encode(&entries).is_err());
        entries.remove("../outside");
        let bytes = encode(&entries).expect("fixture");
        for index in [0, 8, 14, 18, 22, 26, 28, 30, bytes.len() - 1] {
            let mut tampered = bytes.clone();
            tampered[index] ^= 0x80;
            assert!(decode(&tampered).is_err(), "offset {index}");
        }
    }
}
