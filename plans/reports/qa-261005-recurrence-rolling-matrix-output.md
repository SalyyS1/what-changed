# Guarded recurrence aggregate gate output — 2026-10-05

Complete command output for the unchanged profiler-free query gate; source,
commands and artifact hashes: [matrix report](qa-261005-recurrence-rolling-query-matrix.md).
All 30 observations per shape contribute to p95. Individual call durations
are not exported; this records aggregate output, not raw-sample archival.

## 10k — exit 0

```text
Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 47.64s
     Running tests\perf_queries.rs (target\release\deps\perf_queries-5447f552e83eb02d.exe)

running 1 test
test all_eight_query_shapes_meet_the_selected_budget ... seed timings: elapsed_make_batch_ms=68.313 insert_events_ms=859.256 graph_ms=27.471 inventory_ms=42.002 checkpoints_ms=91.939 investigations_ms=228.709 total_run_ms=1366.467
phase20 seed events=10000 payload_profile=canonical elapsed_ms=2044.657
phase20 corpus events=10000 payload_profile=canonical entities=3000 related=1500 bursts=800 checkpoints=40 snapshots=9800 investigations=20 span_days=90
phase20 search_stages checkpoint_ms=0.056 fts_join_sort_ms=0.692 hydrate_ms=0.364 checkpoint_hits=0 event_hits=50
phase20 summary_stages four_counts_ms=0.233 top_important_ms=0.195 latest_checkpoint_ms=0.044 total_count=10000 important_hits=3 checkpoint_present=true
phase20 rows=10000 shape=get_events_first_page_200 samples=30 p95_ms=0.719 budget_ms=10
phase20 rows=10000 shape=get_events_page_10_keyset samples=30 p95_ms=0.767 budget_ms=10
phase20 rows=10000 shape=get_dashboard_summary samples=30 p95_ms=0.140 budget_ms=15
phase20 rows=10000 shape=search_events_two_token samples=30 p95_ms=1.051 budget_ms=40
phase20 rows=10000 shape=get_event_details_and_prior samples=30 p95_ms=0.064 budget_ms=10
phase20 rows=10000 shape=start_investigation_5k samples=30 p95_ms=79.222 budget_ms=400
phase20 rows=10000 shape=compare_checkpoint_3k samples=30 p95_ms=16.693 budget_ms=150
phase20 rows=10000 shape=get_privacy_summary samples=30 p95_ms=0.377 budget_ms=30
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 5.06s
```

## 100k — exit 0

```text
Finished `release` profile [optimized] target(s) in 1.74s
     Running tests\perf_queries.rs (target\release\deps\perf_queries-5447f552e83eb02d.exe)

running 1 test
test all_eight_query_shapes_meet_the_selected_budget ... seed timings: elapsed_make_batch_ms=1328.988 insert_events_ms=32287.723 graph_ms=1246.364 inventory_ms=51.034 checkpoints_ms=111.400 investigations_ms=340.961 total_run_ms=35466.691
phase20 seed events=100000 payload_profile=canonical elapsed_ms=35563.183
phase20 corpus events=100000 payload_profile=canonical entities=3000 related=15000 bursts=8000 checkpoints=40 snapshots=9800 investigations=20 span_days=90
phase20 search_stages checkpoint_ms=0.062 fts_join_sort_ms=4.830 hydrate_ms=0.439 checkpoint_hits=0 event_hits=50
phase20 summary_stages four_counts_ms=0.998 top_important_ms=0.257 latest_checkpoint_ms=0.043 total_count=100000 important_hits=3 checkpoint_present=true
phase20 rows=100000 shape=get_events_first_page_200 samples=30 p95_ms=0.643 budget_ms=20
phase20 rows=100000 shape=get_events_page_10_keyset samples=30 p95_ms=0.721 budget_ms=20
phase20 rows=100000 shape=get_dashboard_summary samples=30 p95_ms=0.440 budget_ms=30
phase20 rows=100000 shape=search_events_two_token samples=30 p95_ms=3.855 budget_ms=90
phase20 rows=100000 shape=get_event_details_and_prior samples=30 p95_ms=0.082 budget_ms=15
phase20 rows=100000 shape=start_investigation_5k samples=30 p95_ms=147.841 budget_ms=600
phase20 rows=100000 shape=compare_checkpoint_3k samples=30 p95_ms=23.828 budget_ms=200
phase20 rows=100000 shape=get_privacy_summary samples=30 p95_ms=0.585 budget_ms=60
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 41.39s
```

## 1M — exit 0; independent seed target missed

```text
    Finished `release` profile [optimized] target(s) in 0.64s
     Running tests\perf_queries.rs (target\release\deps\perf_queries-5447f552e83eb02d.exe)

running 1 test
test all_eight_query_shapes_meet_the_selected_budget ... seed timings: elapsed_make_batch_ms=7139.680 insert_events_ms=294049.614 graph_ms=12955.321 inventory_ms=42.123 checkpoints_ms=101.778 investigations_ms=2041.625 total_run_ms=316393.995
phase20 seed events=1000000 payload_profile=canonical elapsed_ms=316438.748
phase20 corpus events=1000000 payload_profile=canonical entities=3000 related=150000 bursts=80000 checkpoints=40 snapshots=9800 investigations=20 span_days=90
phase20 search_stages checkpoint_ms=0.065 fts_join_sort_ms=41.065 hydrate_ms=0.444 checkpoint_hits=0 event_hits=50
phase20 summary_stages four_counts_ms=2.880 top_important_ms=0.208 latest_checkpoint_ms=0.060 total_count=1000000 important_hits=3 checkpoint_present=true
phase20 rows=1000000 shape=get_events_first_page_200 samples=30 p95_ms=0.633 budget_ms=45
phase20 rows=1000000 shape=get_events_page_10_keyset samples=30 p95_ms=0.973 budget_ms=45
phase20 rows=1000000 shape=get_dashboard_summary samples=30 p95_ms=2.742 budget_ms=80
phase20 rows=1000000 shape=search_events_two_token samples=30 p95_ms=47.948 budget_ms=250
phase20 rows=1000000 shape=get_event_details_and_prior samples=30 p95_ms=0.301 budget_ms=30
phase20 rows=1000000 shape=start_investigation_5k samples=30 p95_ms=1032.501 budget_ms=1500
phase20 rows=1000000 shape=compare_checkpoint_3k samples=30 p95_ms=18.531 budget_ms=300
phase20 rows=1000000 shape=get_privacy_summary samples=30 p95_ms=0.458 budget_ms=200
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 351.59s
```
