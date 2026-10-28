# Offline owner workflow continuation

Status: bounded continuation verified; parent phases remain in progress. Branch
`codex/ownvero-owner-workspace`; no commit, push, publication or release approval
in this continuation.

## Source identity

Base HEAD: `13d60df4c98fd50532c3d4a338fd6eb7be6c0db8`; existing dirty work preserved.
Private `.qa/ownvero-preflight/offline-continuation-source-manifest.json` records
the same 822 source/configuration/test paths as the earlier manifest. Four changed:
`db.rs`, `writer.rs`, `wiring.rs` and `BundleExportReview.svelte`.

Continuation digest:
`849f160273c2cf1c35bb3b7699951726b7f2c241624507b157e09dfe554426b1`.
Algorithm: ordinal-sort relative paths; concatenate each UTF-8 record as
`path + U+0000 + lowercase file SHA-256 + LF`, without BOM; hash the concatenation.
The earlier manifest is retained unchanged. This is source identity, not a clean
commit or release approval.

## Repairs and completed checks

- Restored the established typed writer-job reentrancy error at both early
  database admission guards. Their positions before admission/inner locks remain
  unchanged. Lifecycle-specific errors remain separate. A shared constructor
  prevents the writer and database facade from drifting again.
- Both unchanged collector reentrancy integration regressions passed, including
  zero durable residue. All 13 database lifecycle unit tests passed, including
  nested `write`, `write_at_epoch`, `flush`, recovery and pending-ticket close.
- Rustfmt check passed. Independent scoped source review found no blocking
  issue in the reentrancy repair. This is not a completed full-suite result.
- The wiring check now recognizes exactly the existing feature-gated explicit
  `--soak` diagnostic dispatch, while retaining the three normal GUI exit paths.
  Wiring 5/5 and workspace-file investigation 6/6 passed.
- Expanded selected contract targets: 53 passed, 1 release-artifact audit ignored
  across their final runs. This count includes the wiring/workspace checks above.
- Export review copy now explains the separate replacement confirmation already
  implemented by the backend. Latest frontend typecheck: 0 errors, 0 warnings.
  An earlier attempt failed with ENOSPC; that environmental failure is retained.
- Final current-source checks passed: typecheck (0 errors / 0 warnings), rustfmt,
  docs (35 Markdown files and relative links), and the Windows core no-network
  dependency gate. The latter is not runner network-confinement evidence.

## Interrupted broad Rust run

Command: `cargo test --manifest-path src-tauri/Cargo.toml --locked --all-features
--no-fail-fast -- --test-threads=1`, with debug info/incremental disabled and one
build job. The run exhausted D: while writing its log during `repository`.
There is no complete Cargo exit/result; the harness exit is not an acceptance.

The 87 completed target summaries contain 1,066 passed, 5 failed and 31 ignored.
Library: 626 passed / 3 ignored; seed-history binary: 23 passed / 3 ignored.
These are historical results before the latest reentrancy repair.

Captured failures:

| Check | Evidence / current state |
|---|---|
| Two collector reentrancy checks | Repaired; both and 13 lifecycle unit tests subsequently passed |
| Persisted 3,000-item checkpoint comparison | p95 489.4155 ms against 150 ms; later bounded current-source target passed 4/4 |
| Real process collector cycle | p95 125.2359 ms against 15 ms; later tool-context failure and desktop pass recorded below |
| System Map, 50,000 relationships | first 307.8824 / p95 346.0383 / max 363.3029 ms against 120 ms; later bounded target passed |
| Repository 10,000-event insert | Failure captured before log interruption; later complete target passed 28/28 |

The isolated System Map rerun also failed: first 110.1101, median 124.2369,
p95 194.393, max 294.7356 ms. No threshold was changed. Separate foreign Rust
builds and disk pressure were observed; neither is a proved cause of latency.

Read-only Python SQLite attribution on the existing synthetic C: fixture found
the exact direct-neighbor count cost about 40–47 ms; incoming page about 2.3 ms,
availability about 2.5 ms and selected-node metrics about 2.7 ms after first read.
These use a different SQLite build and are diagnostic, not Rust acceptance.
Exact reachable-node disclosure was retained; no query/schema change made.

## Bounded current-source performance results

