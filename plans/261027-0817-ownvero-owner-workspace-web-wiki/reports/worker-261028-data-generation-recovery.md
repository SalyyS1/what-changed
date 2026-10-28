# Managed generation backup and recovery

Status: DONE_WITH_CONCERNS — source complete; controller compilation/native tests pending.

## Scope and ownership

Implemented new `src-tauri/src/app/data_restore.rs` and its helper modules,
`storage/data_generation.rs`, and the platform managed-file-lock facade/native
backend. Authorized narrow additions: platform/storage module declarations and
`OwnerStore::inspect_backup` in storage/lifecycle.rs. Root owns app module wiring,
startup/purge integration, `Db::is_closed`, docs, Cargo and final verification.
No actual profile reads, backup, restore or purge performed by worker. Official
Microsoft documentation consulted for the native rename diagnostic.

## Maintenance API

Public Rust API under `app::data_restore`; no new IPC or filesystem-path input:

- `ClosedStores::after_shutdown(&AppPaths, &DataLifecycle, &Db, &OwnerStore)`:
  requires producer admission closed/drained and both stores shut down; acquires
  lifecycle operation lock itself. Caller must not already hold that lock.
- `create_backup(&ClosedStores) -> BackupId`.
- `restore(&mut ClosedStores, &BackupId) -> RestoreReceipt`.
- `rollback(&mut ClosedStores, &RestoreReceipt) -> RestoreReceipt`.
- `recover_before_open(&AppPaths) -> RecoveryOutcome` before either DB opens.
- `purge_owned_generations(&AppPaths)` while producer admission is closed/drained.

IDs are canonical UUIDs; paths are derived from the exact resolved AppPaths layout.
Bounded namespaces: `data-backups/<UUID>`, `data-restores/<UUID>`,
`data-restore.json[.tmp]`. `data-generation.lock` contains no application payload.
Bounds: 20 generations/namespace, 4,096 inventory entries, 16 path components,
2 GiB total inventory bytes, bounded manifest/journal reads. No large limit corpus
is allocated by tests.

## Snapshot and switch contract

Both SQLite files use the SQLite backup API, including committed WAL pages.
Schema/integrity/ledger checks use existing migration authority; no migration is
applied during validation. A sealed snapshot contains self-contained DELETE-mode
DBs, only the managed owner-artifact tree, and a SHA-256 file inventory. The manifest
is written last. Profiles, cached exports, external exports, source projects, logs
and unrelated browser cache are excluded.

Native reservations pin destination parents and prevent destination replacement
while SQLite self-opens. Hashes are read through the same reserved handles and
checked again before sealing. Live DB/WAL/artifact fingerprints are compared
around snapshot creation; exact root SQLite SHM bookkeeping and zero-byte WALs
are excluded from that comparison. All sidecars remain held. Nonempty WALs,
journals, database pages and every artifact remain part of the fingerprint.

Restore validates a complete selected snapshot and creates a complete rollback
snapshot. A flushed journal binds both manifest digests and the purge epoch.
Prepared transactions recover the entire rollback generation; committed ones
finish the selected generation. Each DB, sidecar and artifact-file move occurs
with stores closed. The old complete snapshot remains available for explicit rollback.

Native move handles deny external SQLite readers/writers and retain DELETE access.
SetFileInformationByHandle renames the exact validated object using an absolute
destination and pinned ancestors of both source and destination. Reparse points, hard links,
Windows path aliases, unexpected destinations and final-handle path mismatches
fail closed. Held bytes are rehashed before promotion. The complete live inventory,
file/directory handle locations and hashes are verified before commit.

## Privacy precedence and startup

Pending purge removes the exact backup/staging/displaced/journal namespaces before
restore can replay anything, even if restore metadata is corrupt. The helper returns
`PurgeRequired`; the global purge owner still deletes live stores and must not call
that helper outcome a completed purge. Completed purge epoch tombstones invalidate
older manifests/journals. No restore can move the epoch backward.

Read-only review of root wiring confirmed recovery runs before either store opens;
global purge includes helper namespaces. Found/fixed a missing-root startup regression:
recovery now validates existing ancestors and returns Clean without creating data
when a fresh app root is absent.

