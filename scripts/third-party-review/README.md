# Pending third-party review preparation

`bun run prepare:third-party-review` resolves the locked, offline Windows Cargo
tree with `--edges normal,no-proc-macro`, reads matching Cargo metadata and the
schema-2 `dist-installer/frontend-inputs.json` produced by `bun run build`, and
writes `legal/third-party/review-draft.json`. Run after the build, with no other
Cargo job active. This command does not build or download dependencies.

Original top-level LICENSE, LICENCE, COPYING, COPYRIGHT and NOTICE files are
copied byte-for-byte under `legal/third-party/<ecosystem>/<name>/<version>/`.
Each file has a SHA-256 record. Multiple license texts stay separate;
COPYRIGHT files stay attribution records and do not satisfy license-text coverage.
`LICENSE.spdx` stays a declaration document and cannot satisfy the full-text
requirement. Package manifests and original Cargo manifest/VCS metadata are
hashed in the draft. `libsqlite3-sys` also records the version, source ID and hash
read from `sqlite3/sqlite3.c`; this does not prove which native variant a final
binary linked.

`webview2-com-sys` records one pending `WebView2Loader` component for
`x86_64-pc-windows-msvc`: the SHA-256, package-relative source path and byte
length of `x64/WebView2Loader.dll`, `x64/WebView2Loader.dll.lib`, and
`x64/WebView2LoaderStatic.lib`, plus `build.rs` and `src/lib.rs` as selection
evidence. These are packaged candidates and source evidence, not proof of what
the final binary linked. Missing, empty or escaping required files stop preparation
before output writes. Native binaries are hashed in place, never copied as license
texts. The SDK version and native license expression remain null; the crate's
version and MIT declaration do not resolve them or satisfy native license review.

The output is always pending: `reviewedBy` is null, `selectedLicense` is null,
`obligationsReviewed` and `publishable` are false. Missing declared expressions
and missing full texts appear in each entry's `issues`. The command neither
updates `scripts/third-party-notices-manifest.json` nor emits the publishable
`dist-installer/THIRD_PARTY_NOTICES.txt` payload. The existing release gate still
requires exact-version review and frozen artifact evidence.

For precomputed inputs (avoids any Cargo invocation):

```powershell
node scripts/prepare-third-party-review.mjs --cargo-tree path/to/tree.txt --cargo-metadata path/to/metadata.json --input-snapshot path/to/snapshot.json
```

The snapshot must bind those files to the same locks, Cargo manifest, and frontend
inventory. Use SHA-256 of the original bytes and these exact fields:

```json
{
  "schemaVersion": 1,
  "target": "x86_64-pc-windows-msvc",
  "cargoEdges": "normal,no-proc-macro",
  "cargoLockSha256": "<hash of src-tauri/Cargo.lock>",
  "bunLockSha256": "<hash of bun.lock>",
  "cargoManifestSha256": "<hash of src-tauri/Cargo.toml>",
  "frontendInputsSha256": "<hash of dist-installer/frontend-inputs.json>",
  "cargoTreeSha256": "<hash of the saved --prefix none --format {p} Cargo tree>",
  "cargoMetadataSha256": "<hash of the saved --filter-platform Windows Cargo metadata>"
}
```

Stale snapshots, unexpected installed versions, ambiguous Cargo source identities,
unsafe names, escaping symlinks and different existing original-text bytes stop
preparation before output writes. The frontend entries are parsed from the bytes
whose hash is recorded, and all input hashes are checked again before any output
mutation. Repeating the same inputs yields the same draft
bytes. Regenerating a reviewed draft is refused. Text copies created for a pending
draft remain source evidence only; license choice, attribution, NOTICE obligations,
native components and source availability still need review.

## Reviewed native notice gate

The notice generator requires exactly one reviewed `SQLite` component for
`libsqlite3-sys` and one reviewed `WebView2Loader` for `webview2-com-sys`.
Additional declared native components are validated too. Each component needs
its own `licenseExpression`, `selectedLicense`, `obligationsReviewed: true`,
authoritative HTTPS `sourceUrl`, `textFile` and `textSha256`, and an explicit
`noticeFiles` array. Copyleft choices also require HTTPS source availability.
Component `reviewedBy` and `reviewedAt` may inherit the reviewed manifest's
reviewer and date; wrapper MIT approval cannot replace these native decisions.

The manifest must bind `cargoManifestSha256` to `src-tauri/Cargo.toml` when native
components are present. After the pending-manifest guard, the generator resolves
locked offline Windows Cargo metadata and rereads contained installed package
files. SQLite version/source ID and source bytes, and all three WebView2 x64
candidate records plus both selection files, must exactly match reviewed evidence.
A null native version requires exact hashed bytes, `provenanceReviewed: true`,
and an authoritative HTTPS `provenanceUrl` (or the authoritative `sourceUrl`).
WebView2 SDK versions also require that explicit provenance review because the
wrapper does not establish an SDK version. Native sections are emitted separately
only after every check passes. The exported generator's `cargoTree` and
`cargoMetadata` file overrides support isolated fixture checks without Cargo.

This is a prebuild license/source-candidate freshness check. It does not establish
which components a frozen executable or installer contains; final artifact binding
and actual production legal review remain required.
