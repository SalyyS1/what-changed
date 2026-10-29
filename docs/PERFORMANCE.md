# Performance

**Evidence state: verified here only for rows with a named command and date; missing hardware gates remain known limitations.**

Measured numbers only. Every row in this file was produced by a script in
`scripts/`, or by a named test/example, on the machine and date shown — nothing here is
an estimate, and no figure is carried over from documentation or from another
machine. Each section names the command that reproduces it.

## Memory

Reference line from the product brief: **80 MB**. The brief does not say which
figure it means, and the two measured figures land on opposite sides of it — see
D-15 in `DECISIONS.md` for how that was resolved.

Reported as four figures, because a WebView2 app has no single one:

| Figure | Meaning |
|---|---|
| backend RSS | working set of `WhatChanged.exe` by itself |
| WebView2 sum | working set of every `msedgewebview2.exe` descendant (broker, GPU, renderer, utility) |
| total | the sum — what Task Manager shows the user |
| private WS | the part of that total which exists *because this app is running*; shared Chromium pages, mapped identically into every other WebView2 and Edge process on the machine, are excluded |

Both of the last two matter and neither replaces the other. Total is what a user
sees and complains about. Private working set is what the app actually costs the
machine, and it is the only one of the two that moves when the app is optimized —
a Chromium page shared with an already-running Edge is counted in total but costs
nothing extra.

**Decision D-15 (user, 2026-09-03): both are recorded at every checkpoint against
the 80 MB reference line, and neither is a pass/fail gate.** The "Verdict" column
below therefore states where each figure sits relative to the line — it is a
reading, not a grade. The honest consequence is that memory is a measurement in
this release, not a budget; the trend rows are what a future scope conversation
stands on.

Reproduce with the release build running:

```
bun run mem
```

