# Missing full-text source collection — 2026-10-05

Outcome: 22 complete source-text transcriptions collected for all 11 previously
missing-full-text candidates. Exact-version/source correspondence is verified;
license choice, obligations, raw Cargo metadata snapshot and publication remain
unreviewed. This does not clear the flags in the preserved review draft.
[Native correspondence and snapshot issue](qa-261005-native-notice-source-correspondence.md)
remain separate from Rust wrapper/front-end license text.

## Immutable Cargo anchors

Each packaged .cargo_vcs_info.json supplies the exact commit. Official GitHub
tree responses return that exact revision without truncation. Each of ten
upstream crate Cargo.toml byte hashes equals its packaged Cargo.toml.orig hash,
independently establishing the selected subdirectory even when path_in_vcs is
absent. Workspace-inherited declarations are read from the same commit root;
versions/licenses agree with the normalized installed manifest.

| Package/version | Commit | Verified upstream crate manifest |
|---|---|---|
| alloc-stdlib 0.2.4 | `ae42d22078b98549e987d2f03d12df7b984fde47` | [alloc-stdlib/Cargo.toml](https://github.com/dropbox/rust-alloc-no-stdlib/blob/ae42d22078b98549e987d2f03d12df7b984fde47/alloc-stdlib/Cargo.toml) |
| rusqlite_migration 2.6.0 | `37c7a555018ecd80c58b2954779d83c281e4557a` | [rusqlite_migration/Cargo.toml](https://github.com/cljoly/rusqlite_migration/blob/37c7a555018ecd80c58b2954779d83c281e4557a/rusqlite_migration/Cargo.toml) |
| ts-rs 12.0.1 | `7182ad8289596097235406b715fa04506443b4ad` | [ts-rs/Cargo.toml](https://github.com/Aleph-Alpha/ts-rs/blob/7182ad8289596097235406b715fa04506443b4ad/ts-rs/Cargo.toml) |
| unic-char-property 0.9.0 | `5878605364af97a3358368a6eaef02104af2e016` | [unic/char/property/Cargo.toml](https://github.com/open-i18n/rust-unic/blob/5878605364af97a3358368a6eaef02104af2e016/unic/char/property/Cargo.toml) |
| unic-char-range 0.9.0 | `5878605364af97a3358368a6eaef02104af2e016` | [unic/char/range/Cargo.toml](https://github.com/open-i18n/rust-unic/blob/5878605364af97a3358368a6eaef02104af2e016/unic/char/range/Cargo.toml) |
| unic-common 0.9.0 | `5878605364af97a3358368a6eaef02104af2e016` | [unic/common/Cargo.toml](https://github.com/open-i18n/rust-unic/blob/5878605364af97a3358368a6eaef02104af2e016/unic/common/Cargo.toml) |
| unic-ucd-version 0.9.0 | `5878605364af97a3358368a6eaef02104af2e016` | [unic/ucd/version/Cargo.toml](https://github.com/open-i18n/rust-unic/blob/5878605364af97a3358368a6eaef02104af2e016/unic/ucd/version/Cargo.toml) |
| unic-ucd-ident 0.9.0 | `8a6ce83063d90b91ae2ce59eddb803edd393fca9` | [unic/ucd/ident/Cargo.toml](https://github.com/open-i18n/rust-unic/blob/8a6ce83063d90b91ae2ce59eddb803edd393fca9/unic/ucd/ident/Cargo.toml) |
| webview2-com 0.38.2 | `b74dc5e2b394044bea5191052868ce7a106c202c` | [crates/webview2-com/Cargo.toml](https://github.com/wravery/webview2-rs/blob/b74dc5e2b394044bea5191052868ce7a106c202c/crates/webview2-com/Cargo.toml) |
| webview2-com-sys 0.38.2 | `b74dc5e2b394044bea5191052868ce7a106c202c` | [crates/bindings/Cargo.toml](https://github.com/wravery/webview2-rs/blob/b74dc5e2b394044bea5191052868ce7a106c202c/crates/bindings/Cargo.toml) |

The root filenames are preserved: alloc-stdlib LICENSE; rusqlite_migration
LICENSE.txt; ts-rs LICENSE; both WebView2 wrappers LICENSE; each UNIC crate
LICENSE-MIT, LICENSE-APACHE and COPYRIGHT.md. UNIC identifier uses its distinct
commit; its three texts independently match the other revision's content, not
an assumed common release. Both alternative texts are retained without selecting
one license. WebView2 wrapper MIT text is not Microsoft SDK/Runtime authority.

## Frontend publication correspondence

[Version-specific npm publication metadata](https://registry.npmjs.org/@tauri-apps/plugin-dialog/2.7.3)
identifies @tauri-apps/plugin-dialog 2.7.3 and gitHead
`6aa2854f314481a459be1189b02c65a2450789ab`.
The [dialog-v2.7.3 GitHub tag](https://github.com/tauri-apps/plugins-workspace/releases/tag/dialog-v2.7.3)
resolves directly to that commit through the official ref API. Published npm
integrity equals bun.lock exactly:
`sha512-CRgE+7TP4tvq9MjBU6f04NLTFIqVMLKHk3hAqlhil00ngK9ACTrXPH3oHpKMProxILodd3YjBoKbMwSI4IEcfA==`.
The [upstream plugin package manifest](https://github.com/tauri-apps/plugins-workspace/blob/6aa2854f314481a459be1189b02c65a2450789ab/plugins/dialog/package.json)
SHA-256 `8ac162ad18e15a2ca4d2fe061be9e7aa8f519d19f248b21b3ffe8990363448cf`
matches local node_modules package.json byte-for-byte. This is stronger than
the incomplete installed SPDX declaration or branch-only README route.
Root LICENSE_MIT and LICENSE_APACHE-2.0 are collected from that immutable commit,
without choosing between them. No npm tarball/binary execution or install occurs.

## Text identity and pending boundary

[Machine-readable collection evidence](../../legal/third-party/upstream-source-evidence.json)
records package/version, upstream commit/path/URL, original byte count/SHA-256,
verified manifest SHA and local transcription count/SHA. Twenty Cargo plus two
frontend texts are checked against the complete fetched strings after only
CRLF→LF and one final newline. apply_patch output is therefore explicitly
transcribed, not represented as original archive/repository bytes.

All new text paths are under the existing package/version legal boundary;
preflight proves each destination absent before creation. Nothing is overwritten.
The original review draft and release manifest remain unchanged/pending. New
source facts are not linked to a verified current review snapshot: the changed
raw Cargo metadata digest remains unreconciled, despite matching runtime
identities and all recorded source-manifest hashes. A new reviewed snapshot is
a separate future step, not a quiet side effect of collection.

No manifest approval, generator guard bypass, release-candidate notice proof,
installed notice coverage or phase checkbox follows full-text collection alone.
Unresolved questions: snapshot reconciliation, selected license/NOTICE/source
availability obligations, final artifact/linker and installed coverage.

