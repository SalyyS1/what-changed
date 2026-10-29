//! Benign values a Windows developer machine emits every minute: paths,
//! versions, build ids, hashes, ports, service names. Redaction must hand every
//! one of them back byte-identical — a redactor that eats these is as broken as
//! a redactor that leaks a credential, since showing the user what changed is
//! the whole product. Tighten the rules to keep this list passing.

// A consumer may assert over a subset of the corpus.
#![allow(dead_code)]

/// A value the product exists to show the user; redaction must not touch it.
pub struct Benign {
    /// What this value is, used in assertion messages.
    pub what: &'static str,
    /// The value, which must round-trip byte-identical through redaction.
    pub value: &'static str,
}

/// Forty shapes drawn from real Windows telemetry. The hex digests, the
/// all-digit runs and the dotted versions carry the most weight: they are what a
/// naive high-entropy rule destroys first.
pub const BENIGN: &[Benign] = &[
    // Filesystem locations. Spaces, drive letters and the UNC prefix survive.
    Benign {
        what: "install path containing spaces",
        value: "C:\\Program Files\\Docker\\Docker\\resources\\bin\\docker.exe",
    },
    Benign {
        what: "roaming profile path with %APPDATA% already expanded",
        value: "C:\\Users\\dev\\AppData\\Roaming\\WhatChanged\\config.toml",
    },
    Benign {
        what: "UNC share path",
        value: "\\\\build-svr-02\\artifacts\\what-changed\\bundle.msi",
    },
    // Versions and update identifiers: the primary answer to "what changed?".
    Benign {
        what: "semantic version",
        value: "1.2.3",
    },
    Benign {
        what: "four-part runtime version",
        value: "6.0.9314.1",
    },
    Benign {
        what: "Windows build version",
        value: "10.0.26200.1234",
    },
    Benign {
        what: "Realtek network driver version",
        value: "10.70.1216.2024",
    },
    Benign {
        what: "Windows update KB article id",
        value: "KB5031356",
    },
    Benign {
        what: "known folder GUID in braces",
        value: "{6D809377-6AF0-444B-8957-A3773F02200E}",
    },
    Benign {
        what: "registry CLSID key",
        value: "HKEY_CLASSES_ROOT\\CLSID\\{4590F811-1D3A-11D0-891F-00AA004B2E24}",
    },
    // Pure-hex and pure-digit runs. The entropy gate must exclude both classes
    // outright; length alone says nothing about whether a value is a secret.
    Benign {
        what: "sha256 digest, 64 hex chars",
        value: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    },
    Benign {
        what: "git commit sha, 40 hex chars",
        value: "8f3a2c710d9b46e51a7c83f0b2e6d945c018af37",
    },
    Benign {
        what: "docker container id, 64 hex chars",
        value: "3c9d0f14a7b28e635d1c0ab9f4e8720619b3cd5a2e6f8071c58a4d3b06e91f27",
    },
    Benign {
        what: "short docker container id, 12 hex chars",
        value: "a1b2c3d4e5f6",
    },
    Benign {
        what: "sha1 file hash",
        value: "da39a3ee5e6b4b0d3255bfef95601890afd80709",
    },
    Benign {
        what: "md5 file hash",
        value: "d41d8cd98f00b204e9800998ecf8427e",
    },
    Benign {
        what: "crc32 checksum",
        value: "1c291ca3",
    },
    Benign {
        what: "56-char all-hex digest, long enough to trip a naive entropy rule",
        value: "7a1f9c30b58e246d0fb3a7c19e5d8204c6a3f19b7e0d5c8241bf6903",
    },
    Benign {
        what: "48-digit hardware serial, long enough to trip a naive entropy rule",
        value: "482910375648291037564829103756482910375648291037",
    },
    // Numbers and clocks that a change record is useless without.
    Benign {
        what: "listening TCP port",
        value: "49707",
    },
    Benign {
        what: "process id",
        value: "13748",
    },
    Benign {
        what: "free disk space in bytes",
        value: "274877906944",
    },
    Benign {
        what: "RFC3339 timestamp",
        value: "2026-09-03T14:22:07.412Z",
    },
    Benign {
        what: "human-readable duration",
        value: "2h 15m 04s",
    },
    // Services, executables and command lines. No credential appears on any
    // command line here, so the whole string must come back intact.
    Benign {
        what: "service display name",
        value: "Docker Desktop Service",
    },
    Benign {
        what: "update service display name",
        value: "Windows Update",
    },
    Benign {
        what: "executable name as it appears in the process list",
        value: "node.exe",
    },
    Benign {
        what: "command line with a quoted path and no credentials",
        value: "\"C:\\Program Files\\nodejs\\node.exe\" server.js --port 3000",
    },
    // Full NAME=value lines, so the line scrubber sees the key next to the
    // value. Each name reads like a secret; none of the values is one.
    Benign {
        what: "environment line naming a port, not a key",
        value: "API_PORT=8080",
    },
    Benign {
        what: "environment line whose name starts with KEY but holds a locale",
        value: "KEYBOARD_LAYOUT=en-US",
    },
    Benign {
        what: "environment line whose name contains TOKEN but holds a path",
        value: "TOKENIZER_PATH=C:\\tools\\tokenizer.json",
    },
    Benign {
        what: "environment line whose name contains SESSION but holds a timeout",
        value: "SESSION_TIMEOUT_MS=1800000",
    },
    // Network identity. The URL carries no userinfo, so nothing in it is a
    // credential and the host and path must both survive.
    Benign {
        what: "private IPv4 address",
        value: "192.168.1.24",
    },
    Benign {
        what: "IPv6 address",
        value: "2001:db8::8a2e:370:7334",
    },
    Benign {
        what: "fully qualified hostname",
        value: "build-svr-02.corp.example.internal",
    },
    Benign {
        what: "URL with no userinfo",
        value: "https://example.internal/api/v1/status",
    },
    // Event titles as the user reads them, including a bare drive letter that
    // looks like a "key: value" pair to a line scrubber.
    Benign {
        what: "event title naming a port and a process",
        value: "Port 3000 now listening (node.exe)",
    },
    Benign {
        what: "event title with a drive letter followed by a colon",
        value: "Free space on C: dropped by 4.2 GB in the last hour",
    },
    Benign {
        what: "Windows event log id",
        value: "7036",
    },
    Benign {
        what: "Windows event log source name",
        value: "Service Control Manager",
    },
];