or, to record a row directly:

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1 -Append -Label "P01 baseline"
```

What is still enforced regardless of D-15: both figures appear in every row (a
row with one is incomplete), the WebView2 share is broken out, no metric is
silently redefined, and collectors are never disabled to make a number look
better.

| Date | Commit | Label | Backend RSS | WebView2 sum | Total | Private WS | Verdict |
|---|---|---|---|---|---|---|---|
| 2026-09-03 07:15 | uncommitted | P01 baseline, release build, ~20 s after launch | 27.8 MB | 350.8 MB | 378.6 MB | 103.3 MB | total over by 298.6 MB |
| 2026-09-03 07:27 | uncommitted | P01 baseline, same process, ~12 min idle | 27.6 MB | 334.1 MB | 361.7 MB | 77.1 MB | total over by 281.7 MB; **private under target** |
| 2026-09-19 19:36 | 5107ac0 | P05 release gate #2, DB open and tray active, 10 s after launch | 41.9 MB | 405.1 MB | 447.1 MB | 145.5 MB | total over by 367.1 MB; private over by 65.5 MB |

The P05 checkpoint was measured from the optimized `WhatChanged.exe` produced by the same successful
Tauri build that generated `What Changed_0.1.0_x64-setup.exe` (3,196,937 bytes; SHA-256
`570825495FA297409D3C1141E71059A16E105D97FE6F356A3C5A4E07EFB78EE5`). Six WebView2 descendants were
included. Per D-15, this row is evidence rather than a pass/fail budget: both total and private working
set are above the 80 MB reference line, and no collector was disabled for the measurement. This closes
the Phase 05 gates only; it is not evidence of a published or signed release.

### Installed engineering snapshots — 2026-10-27

The [repaired candidate](../plans/reports/qa-261027-0619-repaired-installed-candidate.md)
was measured with `scripts/measure-memory.ps1 -SettleSeconds 0` and six WebView2
descendants. CDP was enabled for native QA; these are not production idle or
four-hour acceptance readings.

| State | Backend WS | WebView2 WS | Total WS | Private WS |
|---|---:|---:|---:|---:|
| Diagnostics shown | 92.9 MB | 507.6 MB | 600.5 MB | 250.3 MB |
| Hidden after navigation/maps | 99.2 MB | 553.7 MB | 652.9 MB | 276.7 MB |

Both total and private readings exceed the 80 MB reference line. Per D-15,
neither is a pass/fail gate. Private logs are named in the report. The required
three checkpoints on one four-hour run remain open.

### P01 baseline — what these numbers mean

Measured on the release build of the Phase 01 scaffold: one window, one IPC
command, no database, no collectors, no Three.js. So this is **the floor of a
WebView2 shell on this machine, not the cost of any feature.** Six WebView2
processes (broker, GPU, network, two renderers, utility) account for ~92% of the
total; the Rust backend is 27.6 MB of it.

The two rows are the same process twenty seconds and twelve minutes after launch,
and the gap between them is the finding:

- **Total working set barely moves** (378.6 → 361.7 MB) because it is dominated by
  shared Chromium image pages that never go away.
- **Private working set falls 26%** (103.3 → 77.1 MB) as Windows trims the pages
  this app actually owns — and at idle it lands **under** §37's 80 MB target,
  before a single optimization.

So §37's target is achievable or unachievable depending entirely on which figure it
means, and the brief does not say. Against Task Manager total it is off by 4.5×
and no application-level work can close that, because there is no application yet
to optimize — the levers are all WebView2-level (fewer renderer processes,
single-process flags, disabling GPU acceleration), and each trades stability or
rendering quality for the number. Against app-attributable memory it is already
met at idle.

The plan's pre-decided response (risk B2) applies: report the measured figures as
a scope question, never silently redefine the metric. That decision is the user's.
Recorded here, escalated in the plan, and re-measured at P05 and P20 so the trend
is visible rather than argued about.

## Build time

Cold builds dominate this project's feedback loop; a first `cargo build` that
compiles the whole Tauri dependency tree taking many minutes is expected, not a
fault.

| Date | Commit | What | Wall time |
|---|---|---|---|
| 2026-09-03 | (pre-commit) | `cargo build --lib` cold, 435 crates resolved, debug profile | 2m 40s |
| 2026-09-03 | (pre-commit) | `bun run verify` from a cleaned dev profile: `check` + `test` + `rs:fmt` + `rs:lint` + `rs:test` | 3m 04s |
| 2026-09-13 | (pre-commit, P08) | Optimized non-test `collector_panic_probe`, unwind profile; executable subsequently passed with exit 0 | 9m 12s |

### Disk, which turned out to be the binding constraint

Not a performance figure, but it stopped a verify run twice and would stop anyone
else's, so it is recorded rather than rediscovered. Build artifacts on this machine:

| Profile | Size |
|---|---|
| `src-tauri/target/debug` | 4.7 GB |
| `src-tauri/target/release` | 1.6 GB |

The debug profile is roughly **3× the release profile** for the same code. With the
D: volume at 100% capacity, `cargo clippy` failed with
`failed to build archive … There is not enough space on the disk. (os error 112)`,
which reads like a linker fault and is not one. `cargo clean --profile dev` recovered
4.4 GB and the same command then passed. Worth knowing before P20's three seeded
corpora (10k / 100k / 1M events) and P21's two-build upgrade rehearsal, each of which
wants headroom of its own.

It happened three more times during P03, so it is not a one-off: holding a debug
and a release profile at the same time is roughly 7 GB, and this volume does not
have 7 GB to spare while also building. The failures look like linker faults
(`link.exe` exit 1180, `failed to build archive at …`) and are not. The working
pattern is to keep one profile at a time — `cargo clean --release` before a debug
test run, `cargo clean --profile dev` before a release measurement — and to check
free space before trusting a timing figure, because the same build that fits on
disk still measures 30% slow while the volume is full.

## Frontend bundle

The webview payload is one JS chunk and one stylesheet. Recorded per phase so a
dependency that doubles it is noticed the week it lands, not at release.

| Date | Phase | JS (raw / gzip) | CSS (raw / gzip) | Notes |
|---|---|---|---|---|
| 2026-09-03 | P02 | 206.6 kB / 63.5 kB | 32.6 kB / 7.2 kB | `bits-ui` (8 wrapped primitives) + Svelte runtime + 7 placeholder screens. Dev states harness and its fixtures confirmed absent from the release chunk (a grep for a fixture string finds nothing), so the large-data harness costs the user nothing. No font shipped (D-16). |

### Timeline virtualization evidence

Phase 07 establishes structural bounds rather than an FPS claim. The production Timeline uses
`@tanstack/svelte-virtual` with fixed 72px event rows and eight-row overscan. Its store retains at most three
pages of 200 raw event payloads; a fixture whose reported total is 128,431 still holds at most 600 and mounts
only the virtual window. Store/component tests also cover eviction and bidirectional refetch.

The frontend gate passed 156 tests, and the 900×600 browser render showed no horizontal overflow. Native
Tauri/WebView behavior for live backend events and focus traps still needs an environment smoke. Measured
scroll FPS against the 100k corpus remains Phase 20 work, so no frame-rate number is recorded here.

## Redaction

Every event and every log line passes through the scrubber, so its cost is paid
on the hot path of both sinks and is a budget rather than a curiosity. Target
from the plan: **under 20 µs per KiB**.

```
bun run rs:bench
```

`src-tauri/examples/redaction_bench.rs` runs `scrub_line` over four payloads of
about a kilobyte each, 2000 rounds apiece, and scales every figure to one KiB. It
is an example rather than a test. **Historical context, 2026-09-04:** the release profile then used
`panic = "abort"`, while test harnesses used unwind. Mixed build attempts in that session produced
link/type failures including *the trait bound `serde_json::Value: Scrub` is not satisfied*; the ordinary
example gave a reproducible product-profile measurement without relying on the test harness.

**Superseded on 2026-09-13:** release now uses `panic = "unwind"` so collector panic boundaries work
in the shipped profile ([D-21](DECISIONS.md#d-21--release-uses-unwind-to-preserve-collector-failure-isolation)).
The current optimized panic example and release query test both passed. The older build incident
above is not a claim that current release tests fail. The redaction example remains the named
benchmark; its 2026-09-04 numbers have not been remeasured as an abort-versus-unwind comparison.

### What a kilobyte costs

Two runs of the same binary, minutes apart — the range is machine state, not code.
The second was taken with the D: volume down to its last gigabyte, which is the
lower bound on how much a busy machine inflates these figures; see the cautions
below.

| Date | Commit | Payload | Per KiB |
|---|---|---|---|
| 2026-09-04 | uncommitted | config — paths with spaces, a port, a build number, one `DB_PASSWORD` | 10.8 – 13.8 µs |
| 2026-09-04 | uncommitted | prose — same length, not one `=` or `:`: the shape pass alone | 4.0 – 5.0 µs |
| 2026-09-04 | uncommitted | the pair scanner's share, by subtraction | 6.7 – 8.8 µs |
| 2026-09-04 | uncommitted | config with a credential in it — every rule runs | 14.3 – 18.0 µs |

The last row is the interesting one. Once any rule fires, all nine run, because
skipping the ones the pre-filter did not name would let one replacement join two
halves of a shape that nothing looks at again. Finding a secret therefore costs
about a third more than not finding one, and both stay inside the budget — the
worst case with the least room, which is the row to watch when a tenth rule is
added.

### How it got there

Three pre-filters, all the same idea: answer "could anything match?" once and
cheaply, so the expensive path is only taken when it can pay off.

- **A `RegexSet` in front of the shape rules.** One automaton pass replaces nine,
  and a line with no credential — the overwhelming majority — costs a single scan
  and no allocation instead of nine scans and nine `Cow`s.
- **A stem filter in front of the key rule.** The pair scanner asks
  `is_sensitive_key` once per `:` or `=` on the line, which on a Windows path or a
  timestamp is often. The full rule lowercases the key into a `String` and splits
  it into a `Vec<String>` of words; the filter first lowercases up to 96 bytes on
  the stack and looks for any of ten substrings that every sensitive name
  contains, removing roughly 96 heap allocations per KiB of config.
- **The entropy rule taken back out of that `RegexSet`.** `[A-Za-z0-9+/]{40,}` has
  no literal in it, and one such pattern in a set costs the whole set its
  prefilter: instead of skipping ahead to the next `sk-` or `://` with a
  memory-scan instruction, the union automaton has to visit every byte. Replacing
  it with a byte loop that looks for a 40-character dense run moved the worst case
  from 19.4 – 22.0 µs to 14.3 – 18.0, and the common case from 13.4 – 14.6 to
  10.8 – 13.8. Two runs on each side of the change, same machine, same session.
  The bands overlap, so read it as a shift rather than a speedup with a decimal
  point — the reason to keep it is that the pathology was real and the worst case
  no longer crosses the budget in any run.

Each filter is sound in one direction only: it may answer "not sure" and cost a
full pass, and it may never answer "definitely not" about a line a rule would have
caught. Unit tests feed each one every term the real rules match on. A fast path in
front of a security control is a leak the moment it is wrong the other way.

### Two cautions for whoever measures this next

**Don't measure on a busy machine.** One run of this example, taken immediately
after a three-minute LTO build with the D: volume nearly full, reported 42 µs per
KiB — three to four times what the same binary reported minutes later on an idle
machine. Nothing about the code changed. The same effect at lower amplitude is
what the ranges in the table above are: numbers taken while something else is
compiling, or while the disk is thrashing, are not numbers. Check free space
first; the section below on disk is not a footnote.