Command selected `checkpoint_compare`, `collector_performance`, `perf_system_map`
and `repository`, with `--locked --all-features --no-fail-fast` and one test thread.
Exit 101: **34 passed, 1 failed, 1 ignored**. This preserves the actual failing
combined result; no timing limit was changed.

| Target | Result |
|---|---|
| checkpoint_compare | 4/4 passed |
| perf_system_map | 1/1 passed; first 75.8965 / p95 80.6248 / max 86.0213 ms, limit 120 ms |
| repository | 28/28 passed; 10,000 inserts in 1.8273349 s, 5,472 events/s |
| Listener collector | Passed; p95 362.1 microseconds |
| Process collector, tool context | Failed; p95 18.749 ms, limit 15 ms |
| Paired process diagnostic | Explicitly ignored in acceptance; separately invoked below |

Paired tool-context diagnostic: 50 cycles, 396 rows, all Noop. Total p95
19.2644 ms; provider p95 19.0941 ms. At that paired slow cycle, snapshot remainder
was 0.1699 ms, acknowledgement region 0.0001 ms and destruction 0.0003 ms.
Instrumentation measures aggregate provider cost; it does not identify an
individual Win32 call or establish the cause of host variability.

A single controlled comparison then used the **same hashed binary** in a normal,
non-elevated Windows desktop context. Its first harness attempt was incomplete:
PowerShell 5.1 treated timing stderr as a terminating native-command error;
acceptance/diagnostic exit receipts were null. The partial 18.0537 ms observation
is preserved and is not a completed test result.

After advisory review, the corrected harness drained stdout/stderr concurrently,
retained process handles, imposed deadlines and captured actual exit codes:

- Desktop acceptance: **1/1 passed**, exit 0, p95 **13.6374 ms** against 15 ms.
- Desktop paired diagnostic: exit 0; total p95 13.4113 ms, provider 13.2968 ms;
  50 Noop cycles / 396 rows. Diagnostic timing is not acceptance.
- Both owned tests and their controller exited normally; no timeout or forced kill.
- Binary SHA-256:
  `866a9c80841d286056776f5b7f254734f6575871bb556600c166bf35123dee5a`.

The desktop pass and tool-context failure coexist. This is context-associated
variability, not a proved environmental cause or a repaired performance defect.
No provider cache, API, query, schema or threshold was changed. Full verification
remains open; another broad build was deferred after resource review because D:
had about 2.1 GB and C: about 0.58 GB free after an earlier ENOSPC interruption.

## Storage and process custody

- D: reached zero free bytes; C: also reached zero earlier.
- NTFS compression recovered C: registry-cache capacity. No registry file deleted.
- Automatic approval review initially rejected exact cache deletion. After the
  user explicitly renewed safe-cache cleanup authorization, 248 generated debug
  PDB files were removed, recovering about 1.16 GB. Executables were preserved.
- Compressed 60 large synthetic fixture files, recovering about 0.34 GB; and
  13 old benchmark compilation objects, recovering about 0.86 GB. No fixture,
  source, installer or backup deleted by these operations.
- Removed 139 generated PDB files from that old benchmark cache, recovering
  about 0.25 GB. Its release executables remain intact.
- Recursive fixture deletion remained rejected by automatic approval review;
  no alternate deletion attempted. Reparse fixtures remain untouched.
- Later compilation consumed some reclaimed space. Before native QA preparation,
  D: had about 1.70 GB and C: about 0.95 GB free. These are observations, not quotas.
- A further 9 generated debug PDBs recovered about 0.13 GB. Removing 1,069 old
  benchmark compiler objects recovered about 1.34 GB. All removals were bounded
  to generated cache files; executables, DLLs, installers, logs and backups remain.
  Cumulative observed recovery from these symbol/object removals: about 2.9 GB.
- The owned cache-compression process was stopped after its output redirection
  failed. Foreign SLTerm Cargo processes were identified read-only and preserved.

## Native debug acceptance

Actual Tauri UI, native clipboard and native Save/Open/import round trip passed
with synthetic data. User authorized own-account QA with verified backup,
isolation and restoration. The original profile is approximately 689.7 MiB.

Initial launch stopped at the Rust-process preflight, before profile access or
isolation. Review then identified cleanup/journal and picker ownership gaps;
the controller now independently establishes quietness after stop errors,
requires verified restoration, and freshly checks dialog/process/path custody.