## Verification fixtures implemented

- Actual committed WAL rows in both SQLite backups; no output WAL/SHM dependency.
- Every prepared/displaced/promoted/committed switch boundary: 22 interruptions,
  recovery to matching two-DB/artifact contents, then idempotent clean recovery.
  Includes every artifact file in each phase and the boundary before empty
  source-directory cleanup. Corpus has a Unicode absolute temporary data root,
  nested portable ASCII artifact names, an empty directory and three distinct
  files. Native direct-rename tests also cover Unicode filenames.
- Complete restore and explicit rollback; unrelated source file remains untouched.
- Partial snapshot, checksum mismatch, newer schema and migration-ledger drift.
- Pending purge cleanup and completed-tombstone rejection of old snapshots.
- Open external SQLite handle exclusion; compatible exact-handle rename.
- Hard links, an unprivileged temporary junction, path aliases, source-name
  substitution, unexpected destinations, staging-parent replacement and SQLite
  reservation/self-open behavior. No silent junction-test skip.

Direct rustfmt passed. Controller's initial native gate: 525 passed, 10 failed,
3 ignored. Worker ran one authorized narrow restore reproduction: failed before
switch, with identical main DB/artifact hashes and only two newly created empty
WAL files. Diagnostic output removed after confirming the cause. Controller owns
post-fix verification.
All out-of-line test files begin with `#![cfg(test)]`.

### Native gate fixes, 2026-10-28

- `RootDirectory` plus relative leaf returned Win32 87 on this host. An isolated
  ctypes probe matched the native access/sharing/layout: failed with 0/2/8 extra
  buffer bytes; null root plus absolute destination succeeded in all three cases.
  This differs from the current [Microsoft structure documentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info).
  Source identity still uses the held handle; every ancestor is pinned. Added a
  real cross-parent Unicode file and directory regression.
- SQLite read-only inspection of a clean WAL-mode source creates empty WAL/SHM
  companions. Actual narrow Rust diagnostic and an isolated SQLite reproduction
  confirmed main database bytes unchanged. Normalize only exact root empty WALs;
  preserve every lock and nonempty-WAL comparison. Artifact names ending `-shm`
  are now explicitly hashed. Added a regression covering all three cases.
- Controller rerun after those fixes: 536 passed, 2 failed, 3 ignored. Native
  low-level moves and reviewed bundle saves passed. Remaining restore failures
  had two separately verified causes, corrected before the next gate:
  - [Microsoft MS-FSA FileRenameInformation](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/87f86c9b-6c2a-4803-84b7-131a74a434fa)
    refuses directory renames while descendants are open. Standalone native
    probe confirmed success with no child handles; holding either a child file
    or child directory returned access denied, including extended rename flags.
    Artifact promotion now retains every exact content handle, creates fresh
    pinned destination directories, moves files individually, then removes only
    empty source containers. Existing destination trees are never merged. The
    same whole-generation journal remains Prepared until complete verification.
  - Absolute Win32 rename path conversion requires a trailing UTF-16 zero.
    The old aligned allocation sometimes had padding but did not guarantee a
    terminator. Probe observed success with a garbage Unicode path suffix;
    explicitly terminated buffers produced exact paths at all eight tested leaf
    lengths. Buffer now always reserves the terminator; FileNameLength excludes
    it. Added eight-length native exact-path regression.
- Initial focused rerun caught a fixture-policy mismatch: the managed inventory
  intentionally permits portable ASCII relative artifact components. Kept that
  production policy; changed only test relatives to ASCII and moved Unicode
  coverage to the absolute temporary data root. Direct native Unicode filename
  coverage remains. Controller rerun pending.

## Remaining acceptance

- Controller compilation, focused native/storage/restore suites and existing
  lifecycle/privacy/architecture gates must pass before claiming verified delivery.
- UI/installed restore acceptance is separate from this public Rust maintenance API.
- Process interruption fixtures do not establish sudden-power-loss behavior on every
  filesystem/storage controller; no such hardware claim is made.
- Future persisted artifact-reference schemas must extend storage validation; current
  schemas have no artifact-reference table to invent or silently assume.

## Unresolved questions

None blocking source implementation. Native verification and integration evidence pending.