**Don't compare across build paths.** Earlier figures for this hot path were taken
from a timing assertion inside the leak suite, which is a different profile and a
different linkage: 1557 µs per payload in debug with no pre-filter, 231 µs with
the first one, and 30.6 → 16.8 µs in `cargo test --release` across the second.
They are kept here as the direction of travel, not as rows to subtract from the
table above. The suite still carries that assertion with a deliberately loose
ceiling (2000 µs per KiB) so a rule that turns quadratic fails the build in any
profile; the budget itself is checked by the example.

The debug figure is ~15× the release figure because the regex engine is
unoptimised there. The in-suite ceiling is therefore deliberately loose (2000 µs
per KB) so it fails on a rule that turns quadratic rather than on an unoptimised
build; the 20 µs budget is checked against the release row above.

## Query and analysis budgets

Populated from Phase 06 (event engine) onward, against the synthetic 10k / 100k
/ 1M event suites. Recorded per query shape, with the row count returned, so a
regression is visible as a change in either time or unbounded growth.

Phase 06's release gate uses 11 warmed samples and nearest-rank p95. Its budgets
are page ≤ 10 ms, dashboard summary ≤ 15 ms and search ≤ 40 ms. All three rows
below pass. At that snapshot, 100k and 1M enforcement was still pending; later
Phase 20 results appear below.

| Date | Commit | Suite size | Query | Rows | Time |
|---|---|---|---|---|---|
| 2026-09-04 | fadaccf | 10k | keyset page, cursor 5,000 rows deep, `limit 200` | 200 | 1.08–4.14 ms across three runs |
| 2026-09-11 | c794948 | 10k | keyset page, cursor 5,000 rows deep, `limit 200` | 200 | p50 1.649 ms / p95 4.545 ms |
| 2026-09-11 | c794948 | 10k | dashboard counts + top important | 1 summary | p50 1.960 ms / p95 5.281 ms |
| 2026-09-11 | c794948 | 10k | FTS search for `Synthetic Service`, `limit 100` | 100 | p50 12.332 ms / p95 18.348 ms |
| 2026-09-13 | (pre-commit, P08) | 10k | keyset page, cursor 5,000 rows deep | limit 200 | p50 1.154 ms / p95 1.473 ms |
| 2026-09-13 | (pre-commit, P08) | 10k | dashboard counts + top important | 1 summary | p50 1.044 ms / p95 1.764 ms |
| 2026-09-13 | (pre-commit, P08) | 10k | FTS search for `Synthetic Service` | limit 100 | p50 7.184 ms / p95 8.094 ms |

The Phase 08 release rerun passed **1/1** with the same **11 warmed samples** and nearest-rank p95;
all three were below their existing budgets. This was a dated source-snapshot measurement, not an isolated
claim that the runtime made query processing faster. Reproduce with the production sanitizer/writer
and optimized query profile:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --release --features devtools -j 2 --test perf_scale -- --nocapture
```

### The write path, first measured with the repository

One test does the whole measurement: 10,000 synthetic events through the real
writer thread in batches of 200, then a keyset page 5,000 rows deep, then the
file sizes. It prints its three lines and asserts only a loose ceiling.

```
cargo test --manifest-path src-tauri/Cargo.toml --all-features --test repository -- --nocapture ten_thousand
```

| Date | Commit | Machine state | Events/s | Per batch of 200 | Page 5,000 deep | Database | WAL | Bytes/event |
|---|---|---|---|---|---|---|---|---|
| 2026-09-04 | fadaccf | idle | 5,393 | 37.1 ms | 1.61 ms | 5,509,120 B | 4,791,592 B | 1,030 |
| 2026-09-04 | fadaccf | a second cargo build running | 1,964 | 101.8 ms | 4.14 ms | 5,509,120 B | 4,791,592 B | 1,030 |
| 2026-09-06 | (pre-commit) | idle | 4,107 | 48.7 ms | 1.08 ms | 5,509,120 B | 4,791,592 B | 1,030 |
| 2026-09-06 | (pre-commit) | idle | 5,737 | 34.9 ms | 1.37 ms | 5,509,120 B | 4,791,592 B | 1,030 |

Four runs of the same test on the same write path. They are kept as a set rather
than averaged, because the 2.9× spread between the fastest and the slowest is the
caution above made concrete: contention, not code, is the difference, and the byte
figures — which do not depend on the machine — are identical to the byte in all
four. The suite's own ceiling is 60 s for the 10,000 inserts, loose on purpose so
it fails on a write path that has turned quadratic rather than on a busy laptop.
Later 100k / 1M results are recorded in the Phase 20 section below.

Storage is about **1 KB per event**, roughly half of it in the write-ahead log
until a checkpoint folds it back. A year of a hundred events a day is therefore
tens of megabytes, which is what makes the 30-day default retention a comfort
setting rather than a necessity.

## Idle cost

The app is resident all day, so idle CPU matters more than throughput. Target from the brief:
approximately 0-1% whole-app idle. Phase 08 measures the runtime/Db contribution separately below;
whole-app sampling with real platform collectors remains Phase 20 work.

| Date | Commit | Sample window | Idle CPU | Notes |
|---|---|---|---|---|
| 2026-09-13 | (pre-commit, P08) | 62.324830 s | 0.000000% of one core; also 0.000000% normalized over 12 logical CPUs | Debug runtime/Db only; two fake 1 s collectors; Windows CPU counter delta 0 s at its resolution; exit 0 |

### Phase 08 runtime overhead — 2026-09-13

Retained example: [`collector_idle_probe.rs`](../src-tauri/examples/collector_idle_probe.rs), explicitly
gated by the `devtools` feature in the Cargo manifest. It uses the real Db and `CollectorSupervisor`
with two empty fake collectors at 1 s cadence. **This measurement used the debug profile**, not a
release desktop application; there was no WebView2 process tree and no platform enumeration.

| Measurement | Result |
|---|---|
| External sample duration | 62.324830 s |
| Process CPU time delta | 0 s at Windows counter resolution |
| CPU of one logical core | 0.000000%; below the Phase 08 <0.3% gate |
| CPU normalized across machine | 0.000000% / 12 logical CPUs |
| Peak process working set | 8,736,768 B (8.33 MiB) |
| Collector counters | 2/2 at readiness; 69/69 at exit, after sample plus exit margin |
| Exit | 0 |

Zero CPU is the measured counter resolution, not a claim of literally zero work. The working-set
figure is a runtime probe metric, **not** a substitute for the total/private app-memory pair required
by D-15. Those Phase 05 checks were subsequently completed on 2026-09-19: the release build produced
the NSIS bundle and the full-app memory row above records both required figures.

Reproduce from the repo root; retain the shared task-specific target directory on this disk-limited
machine and keep Cargo at two jobs. The directory name is a build-cache location, not a build profile:

```powershell
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) '.qa\phase08-release-target-20260912'
$env:CARGO_BUILD_JOBS = '2'
cargo build --manifest-path src-tauri/Cargo.toml --features devtools --example collector_idle_probe -j 2
& (Join-Path $env:CARGO_TARGET_DIR 'debug\examples\collector_idle_probe.exe')
```

After `IDLE_PROBE_READY pid=...` appears, sample that PID externally for at least 60 s. Record
`Get-Process -Id <pid>` / `TotalProcessorTime.TotalSeconds` at both boundaries, refreshing the process
object, and measure elapsed wall time with `System.Diagnostics.Stopwatch`. Compute one-core CPU as
`100 * cpu_seconds_delta / elapsed_seconds`; divide by logical CPU count for the whole-machine
percentage. Read `PeakWorkingSet64` before exit and retain `IDLE_PROBE_COUNTS` plus exit status.
The example remains alive 68 s after readiness to leave sampling margin. Running the binary alone
proves its counters, not CPU usage; the external sample is required for the <0.3% gate.

### Separate optimized panic-isolation artifact

[`collector_panic_probe.rs`](../src-tauri/examples/collector_panic_probe.rs) is also devtools-gated.
The optimized non-test executable passed with exit 0: a deliberate collector panic was contained and
the healthy neighbour continued. Build time: 9 min 12 s. Reproduce independently of the debug idle run:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --release --features devtools --example collector_panic_probe -j 2
```

