# Actual-bound canonical checkpoint-window smoke — 2026-10-05

Status: exactly one approved canonical10k diagnostic PASS, terminal exit0.
One earlier preflight failure occurred before any workload; counsel approves
only its absolute-path correction. [Preflight receipt](qa-261005-checkpoint-window-preflight-correction.md).
[Source freeze](qa-261005-checkpoint-window-source-freeze.md),
[protocol/gates](qa-261005-checkpoint-window-capture-ledger.md).

Command (cwd src-tauri):

```text
cargo test --release --locked --features devtools --test checkpoint_window_plan_profile canonical_10k_actual_checkpoint_compare_event_window_plan -- --exact --ignored --nocapture --test-threads=1
```

One build job, existing release target; owned TEMP/TMP at workspace .qa/linker-temp,
all ancestors checked non-reparse. Stage-timing opt-in unset. Disk admission
500,000,000 bytes for this 10k-only smoke is not the independent 1M admission.
No overlapping Cargo/rustc/link/seeder/profile process before execution.
Executable SHA-256 `111cd9c1416869f1d1e4cfafcafd7681b10785f4033cdc4afc0fd34ef3e90717`;
seeder SHA-256 `9098e91763876d3535d91dbf53290a0f4f9622bc5e1bc6945deb44d485c0ebfa`;
unchanged fixture source SHA-256 `c34d037eeae47ad86ac91e7e1907c4d3540c4d101adab592ca018aea49ba1782`.
All three rechecked after execution; eight reviewed source identities are
owned by the source freeze, not historical PAGE/rollback artifacts.

Two real compare(Some) calls, one inactive reference and one captured, return
fully equal CheckpointDiff values; no raw result/ID/bind/time is dumped.
One actual window invocation prepares three binds/parameters, starts and
completely decodes245 IDs. Typed EQP is a complete covering indexed SEARCH
via idx_events_timestamp with lower/upper timestamp constraints. Existing
inclusive/normalized/order/cap behavior is separately checked in owner tests.
This is one compare subquery at10k, not by_id/items reads, the whole comparison
plan, all scales/filter branches or p95/seed acceptance. Seed timing below is
setup diagnostic, not a new performance acceptance result.

Complete retained stdout/stderr:

```text
    Finished `release` profile [optimized] target(s) in 2.90s
     Running tests\checkpoint_window_plan_profile.rs (target\release\deps\checkpoint_window_plan_profile-7412c5938e94396d.exe)

running 1 test
test canonical_10k_actual_checkpoint_compare_event_window_plan ... seed timings: elapsed_make_batch_ms=150.881 insert_events_ms=2011.936 graph_ms=60.005 inventory_ms=97.564 checkpoints_ms=199.559 investigations_ms=485.514 total_run_ms=3088.889
phase20 seed events=10000 payload_profile=canonical elapsed_ms=4184.935
phase20 corpus events=10000 payload_profile=canonical entities=3000 related=1500 bursts=800 checkpoints=40 snapshots=9800 investigations=20 span_days=90
actual checkpoint event-window plan: corpus_rows=10000 comparison_calls=2 captured_window_invocations=1 decoded_rows=245 full_return_parity=1 whole_compare_plan_claimed=0 timing_acceptance=0 evidence=CheckpointWindowPlanEvidence { invocations: 1, prepared: true, bindings: Some(3), statement_parameters: Some(3), explain: Complete([PagePlanNode { id: 4, parent: 0, kind: Events(PagePlanAccess { operation: Search, index: Some("idx_events_timestamp"), covering: true, constraints: [PagePlanConstraint { column: "timestamp", kind: Lower }, PagePlanConstraint { column: "timestamp", kind: Upper }] }) }]), query_started: true, fully_decoded: true, decoded_rows: Some(245) }
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 4.61s
```

Root checks zero selected owned runners and zero what-changed-perf-* fixtures
post-exit. PerfFixture shuts down/joins AppState, closes SQLite and removes only
its exclusively created nonce root. Other .perf-temp entries remain untouched,
including the unrelated seed-size-probe. C2,153,209,856/D3,316,060,160 bytes free
at the post-run check. No current-user AppData, installed native app, account
creation, autostart or external publication is involved.

Unresolved questions: remaining complete service/branch/scale plan evidence,
frozen-RC latency refresh, separate Windows test-account name/profile and
independent native/release/license gates. No acceptance checkbox closes.

