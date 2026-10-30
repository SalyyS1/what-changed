# Guarded recurrence canonical query matrix — 2026-10-05

Outcome: frozen production-candidate 10k, 100k and 1M matrices pass 8/8 each.
All 24 query-budget cells pass. Independent 1M seed target remains missed.
[Production verification/source revision](qa-261005-recurrence-rolling-production.md).

## Reproduction and provenance

Workdir `src-tauri`; existing target, one root Cargo/one job, owned `.perf-temp`
TEMP/TMP. `WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS` unset; selected rows:

```powershell
cargo test --release --locked --features devtools --test perf_queries all_eight_query_shapes_meet_the_selected_budget -- --ignored --exact --nocapture --test-threads=1
```

Same frozen source/artifacts for all selected scales; no tuning or unchanged
rerun for green. All 30 samples per shape contribute to p95; individual
sample durations are not exported. No raw-call archival/reconstruction,
outlier removal, host-cause inference or cross-run causal speedup claim.
Complete aggregate command output is retained in the [gate output record](qa-261005-recurrence-rolling-matrix-output.md).

| Source/artifact | SHA-256 |
|---|---|
| tests/perf_queries.rs | DB0E96813370A4FC0DCA12C10271FB9D620F654BDFB281CD30BE862B07FFCB55 |
| tests/support/perf_fixture.rs | C34D037EEAE47AD86AC91E7E1907C4D3540C4D101ADAB592CA018AEA49BA1782 |
| src/bin/seed_history.rs | 46E6A936BA886A4FC6C7DAA3FEBFD59B8DDEEC067B3548DD9EC65CB23280E624 |
| src/analysis/graph.rs | DAF84DD922DA45E4E4DCF1A2323C2E5AACBF3435F11F50EA590E867BF57D7C98 |
| Cargo.toml | 5D64037750D5A074843ED2D4F46D1EC65AC709EC5D8825210C7A0F214ECDA65D |
| target/release/deps/perf_queries-5447f552e83eb02d.exe | 3E634F0F7D2427FD044B6FD102F3968CC6B941A32F30EB5A3718EAA45840F584 |
| target/release/seed_history.exe | F295DBD0860AD54AD5D04A0298ABF4B3013A58623C2952946A25FB13C5755BD0 |

Paths above are relative to `src-tauri/`; the production helper/capture source
hashes are in the linked revision ledger. Pinned release, locked dependencies,
canonical recipe, actual service calls, scoring/persistence assertions and
budgets are unchanged. No profile diagnostics were rerun.

## Measured cells

Each cell is p95 / budget, milliseconds; all cells pass.

| Actual service shape | 10k | 100k | 1M |
|---|---|---|---|
| get_events_first_page_200 | 0.719 / 10 | 0.643 / 20 | 0.633 / 45 |
| get_events_page_10_keyset | 0.767 / 10 | 0.721 / 20 | 0.973 / 45 |
| get_dashboard_summary | 0.140 / 15 | 0.440 / 30 | 2.742 / 80 |
| search_events_two_token | 1.051 / 40 | 3.855 / 90 | 47.948 / 250 |
| get_event_details_and_prior | 0.064 / 10 | 0.082 / 15 | 0.301 / 30 |
| start_investigation_5k | 79.222 / 400 | 147.841 / 600 | 1032.501 / 1500 |
| compare_checkpoint_3k | 16.693 / 150 | 23.828 / 200 | 18.531 / 300 |
| get_privacy_summary | 0.377 / 30 | 0.585 / 60 | 0.458 / 200 |

10k: one test passed, zero failed, three filtered in 5.06 s; compilation
47.64 s. Seed wall 2,044.657 ms, inner 1,366.467 ms: generation 68.313,
insertion 859.256, graph 27.471, inventory 42.002, checkpoints 91.939,
investigations 228.709 ms. Canonical counts: 10k events, 3k entities,
1,500 related facts, 800 burst rows, 40 checkpoints, 9,800 snapshots,
20 investigations over 90 days. Warm actual investigation exercises scored,
persisted 5,000 inputs. Owned fixture cleanup completes without error.

100k: one test passed, zero failed, three filtered in 41.39 s. Seed wall
35,563.183 ms; inner 35,466.691 ms: generation 1,328.988, insertion
32,287.723, graph 1,246.364, inventory 51.034, checkpoints 111.400,
investigations 340.961 ms. Canonical counts: 100k events, 3k entities,
15k related facts, 8k burst rows, 40 checkpoints, 9,800 snapshots,
20 investigations over 90 days. Scoring/persistence warm assertions pass.
Owned fixture cleanup completes without error.

1M: one test passed, zero failed, three filtered in 351.59 s; cached build
0.64 s. Seed wall 316,438.748 ms / 240,000 ms — MISS; inner 316,393.995 ms:
generation 7,139.680, insertion 294,049.614, graph 12,955.321, inventory
42.123, checkpoints 101.778, investigations 2,041.625 ms. Canonical counts:
1M events, 3k entities, 150k related facts, 80k burst rows, 40 checkpoints,
9,800 snapshots, 20 investigations over 90 days. Warm scored/persisted 5k
assertions pass. Cleanup reports no error; owned fixture is absent and the
Cargo/test/seeder processes exited. Root rehashed both artifacts after the
terminal run; SHA-256 matches the frozen entries above.

## Acceptance boundaries

Counsel GO closes exactly the two P20 query-budget checkboxes. The prior
1M 1629.336/1500 ms FAIL is historical for the earlier revision, not erased.
Seed remains 76.439 seconds over target. Counsel selects explicitly enabled,
devtools-only batch insertion attribution before any insertion optimization.
No unchanged matrix rerun, threshold change, outlier exclusion or causal
cross-run speedup claim. SQL-plan branch parity, resources/soak, installed
accessibility, DB-size sensitivity and release remain independent and open.

Unresolved questions: seed attribution/repair and remaining installed/release
evidence. No entire phase or release verdict promoted.