No binary-size or steady-state overhead comparison between abort and unwind was measured. Full-app
idle and real collector cost remain Phase 20 evidence gaps; later 100k/1M
query results are recorded below.

## Phase 11 Windows service/application provider gates — 2026-09-19

The ignored real-OS test runs twenty steady sweeps after one explicit cold/full-configuration service
refresh. This separation is intentional: the five-second path must stay cheap, while config-only start
type changes are reconciled by an owned synchronous 15-minute sweep rather than an unmeasured detached
worker.

| Measurement | Result | Budget |
|---|---:|---:|
| Win32 service steady p95, 20 sweeps | 6.519 ms | ≤12 ms |
| Uninstall registry p95, 20 sweeps | 31.9952 ms | ≤150 ms |
| Full service configuration refresh, 301 services | 173.0581 ms | ≤500 ms |

The provider enumerates `SERVICE_WIN32`; the 443 driver rows on this workstation belong to Phase 12.
Combining those with 301 services made 744 rows and violated the fast-cycle boundary. Reproduce:

```powershell
cd src-tauri
cargo test --test collector_windows_performance -- --ignored --nocapture
```

## Phase 12 Windows Update and driver provider gates — 2026-09-20

The WUA gate performs twenty warm history polls on the collector-owned STA thread. The SetupAPI gate
performs five real sweeps, while a separate test queries a seeded 1,000-event timeline during a real
driver enumeration. The forced-stall test validates the hard deadline and recovery path rather than
including that deliberate ten-second delay in normal sweep latency.

| Measurement | Result | Budget |
|---|---:|---:|
| WUA history p95, 20 warm polls, 18 rows | 2.7221 ms | ≤200 ms |
| SetupAPI p95, 5 sweeps, 217 devices | 376.8582 ms | ≤3500 ms |
| Timeline query p95 during SetupAPI sweep | 1.2294 ms | ≤20 ms |
| Forced driver stall abandonment | 10.0045373 s; recovered, no overlap | hard 10 s target |

Reproduce the real-OS gates explicitly because they remain ignored in the ordinary suite:

```powershell
cd src-tauri
cargo test --test collector_wua_performance -- --ignored --nocapture
cargo test platform::windows::devices::tests::five_real_sweeps_stay_inside_the_phase_budget -- --ignored --nocapture
cargo test --test collector_driver_query_latency -- --ignored --nocapture
cargo test --test collector_drivers -- --ignored --nocapture
```

## Phase 13 network, startup, and Event Log gates — 2026-09-20

The network and Task Scheduler providers were exercised on Windows while 100 timeline queries were
sampled only during overlapping sweeps. A separate forced-stall gate proves the startup coordinator keeps
ownership across the 15-second abandonment boundary. Event Log subscription lifetime was opened and
closed 200 times after the subscribe-first/current-boot query sequence.

| Measurement | Result | Budget |
|---|---:|---:|
| Network sweep p95, 46 adapters | 15.2741 ms | ≤2500 ms |
| Task-provider safe unavailable path p95, 0 rows | 5.0435 ms | ≤2500 ms |
| Timeline query p95 during network/task sweeps | 3.0725 ms | ≤20 ms |
| Forced startup stall | 15.58 s; recovered, no overlap | hard 15 s abandonment target |
| Event Log subscription lifecycle | 200 open/close cycles, pass | no lifetime fault |

This host reports Task Scheduler root `0x80070003` through both COM and `schtasks`, so the real task row
latency is explicitly not claimed here. The provider reports the store unavailable, preserves the prior
task baseline, and continues Run-key/Startup-folder collection. Callback code is bounded to XML render,
`try_send`, counters, and wake; no synthetic record was written into the machine Event Log solely to
manufacture a callback timing sample.

```powershell
cd src-tauri
cargo test --test collector_network real_network_sweep_stays_inside_the_phase_budget -- --ignored --nocapture
cargo test --test collector_startup real_task_scheduler_sweep_stays_inside_phase_budget -- --ignored --nocapture
cargo test --test collector_startup hard_timeout_abandons_a_stall_without_overlapping_the_query -- --ignored --nocapture
cargo test --test phase13_query_latency -- --ignored --nocapture
cargo test --test collector_eventlog real_subscriptions_open_and_close_two_hundred_times_without_callback_lifetime_leaks -- --ignored --nocapture
```

## Phase 14 checkpoint gates — 2026-09-20

The final repository gate passed 670 Rust tests with 14 ignored. The frontend gate passed 211 tests, and
typecheck, strict Clippy, formatting and generated-binding parity also passed. These are test-run totals,
not a claim that ignored real-machine cases ran inside the ordinary suite.

The scripted 3,000-item comparison gate passed its asserted nearest-rank p95 budget of at most 150 ms in
the clean full run, eight sequential repeats and two ordinary-contention repeats. The passing output did
not print a narrower p95, so none is recorded. Performance gates must not be co-scheduled with another
copy of the same benchmark: that synthetic double load can exceed the workstation budget and is outside
this point-in-time contract. Its creation companion verifies a single atomic commit,
the 3,000-item count and the 2 MiB database-growth ceiling while concurrent timeline reads remain within
their asserted 20 ms budget.

The separately invoked Windows real-capture gate completed in 711 ms with 1,246 non-partial items on this
workstation. That figure is one observed capture, not a stable collector inventory or a cross-machine
benchmark; installed software, devices and enabled sources can change both values.

