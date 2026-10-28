# Continuation evidence — 2026-10-26

Scope: accepted full MVP, `--auto --advise`. Phase sweep: **316/369 checked,
53 open**. No acceptance checkbox changed in this run.

## Current-account authorization and data preservation

The user explicitly authorized using the current Windows account, superseding
the earlier separate-QA-account requirement. No further account creation is needed.
This does not authorize publication or imply installed acceptance passed.

An online SQLite backup opened the original database read-only and created
`.qa/native-profile-backup-20261026/events.db` (434,352,128 bytes). `PRAGMA
quick_check` returned `ok` both after backup and after closing the connections.
The finalized local manifest hashes the database, startup marker and window-state
file. Temporary WAL/SHM entries were removed from the manifest after connection
closure. The original profile was neither renamed nor replaced.

The existing engineering app (PID 17316) was started with `--minimized`. The
native Quit attempt verified its tray/menu owner and the live `Quit` item before
dispatch. UIA could not expose the item; Win32 inspection found the expected
menu. A verified command did not terminate the process within 30 seconds. A
physical-click fallback refused to act because another window covered the item;
no pointer input was injected. One verified-owner menu cancellation returned
success, but the process remained alive. Follow-up inspection found a clean
startup marker, no collector workers and no remaining popup. Exit-path diagnosis
was narrowed to framework exit dispatch. One PID/class-verified internal paint
request then allowed actual process exit. This assisted recovery does not prove
ordinary Quit acceptance. No forced termination or data-directory switch occurred.

Two attempts to prepare a fresh native profile stopped before isolation. The
first exposed a Windows PowerShell null-string binding error in journal
replacement; an explicit previous-journal path fixed it and three real writes
passed. The second could not rename the original directory (`Access denied`).
Both complete offline backups remain preserved with matching file manifests;
the original still has all 13 files (447,312,429 bytes). Neither attempt started
a QA app or moved the original profile. No ACL change or elevated retry occurred.

See [native QA route](native-qa-route-261026.md) for artifact identities and
guarded backup/restore procedures. The preserved installer is old and its notice
payload remains pending; it is not a current release candidate.

## Collector performance: bounded repeat experiment

The recovered build session no longer existed. A new serial release invocation
used `.qa/collector-performance-release-20261015` and completed after 15m 07s:

```text
cargo test --release --locked --manifest-path src-tauri/Cargo.toml
  --features devtools --test collector_performance -- --nocapture --test-threads=1
```

Initial result: listener **1.9762 ms** passed; process **92.4285 ms** failed the
unchanged **15 ms** p95 threshold. CPU/process counts were not captured for that
sample, so its cause is not established.

After advisory review, exactly three sequential runs used the same existing
test executable without rebuilding. SHA-256:
`E91F8132B80417F163996EA4C2A99566D762C164EAFBAE0AB6DA41428CF9F69D`.

| Run | Process p95 ms | Listener p95 ms | CPU before → after | Processes before → after | Exit |
|---|---:|---:|---|---|---:|
| 1 | 17.4784 | 0.3426 | 23% → 15% | 331 → 331 | 101 |
| 2 | 57.5130 | 2.1934 | 79% → 98% | 343 → 349 | 101 |
| 3 | 86.1510 | 4.5333 | 100% → 100% | 349 → 379 | 101 |

Raw logs and before/after timestamps are in
`.qa/collector-performance-release-20261015/repeat-20261026/`. Aggregate CPU
samples are workload indicators, not measurements of each timed cycle. All three
process runs failed; even the least-loaded run missed. No additional retries or
provider changes were made. P09 remains **11/12**. The next diagnostic should
separate provider enumeration, merge/map publication, acknowledgement and
destruction costs before selecting a change.

A subsequent [source scout](process-cost-scout-261026.md) identified an existing
provider trait boundary. A separate ignored test now measures paired full-cycle,
provider, remaining snapshot, acknowledgement-region and destruction timings;
result-kind counts distinguish quiet and staged cycles. It preserves the provider's
reused Vec and the existing acceptance tests. The acknowledgement region includes
classification/branch overhead; instrumentation overhead stays in the full timer.
Compilation passed; the one explicitly invoked diagnostic completed **1 passed,
2 acceptance tests filtered out**, exit 0. Artifact SHA-256:
`926ED422C9F788934FE10C541B1B2C0F1DD1759C6CAF4A84B08E40CE48ADCE31`.
Raw evidence: `.qa/process-cost-run-20261026/{diagnostic.log,manifest.json}`.

All **50 cycles were Noop**, with **334 provider rows** in the printed slow tuples.
Total p50/p95 **11.6163 / 12.6546 ms**; provider **11.5798 / 12.6185 ms**;
snapshot remainder **0.0358 / 0.0406 ms**. Acknowledgement-region p95 was
**0.0001 ms**, destruction **0.0004 ms**. The slowest same-cycle tuple was
**17.2403 ms total / 17.1948 ms provider**. Host CPU snapshots were **39% → 23%**,
process counts **335 → 333**, with no compiler running. These are host snapshots,
not per-cycle CPU attribution. Provider enumeration dominates this quiet sample;
Atomic/staged behavior and the cause of the earlier failed runs remain unproven.
No production optimization or acceptance pass is inferred from this diagnostic.