The first content-isolation attempt found a tool-context filesystem view whose
new sibling resolved to a different volume. It refused the move and rolled back
before app launch. A fresh normal-desktop probe verified account/session,
non-elevation, NoPackage, WinSta0/Default, original file hashes and physical volume
identity. The reviewed desktop launch route then ran the isolated synthetic session.
It was never used to retry the policy-denied fixture deletion.

Final phases passed: `resume-prepare`, `copy`, `bundle`, `import`, `confirm-import`.

- Confirmed rule: `Staff can see a saved synthetic booking.`
- First saved request: `Thêm ghi chú cho lịch hẹn QA`; reviewed Markdown copied
  through the native clipboard and matched exactly. Original clipboard restored.
- Second request: `Add a reminder to the synthetic booking`; confirmed rule reused.
- Actual Windows Save and Open dialogs exported/imported a 3,751-byte ZIP.
  Entries: `manifest.json`, `workspace.json`, `README.md`, `recipes.json`.
  Both requests and the rule are present. Imported app ID is distinct; historical
  evidence remains untrusted.
- ZIP SHA-256:
  `3e063440e25847173628be5916bd2e656807e9b64c016bcefb0ef01ed58d571e`.
- Debug app SHA-256:
  `2ff6e976be1b08ac60adb73e660d6f3b07951eaea576f3d4e22f6e46794d08d9`.
- App and controller exited with code 0, observed without forced cleanup.
  Original profile verified **Restored**; both original backup/journal sets retained.
  No original app process was running at entry, so no original process was restarted.

Custody proof: private `.qa/native-owner-final-evidence-20261028.json`, session
and profile-content journals, native clipboard result and WMI launch transcript.
This proves a synthetic current-source debug round trip. It does not establish
installed-release, populated-evidence or real-owner acceptance.

## Logs

Private logs under `.qa/ownvero-preflight/`:

- `offline-continuation-contracts.txt`, `offline-continuation-wiring-repair.txt`.
- `offline-continuation-full-rust.txt` (interrupted; retains failures).
- `offline-writer-reentry-repair.txt`, `offline-writer-lifecycle-repair.txt`.
- `offline-continuation-frontend-check-recovered.txt`.
- `offline-fixture-compression-result.json`, `offline-old-objects-compression.json`,
  `offline-old-symbol-cleanup.json`, `offline-rebuilt-symbol-cleanup.json`,
  `offline-old-compiler-cache-cleanup.json`.
- `offline-performance-recovered.txt`, `offline-process-cost-recovered.txt`.
- Final current-source checks: `offline-continuation-typecheck-final.txt`,
  `offline-continuation-rustfmt-final.txt`, `offline-continuation-docs-final.txt`,
  `offline-continuation-no-network-final.txt`.

Private desktop comparison files under `.qa/`:
`native-process-perf-context-20261028.json` (incomplete initial attempt),
`native-process-perf-complete-context-20261028.json` and its separate acceptance/
diagnostic text logs. Timing diagnostics omit enumerated process identities and
paths; private context records contain execution identity and executable paths.

Initial disk-pressure and isolated-performance evidence also remains in the
local Codex visualization artifact directory; it contains synthetic QA only.

## Final reconciliation

- Rehashed all 822 continuation-manifest files: zero mismatches.
- Final process check: zero project app/test/build/dev-server processes; no
  listener on ports 1420, 5183 or 9223. Native helper exit receipts are retained.
- Free-space observation: C: 600,596,480 bytes; D: 2,095,775,744 bytes.
  Evidence: `.qa/ownvero-preflight/offline-continuation-final-state.json`.
- Six affected plan/report/journal documents: 35 local links checked, none broken.
  Scoped source whitespace check passed. Independent report review found no
  blocking completion/custody/performance claim; historical wording was clarified.
- Native evidence, ZIP, source manifest and original-profile backup remain
  gitignored. No additional deletion attempted during final reconciliation.
- Local journal created through `ak journal create`. AgentWiki publish skipped.

## Remaining gates

P03–P06 remain in progress. Full verify, repeatable performance, installed upgrade/downgrade,
accessibility/resource acceptance and real-owner two-change rehearsal remain open.
Before P07: clean-Windows packaging, enforceable runner network confinement and
the real-owner rehearsal. User has no project/participants and chose to retain
that gate. G2/G3 remain unapproved.

Unresolved questions: no new product decision requested. External acceptance
evidence and resource headroom remain required.