Executable owners: [`checkpoint_create.rs`](../src-tauri/tests/checkpoint_create.rs),
[`checkpoint_compare.rs`](../src-tauri/tests/checkpoint_compare.rs),
[`checkpoint_auto.rs`](../src-tauri/tests/checkpoint_auto.rs),
[`checkpoint_leak.rs`](../src-tauri/tests/checkpoint_leak.rs), and
[`checkpoint_real.rs`](../src-tauri/tests/checkpoint_real.rs).

## Phase 15 investigation gates — 2026-09-20

Both explicit Phase 15 p95 gates passed on this workstation:

| Workload | Enforced budget | Result |
|---|---:|---|
| Pure deterministic engine over 5,000 events | ≤400 ms p95 | passed |
| Persisted investigation over a graph-rich 100,000-row database; 5,000 analysed after the cap | ≤1.5 s p95 | passed |

Each gate takes twenty samples and asserts the nearest-rank p95 in
[`analysis_perf.rs`](../src-tauri/tests/analysis_perf.rs). The passing output retained no exact p95, so this
record deliberately preserves only the budgets and pass result. These checks establish the Phase 15
investigation bound; they do not replace Phase 20's broader 100k/1M UI, memory and frame-time work.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test analysis_perf -- --include-ignored
```

## Change Map rendering — 2026-09-20

The dedicated `#/dev/perf` browser fixture supplied 3,000 nodes to the Change Map, forced the low quality
tier, and retained 300 end-to-end `requestAnimationFrame` intervals. Headful Chrome reported
**16.70 ms p50 / 16.80 ms p95** on `ANGLE (Intel Iris Xe Graphics, Direct3D11)`, the monitor's quantized
single-frame cadence (about 59.9 FPS). The accessible node navigator kept **41 DOM options**, and the page
reported no errors. This replaces the earlier synchronous `renderer.render()` CPU-submit measurement,
which did not include GPU completion or presentation cadence.

This closes the phase's measurement on this workstation; it does not predict WebView2 performance on a
different graphics driver or machine. The executable fixture is
[`ChangeMapPerfHarness.svelte`](../src/features/dev/ChangeMapPerfHarness.svelte); frame sampling
and quality selection are owned by [`src/three/changeMap/`](../src/three/changeMap/), and the accessible
window is owned by [`MapNodeList.svelte`](../src/components/visualization/MapNodeList.svelte).

## System Map query and rendering — 2026-09-20

The ordinary dense-database performance fixture inserts 50,000 relationships and asks for depth three.
Its 20-query gate verifies the fresh-reader sample and every sample stay within 120 ms while returning
**50,001** as the exact connected-node count. Final review measured **74.254 ms first/max, 67.745 ms
median, and 73.951 ms p95**. Its query-plan assertions require both relationship indexes;
the executable authority is [`perf_system_map.rs`](../src-tauri/tests/perf_system_map.rs), with index
ownership in the migrations and traversal ownership in
[`system_map_repository.rs`](../src-tauri/src/storage/system_map_repository.rs).

The dedicated `#/dev/system-map-perf` fixture rendered the capped 400-node graph in headful Chrome on
`ANGLE (Intel Iris Xe Graphics, Direct3D11)` at **16.70 ms p50 / 16.90 ms p95**, within the 17.5 ms
frame gate. The page exposed **41 DOM options**, kept the dependency panel beside the canvas at the
measured viewport, and reported no errors. This is workstation evidence, not a promise for other GPUs,
drivers or WebView2 versions. The fixture owner is
[`SystemMapPerfHarness.svelte`](../src/features/dev/SystemMapPerfHarness.svelte); scene lifecycle coverage
is owned by [`scene.lifecycle.test.ts`](../src/three/systemMap/scene.lifecycle.test.ts).

Phase-specific validation passed 54 frontend files / 304 tests, frontend check and production build,
focused Rust command tests (9), architecture tests (10), payload tests (4), the dense performance test
(1), generated bindings, rustfmt and strict Clippy. A repository-wide Rust run separately failed the
Phase 09 real-machine process timing assertion. That host-sensitive result is outside the System Map
boundary and remains recorded here so the broader run is not misreported as green.

## Phase 20 query gates — 2026-09-26 source snapshot (pre-v14)

That day's warm 30-sample `perf_queries` gate passed the three measured stable read paths
at 10k, 100k and 1M:

| Corpus | Page p95 | Summary p95 | Search p95 | Budgets (page / summary / search) |
|---:|---:|---:|---:|---:|
| 10k | 2.583 ms | 0.655 ms | 2.280 ms | 10 / 15 / 40 ms |
| 100k | 7.513 ms | 10.202 ms | 26.749 ms | 20 / 30 / 90 ms |
| 1M | 10.069 ms | 12.127 ms | 219.873 ms | 45 / 80 / 250 ms |

Migrations 009–013 in that snapshot add a narrow ordering projection, importance-token FTS
buckets and exact daily importance aggregates. Query-plan, migration lifecycle and
repository search-order tests pass. The
1M fixture used D-backed `TEMP`/`TMP` and took 905 s to seed on this workstation; database-size,
resource/soak, detailed-query and manual accessibility gates are not represented by this table.
Migration 014 and later repository/capture changes postdate these measurements.
No current-source 1M query rerun or 1M v13→v14 upgrade timing/free-space result is claimed.

## Phase 20 pre-density 10k diagnostic — 2026-09-29

The devtools fixture streamed a 90-day corpus through the sanitized writer:
10,000 events, 3,000 graph entities, 15% events with related IDs, 8% burst-labelled rows,
40 checkpoints and 20 investigations. This first version persisted only 150 distinct
evidence edges (1.5% of events); the corrected generator now asserts 1,500 (15%).
The following seven-of-eight query result is historical until rerun on the corrected corpus:

| Query | p95 | 10k budget |
|---|---:|---:|
| First page | 1.849 ms | 10 ms |
| Keyset page 10 | 1.869 ms | 10 ms |
| Dashboard summary | 0.251 ms | 15 ms |
| Two-token search | 1.501 ms | 40 ms |
| Details and recurrence | 0.279 ms | 10 ms |
| Persisted investigation (5k window) | **644.889 ms** | **400 ms — fail** |
| Compare two 3k-item checkpoints | 65.463 ms | 150 ms |
| Privacy summary | 1.550 ms | 30 ms |

This near-canonical 10k database measured **11,411,456 bytes** against the 12,000,000-byte
limit. Tagged purge followed by explicit full vacuum reduces pages from 2,786 to 308 and
freelist to zero; 3,000 entity rows remain because ownership cannot safely be proved.
This is diagnostic evidence, not a release pass: the graph-density correction invalidates
the measured fixture, investigation exceeded budget, and 100k/1M canonical results are missing.
The CLI seeder is now restricted to fresh, reserved QA directories;
the shipped app refuses those roots. Purge still conservatively retains synthetic entity rows.