Advisory review authorized exactly one subsequent invocation of the unchanged
original tests, justified by the new paired evidence, with no retry regardless
of outcome. That same `926ED422…` executable completed **2 passed / 1 diagnostic
ignored**, exit **0**: process **13.7265 ms p95**, listener **0.3222 ms p95**.
Host CPU snapshots **42% → 34%**, process counts **343 → 342**. Logs and manifest:
`.qa/process-acceptance-after-diagnostic-20261026/`. Targeted strict Clippy passed
after adding the diagnostic. The original 50-cycle/full-region/15 ms tests remain
unchanged; no production optimization or threshold change was made.

This is a current-artifact acceptance **run** pass. P09 nevertheless remains
**11/12**: its existing verification record explicitly requires a calm,
repeatable result after substantial variation and says one calm pass is
insufficient. No further repeat was performed. Provider-internal attribution,
staged-cycle cost and reliability across ordinary host loads remain unresolved.

The test measures 50 full snapshot cycles including acknowledgement hooks and
snapshot destruction; it does not include durable SQLite commits. Its shared
mutex now recovers poison so a failed process assertion cannot mask the independent
listener result. Threshold, sample count and timed region are unchanged.

## Unresolved

- Fresh-profile isolation remains unavailable; existing-profile native checks below passed.
- Process collector repeatability: four initial runs missed 15 ms; the one
  original acceptance invocation after paired diagnosis passed. Existing P09
  repeatability requirements remain unmet.
- Soak hardening is verified: 8 unit / 5 observer / 1 join tests, PowerShell
  negatives, library 571 passed / 3 ignored, strict Clippy and devtools build.
  Bounded live smoke correctly rejected unavailable Docker. See
  [hardening evidence](soak-hardening-261026.md).
- Remaining installed, performance, accessibility, legal and publication gates stay open.

## Standalone native UI — completed continuation

The first existing-profile QA executable loaded `chrome-error://chromewebdata/`
with `unreachableUrl: http://localhost:5183/`. It had been built through raw Cargo
without `tauri/custom-protocol`. No Vite server was started to conceal that defect.
The new `bun run build:standalone` route builds the frontend and enables the
embedded protocol explicitly. Frontend build passed (837 modules); Cargo release
compilation passed in 8m 12s. The initial wrapper then failed at `Get-FileHash`
discovery under nested Windows PowerShell; this was a post-build script failure,
not a successful end-to-end command. The corrected script uses disposed .NET
SHA-256 objects and Cargo's exact `compiler-artifact.executable` output, including
configured target directories. Final command verification is recorded separately.

Corrected `bun run build:standalone` then completed **exit 0**: frontend 837 modules,
Cargo release 10m 14s, exact output
`.qa/collector-performance-release-20261015/release/what-changed.exe`, SHA-256
`4DAB1F31BAC288E308D6801D262EEB899DA31372345789FE375A422C741308C8`.
Log: `.qa/standalone-build-final-20261026.log`. PowerShell 5.1 parsing and 14
focused wrapper assertions passed; independent review found no remaining source
issue. This second artifact was not substituted for the separately identified
native-tested executable below; binary byte reproducibility is not claimed.

Verified engineering executable: **15,360,000 bytes**, SHA-256
`FF64A0D867C56C19802E321A4E59C7278BB911B55C4DA78DC20FCF7C24F26162`.
Native process 4216 used the original profile with verified backups in place:

- Embedded URL `http://tauri.localhost/`, no unreachable URL; port 5183 absent.
- Real `get_app_status`: database ready, tray available, safe mode false,
  dropped writes 0 at the sampled checkpoint.
- Home, Timeline, Investigations, Checkpoints, System Map, Settings and Diagnostics
  opened via keyboard shortcuts. All seven headings and current-navigation states
  appeared; no document overflow, rendered error state or unhandled JavaScript error
  was observed in this bounded traversal. Viewport 1280×678, device pixel ratio 1.5.
- Ctrl+K focused the search input; Escape closed its dialog. The Add workspace
  control was visible/enabled; Privacy was reachable by scrolling. No workspace,
  checkpoint, investigation or persisted setting was created or changed by QA.
- CDP **viewport emulation** at 900×600 passed document/main horizontal-overflow
  checks on Home, Timeline and Settings. This does not prove native minimum-window
  sizing, Windows DPI changes, 200% zoom, every control or Narrator behavior.
- Real tray Quit exited both the preceding monitoring process and this QA process
  normally. QA exit code **0**, no forced termination or internal-paint recovery.

Local screenshots and JSON evidence: `.qa/native-ui-20261026/`. Session record:
`.qa/native-standalone-session-20261026.json`. These contain current-profile data
and are not publication/launch assets. Earlier assisted-exit evidence remains
historical; these later ordinary-Quit successes do not erase it.

While its destination was stopped, the verified standalone executable replaced
the old engineering binary at `src-tauri/target/release/what-changed.exe`. The old
binary and its hash remain in `.qa/native-pre-standalone-executable-20261026.exe`
and `.qa/native-standalone-replacement-20261026.json`. The harness relaunched that
verified binary with `--minimized` (PID 27156) and no debugging arguments. Original
app data stayed in place throughout. No installer, fresh onboarding, migration,
four-hour soak, network monitor, screen-reader or publication acceptance is claimed.
