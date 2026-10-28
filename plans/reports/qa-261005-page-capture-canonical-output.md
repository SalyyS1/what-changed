# Actual-bound canonical page smoke — 2026-10-05

Status: exactly one approved canonical10k diagnostic PASS, exit0. No retries.
[Frozen source](qa-261005-page-capture-source-freeze.md),
[protocol](qa-261005-page-plan-capture-ledger.md),
[caller output](qa-261005-page-capture-callers-output.md),
[strict lint](qa-261005-page-capture-lint-output.md).

Command: `cargo test --release --locked --features devtools --test page_plan_capture_profile canonical_10k_actual_first_and_tenth_page_plans -- --exact --ignored --nocapture --test-threads=1`.
One build job, existing release target, workspace-owned TEMP/TMP. Frozen
executable SHA-256 `361ee62e997545141a4aa8372fbd05a09869119fb734de3f508d3a32fd51ddbc`.
Unchanged canonical fixture SHA-256 `c34d037eeae47ad86ac91e7e1907c4d3540c4d101adab592ca018aea49ba1782`.

Actual first-page and page-ten calls both completely decode201 rows including
lookahead; expected200 raw rows, cluster expansion, strict ordering and cursor
contracts pass. Typed EQP records indexed SEARCH events using
idx_events_timestamp, timestamp lower/upper bounds. Exactly10 actual calls;
eight intermediate calls uncaptured. No identifying binds/SQL or raw EQP
text retained. Captures stay outside p95 acceptance.

```text
    Finished `release` profile [optimized] target(s) in 3.79s
     Running tests\page_plan_capture_profile.rs (target\release\deps\page_plan_capture_profile-231292b0146e9340.exe)

running 1 test
test canonical_10k_actual_first_and_tenth_page_plans ... seed timings: elapsed_make_batch_ms=92.412 insert_events_ms=1070.940 graph_ms=30.943 inventory_ms=88.427 checkpoints_ms=208.136 investigations_ms=429.663 total_run_ms=1961.895
phase20 seed events=10000 payload_profile=canonical elapsed_ms=2805.159
phase20 corpus events=10000 payload_profile=canonical entities=3000 related=1500 bursts=800 checkpoints=40 snapshots=9800 investigations=20 span_days=90
actual page plan: corpus_rows=10000 page_number=1 timing_acceptance=0 evidence=PageExecutionEvidence { variant: PagePlanVariant { mask: 0, category_arity: 0, importance_arity: 0 }, invocations: 1, prepared: true, bindings: Some(3), statement_parameters: Some(3), explain: Complete([PagePlanNode { id: 5, parent: 0, kind: Events(PagePlanAccess { operation: Search, index: Some("idx_events_timestamp"), covering: false, constraints: [PagePlanConstraint { column: "timestamp", kind: Lower }, PagePlanConstraint { column: "timestamp", kind: Upper }] }) }]), query_started: true, fully_decoded: true, decoded_rows: Some(201) }
actual page plan: corpus_rows=10000 page_number=10 actual_page_invocations=10 timing_acceptance=0 evidence=PageExecutionEvidence { variant: PagePlanVariant { mask: 8, category_arity: 0, importance_arity: 0 }, invocations: 1, prepared: true, bindings: Some(5), statement_parameters: Some(5), explain: Complete([PagePlanNode { id: 5, parent: 0, kind: Events(PagePlanAccess { operation: Search, index: Some("idx_events_timestamp"), covering: false, constraints: [PagePlanConstraint { column: "timestamp", kind: Lower }, PagePlanConstraint { column: "timestamp", kind: Upper }] }) }]), query_started: true, fully_decoded: true, decoded_rows: Some(201) }
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 3.02s

```

The owned fixture drops, closes/joins and removes its reserved directory before
the terminal PASS. Root checks no remaining what-changed-perf-* fixture or owned
Cargo/rustc/seeder/page-profile process. One pre-existing unowned
what-changed-seed-size-probe-* directory is left untouched; do not claim the
entire reserved parent is empty. No native app or existing personal history used.

Unresolved questions: other page filter variants on canonical corpora, larger
scales, other actual-bound service owners and fresh current-source p95 acceptance.
This two-shape10k result does not close either full query-plan checkbox or P20.