## Phase 20 final-corpus release 10k gate — 2026-09-29

The corrected 90-day corpus has 10,000 events, 3,000 entities, 1,500 distinct persisted
event-backed edges, 800 rows that form real Timeline clusters, 40 checkpoints and 20
investigations. With 30 warm release-profile samples, all eight 10k query budgets pass:

| Query | p95 | 10k budget |
|---|---:|---:|
| First page | 0.559 ms | 10 ms |
| Keyset page 10 | 0.671 ms | 10 ms |
| Dashboard summary | 0.384 ms | 15 ms |
| Two-token search | 0.727 ms | 40 ms |
| Details and recurrence | 0.064 ms | 10 ms |
| Persisted investigation (5k window) | 144.212 ms | 400 ms |
| Compare two 3k-item checkpoints | 15.159 ms | 150 ms |
| Privacy summary | 1.006 ms | 30 ms |

Database size is **11,599,872 / 12,000,000 bytes**. Tagged purge removes 10,000 events;
explicit full vacuum changes `page_count` **2,832 → 309** and ends with `freelist_count=0`.
The QA-owned entity rows remain because later ownership cannot be proved. This is a 10k
gate result, not the full Phase 20 pass: 100k/1M, routine retention reclaim, canonical
query-plan parity, soak/resource/frame and installed accessibility evidence remain open.
The synthetic before/after state and metadata are shorter than many real collector payloads;
a representative collector-payload spot-check is still needed before extrapolating the
bytes-per-event result to production histories.

## Phase 20 final-corpus release 100k gate — 2026-09-29

Each 100k fixture seeded in about 23–25 seconds. Database size passes at
**82,464,768 / 110,000,000 bytes**; after tagged purge and explicit full vacuum,
`page_count` falls **20,133 → 4,587** and `freelist_count=0`.

| Query | p95 | 100k budget |
|---|---:|---:|
| First page | 0.636 ms | 20 ms |
| Keyset page 10 | 0.597 ms | 20 ms |
| Dashboard summary | 0.382 ms | 30 ms |
| Two-token search | 4.481 ms | 90 ms |
| Details and recurrence | 0.113 ms | 15 ms |
| Persisted investigation (5k window) | **896.470 ms** | **600 ms — fail** |
| Compare two 3k-item checkpoints | 23.862 ms | 200 ms |
| Privacy summary | 9.772 ms | 60 ms |

The 100k query gate is **7/8**, not a pass. The matrix stopped before 1M;
no budget was relaxed. A subsequent batched indexed recurrence change passed
23/23 focused repository/investigation tests and its covering-index plan gate.
On the same final corpus, the 100k release-profile investigation p95 fell to
**784.607 / 600 ms** (still fail); the other seven paths passed. Stage profiling
measured the recurrence batch at **76.617 ms p95**, so further investigation
targets the remaining load/analysis/persistence cost. The full 1M matrix has
not run after this change.

## Phase 20 routine retention and incremental reclaim — 2026-09-29

The opt-in current-source 10k gate calls production `maintenance::run_pass`,
not the QA tagged purge or full `VACUUM`. With a fixed retention clock it found
and deleted **1,575** eligible events, retained important events within their
longer lifetime, and reduced `page_count` **2,832 → 2,691** with
`freelist_count` **0 → 0** in **148 ms**. A focused regression with more than
512 free pages verifies that one pass reclaims at most 256 pages. This closes
the 10k routine-reclaim behavior only; 100k/1M retention and installed-app
resource measurements remain open.

## Phase 20 100k low-load rerun — 2026-09-29

An unchanged-source final-corpus release rerun passed **8/8**. The persisted
investigation p95 was **317.194 / 600 ms** over 30 warm samples; the other
seven p95 values (in table order above) were 0.899, 0.794, 0.391, 3.851,
0.080, 16.467 and 6.553 ms. A preceding same-code/corpus diagnostic under
higher host load measured the investigation at **786.441 ms**, after the
784.607 ms failed gate above. The profiler's independently sampled stage p95s
are not additive. This green run establishes one low-load 100k result, not a
stable release-candidate pass or a 1M result.

## Phase 20 final-corpus release 1M query gate — 2026-09-29

The canonical corpus contained **1,000,000** events, **3,000** entities,
**150,000** evidence-backed edges and **80,000** burst events. Seeding took
**316.041 s**, above the plan's four-minute operational target. The 30-warm-
sample release gate passed six of eight paths:

| Query | p95 | 1M budget |
|---|---:|---:|
| First page | 0.544 ms | 45 ms |
| Keyset page 10 | 0.898 ms | 45 ms |
| Dashboard summary | 2.333 ms | 80 ms |
| Two-token search | 41.953 ms | 250 ms |
| Details and recurrence | 0.436 ms | 30 ms |
| Persisted investigation (5k window) | **11,187.263 ms** | **1,500 ms — fail** |
| Compare two 3k-item checkpoints | 38.111 ms | 300 ms |
| Privacy summary | **262.719 ms** | **200 ms — fail** |

The 1M query gate is **6/8**, not a pass. These misses are being diagnosed
without changing the budgets. The separate corrected-corpus 1M database-size
gate passed **before** category-count migration 015: seeding took 362.920 s;
database size was **795,377,664 / 1,100,000,000 bytes**, WAL was 88,921,992
bytes, and tagged purge plus explicit full `VACUUM` reduced pages
**194,184 → 43,822** with freelist ending at zero. This full compaction is
distinct from the routine bounded retention gate. The v15 size/seed result
remains to be measured.

The v15 diagnostic fixture then seeded 1M in **325.504 s**. On five mirrored
samples the investigation load stage measured **1,961.442 ms p95** and pure
analysis **7,840.230 ms p95**. Inside analysis, `FailureIndex::build` alone
measured **8,509.949 ms p95** across separate samples: it traversed two hops
from roughly 2,500 candidate entities through a dense 150k-edge graph.
Relationship fetch measured **1,099.504 ms p95**; an exact-equality ordered
full-scan diagnostic measured 212.841 ms on the same corpus. Separately,
v15's transactional category-count projection measured **0.539 ms p95** versus
the former `GROUP BY events` read at 106.629 ms, with equal counts. These
substage p95s are independent samples and must not be added. Neither the
investigation optimization nor the full v15 eight-shape gate has passed yet.

## Phase 20 optimized v15 100k gate — 2026-09-29

The first release-profile eight-query run after reverse two-hop failure
propagation and adaptive relationship loading passed **8/8** on the unchanged
canonical 100k event/15k-edge corpus. p95 milliseconds: first page **0.463**,
page 10 **0.563**, summary **0.828**, two-token search **3.340**,
details/recurrence **0.089**, persisted investigation **213.908/600**,
checkpoint compare **17.322**, privacy summary **0.627/60**. The seed took
**19.153 s**. Investigation stage p95s were read **145.947** and analysis
**56.429**; FailureIndex alone was **1.052**. No budget was relaxed. A graph-
skew fallback regression and the optimized 1M gate remain open; the latter's
earlier 6/8 result belongs to the pre-optimization build.

## Phase 20 local release-build resource run — 2026-10-07 (onboarding re-mount defect present)

One local `cargo build --release --features tauri/custom-protocol` build of the
working tree (`what-changed.exe`, built 2026-10-07 01:35, not an installer and not
a release candidate) was launched against the developer's own profile with
onboarding unfinished, on a 12-logical-core host. Sampled every 5 s for 604 s
with the window hidden to the tray after a close request. Both memory figures are
reported and neither is a verdict (D-15); the run is **not** an acceptance
measurement because the build contained the defect described below.

| Reading | Total working set | Private working set | WebView2 share (total / private) | Backend only (total / private) |
|---|---:|---:|---:|---:|
| Window shown, 20 s after launch | 628.6 MB | 356.1 MB | 580.8 / 344.2 MB | 47.8 / 11.9 MB |
| Window hidden, 20 s after close | 668.1 MB | 376.5 MB | 620.1 / 364.5 MB | 48.0 / 12.0 MB |
| After 10 min hidden | 917.2 MB | 601.0 MB | 868.5 / 588.5 MB | 48.8 / 12.6 MB |
| Peak sampled during the 10 min | 1,036.8 MB | 731.1 MB | n/a | n/a |

Start to first visible top-level window: cold **1,260 ms**, warm **283 ms**. This
is window creation, not first paint of content, and tray-only timing was not
measured (no external way to observe the tray icon).

Idle CPU over the process tree was **328.2 % of one core** on average (27.4 % of
the machine), p95 interval 347.1 %, max 352.5 %, far above the 1 % target. A
per-process split with collectors off in safe mode still showed backend ~52 %,
renderer ~122 %, WebView2 browser ~98 % and network service ~29 % of a core, so
collectors were not the cause.

Cause, measured by counting `fetch` calls to `ipc.localhost` through a CDP
session on the launched process: about 63 round-trips per second of one fixed
command set (`get_settings`, `list_watched_locations`, `get_dashboard_summary`,
`get_app_status` and two `get_collector_status`). `settings.load()` cleared
`current` while reloading, which made `App.svelte` unmount `OnboardingFlow`, whose
mount reloads settings, on any profile with onboarding unfinished. The store now
keeps loaded settings during a reload and a regression test covers it. The fix is
unit-tested only: the idle CPU and B2 rows must be re-measured on a rebuilt exe
before any P20 resource checkbox changes. An earlier reading of the same shape
(319 %) came from a stale 2026-09-26 binary and is discarded.

## Phase 20 local release-build resource run after the settings-reload fix — 2026-10-07

The same local `cargo build --release --features tauri/custom-protocol` procedure was
repeated on a rebuild that keeps loaded settings during a reload (`what-changed.exe`,
built 02:25, not an installer or release candidate; 12 logical cores; window hidden to
the tray after a close request; 602 s sampled every 5 s). The profile still had
onboarding unfinished, and the backend disables every collector in that state, so this
measures the shell, WebView2 and an idle backend **with all collectors off**. It does
not satisfy "idle CPU with all collectors enabled". Both memory figures are reported;
neither is a verdict (D-15).

| Reading | Total working set | Private working set | WebView2 share (total / private) | Backend only (total / private) |
|---|---:|---:|---:|---:|
| Window shown, 20 s after launch | 411.9 MB | 140.4 MB | 375.0 / 129.5 MB | 36.9 / 10.9 MB |
| Window hidden, 20 s after close | 411.5 MB | 130.3 MB | 374.5 / 119.4 MB | 36.9 / 10.9 MB |
| After 10 min hidden | 406.2 MB | 100.5 MB | 369.1 / 90.2 MB | 37.1 / 10.3 MB |
| Peak sampled during the 10 min | 417.6 MB | 130.4 MB | n/a | n/a |

Idle CPU over the process tree: **0.210 % of one core** mean (0.018 % of the machine),
p95 interval 0.874 %, max interval 3.537 %. Before the fix the same procedure measured
328.2 %. Total and private working set no longer drift upward. Start to first visible
top-level window: cold **1,288 ms**, warm **124 ms** (window creation, not first paint;
tray-only timing still unmeasured).

Still open: idle CPU, B2 readings and soak with collectors enabled, which requires a
completed onboarding on the measured profile; an installed-app and 4-hour soak run; and
the idle p95 on a quiet host with the Timeline or Investigations screen open.

## Phase 20 attach-mode run with collectors on — 2026-10-07

A read-only 604 s measurement of a locally built `what-changed.exe` (built 02:25, not an
installer) that the developer had launched and taken through onboarding with **Skip**
(Skip does not register autostart). Collectors were therefore enabled with their
persisted defaults. The window was **visible** throughout, and the run did not hide,
restart or stop the process. 12 logical cores, sampled every 5 s. Both memory figures are
reported; neither is a verdict (D-15).

| Reading | Total working set | Private working set | WebView2 share (total / private) | Backend only (total / private) |
|---|---:|---:|---:|---:|
| Start of run (process age 12 min) | 490.9 MB | 175.5 MB | 439.9 / 154.5 MB | 51.0 / 21.1 MB |
| End of run | 492.4 MB | 178.4 MB | 435.0 / 151.2 MB | 57.3 / 27.2 MB |
| Peak sampled | 539.3 MB | 224.3 MB | n/a | n/a |

Process tree CPU: **9.973 % of one core** mean (**0.831 %** of the machine), p95
interval 15.802 %, max 25.140 %. Threads held at 52 and handles moved 510 → 512. Backend
private working set rose 6.1 MB over the run.

This is not a quiet-host idle figure. The process collector had recorded about 14,500
events in the profile, nearly all `process started/stopped` for short-lived developer and
tooling processes (`bash.exe`, `conhost.exe`, `node.exe`, `git.exe`, `powershell.exe`,
`ak.exe`). The per-minute event rate stepped from roughly 50-600 to 1,200-2,500 from
03:03 while the run was in progress, and the host's other activity was not controlled, so the
CPU and backend-memory rise cannot be attributed to the idle product. The 1 % target is
ambiguous between machine-wide and single-core percentage; both are given. Idle CPU with
collectors enabled, the hidden-window state, three B2 checkpoints and the soak remain
open until a rerun on a quiet host.

## Phase 20 current-source B2 snapshots — 2026-10-14

A current-source non-packaged release binary was built with `cargo build --release --locked
--no-default-features` and measured with `scripts/measure-memory.ps1`. Both readings are
engineering evidence only; D-15 attaches no pass/fail verdict to either memory figure.

| Reading | Total working set | Private working set | WebView2 share (total) | Backend only (total) |
|---|---:|---:|---:|---:|
| Window shown after 5 s settle | 414.9 MB | 132.7 MB | 366.1 MB | 48.8 MB |
| Window hidden after 5 s settle | 410.5 MB | 128.9 MB | 359.8 MB | 50.7 MB |

The after-four-hour checkpoint, idle CPU/soak and installed-app readings remain open.

## Phase 09 process provider path-buffer experiment — 2026-10-14

The process provider's reusable UTF-16 path workspace now starts at 4,096 units
and grows/retries only on `ERROR_INSUFFICIENT_BUFFER`, capped at 32,767 units.
The experiment keeps the existing limited handle, creation-time identity,
access-denied downgrade, path clearing and no-cache/no-retained-handle rules.
Release functional suites passed: `collector_process` 9/9 (one real-machine
test ignored) and `relationships_process_port` 10/10. The release 50-cycle
collector gate measured listener **286.6 µs p95** (≤15 ms) and process
**16.9625 ms p95** (>15 ms). The process result improves on the prior 19.4261
ms current-source result but remains a failed acceptance gate; no budget was
relaxed and no release claim follows.

## Phase 20 larger query-shape diagnostics — 2026-10-14

Following the 10k collector-payload size sensitivity miss, the existing release-profile query
harness was run independently at 100k and 1M with unchanged SQL, fixtures, p95 thresholds and
one Cargo job. These are query diagnostics only; they do not close the DB-size, seed-time or
resource gates.

| Corpus | First page | Page 10 | Summary | Search | Details | Investigation | Checkpoint | Privacy |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 100k (p95 ms) | 0.777 / 20 | 0.669 / 20 | 0.387 / 30 | 4.013 / 90 | 0.105 / 15 | 144.144 / 600 | 20.950 / 200 | 0.670 / 60 |
| 1M (p95 ms) | 1.657 / 45 | 1.203 / 45 | 6.904 / 80 | 118.718 / 250 | 0.563 / 30 | 755.032 / 1,500 | 25.950 / 300 | 0.474 / 200 |

Both diagnostic runs passed all eight query budgets. The 1M seed took 319.636 s, so the
separate seeder-time target and the 1M database-size/vacuum gate remain unclosed. The 10k
collector-shaped size run remains the authoritative failure: 12,042,240 bytes versus the
12,000,000-byte budget. Its paired byte-level inspection found zero free pages and growth in
the event payload/table, not reclaimable slack; no budget was relaxed and no payload was removed.

## Phase 20 headless collector-soak harness — 2026-10-14

The devtools-only headless runner requires an explicit reserved QA directory,
opens the platform-composed `AppState`, starts the real collector supervisor,
and drives the shared sanitized simulation helper through a local trigger/ack
protocol. A direct trigger smoke committed one configuration event (`ok|1`)
with migration 16 and clean shutdown metadata. The first long-run attempt was
discarded because the runner initially omitted `spawn_all()`; it was not used as
acceptance evidence.

After the lifecycle fix, a fresh livecheck started the workers and stopped with
the explicit health failure `enabled collectors are unhealthy: docker` because
Docker is not installed on this machine. The QA WAL grew to **1,178,352 bytes**,
showing real collector/writer activity. The runner verifies the exact production
collector registry, treats `Degraded` as unhealthy, and fails on any non-zero
writer-loss counter (queue drops, failed commit rows or cycles, or cycle-admission
drops). Those checks alone did not establish lifetime collection coverage. It also refuses
to compare against an unresolved normal-app-data path, avoiding a false exclusion
when that path does not yet exist.
No 4-hour result or soak pass is claimed; the full gate remains open pending an
all-capabilities host.

## Collector repeat evidence — 2026-10-26

A fresh release test build measured process **92.4285 ms p95** and listener
**1.9762 ms p95**. Three predeclared runs of that same executable then measured
process **17.4784 / 57.5130 / 86.1510 ms** and listener **0.3426 / 2.1934 /
4.5333 ms**. Every process run misses the unchanged 15 ms target; all listener
runs pass. The before/after host CPU samples were 23→15%, 79→98% and 100→100%.
They show varying host load, but do not isolate the cause of the misses. P09
remains open. Artifact hash, timestamps, process counts and next diagnostic:
[continuation evidence](../plans/reports/qa-261026-continuation.md).

The subsequent ignored paired diagnostic preserves the provider's reusable Vec
and records numeric-only full/provider/remainder/acknowledgement-region/destruction
timings. Its one 50-cycle run was entirely Noop: total p95 **12.6546 ms**, provider
**12.6185 ms**, remaining snapshot **0.0406 ms**. Instrumentation overhead remains
included. Provider work dominates that quiet sample; staged cycles and the cause
of historical misses are not established.

Exactly one original acceptance invocation then passed on the same new test
artifact: process **13.7265 ms**, listener **0.3222 ms**, **2 passed / 1 diagnostic
ignored**, exit 0. CPU snapshots were 42→34%, process counts 343→342. The new paired
evidence justified this bounded check; there was no retry or production change.
P09's existing repeatability requirement still keeps its checkbox open. All
earlier failed observations remain recorded in the continuation report.

## Backend soak verification — 2026-10-26

The devtools runner now exclusively creates a fresh QA root, rejects normal-profile
and reparse aliases, and requires distinct direct-child trigger/ack files. Readiness
requires a successful snapshot from every enabled production collector. Observer
outcomes retain failures after recovery; writer-loss counters and incomplete
thread joins invalidate the final result. The resource sampler requires matching
readiness/result identities and a zero exit code. Its output explicitly excludes
native/WebView acceptance. See [runner and sampler](../scripts/soak.ps1) for the
current protocol rather than reusing an old QA directory.

Verification: 8 soak unit tests, 5 runtime observer integration tests, the shutdown
join test and the PowerShell protocol negatives pass. The wider Rust library run
passed 571 tests with 3 intentional ignores; final all-target/all-feature strict
Clippy and the devtools debug build pass. Independent review found and verified
repairs for equivalent protocol paths and an invalid atomic test fixture.

A bounded live negative check on this host correctly rejected unavailable Docker:
backend exit 1, `status=fail`, `ready=false`, full Docker denominator retained,
zero writer loss and `shutdown_complete=false`. This is rejection evidence, not
a successful soak. No four-hour or installed-app gate closes. Details:
[hardening report](../plans/reports/soak-hardening-261026.md),
[review](../plans/reports/soak-review-261026.md).
