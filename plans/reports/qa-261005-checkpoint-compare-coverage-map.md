# Actual checkpoint-compare coverage map — 2026-10-05

Read-only scout, no edits/workloads. This is a completion map, not another
executed plan receipt or acceptance closure. Full plan314/369,55 open/P20 6/23.

The accepted canonical3k-item timing cell calls compare(left, Some(right)).
[Service](../../src-tauri/src/checkpoint/service.rs) executes five owner reads:
by_id twice, items twice (3k per side), then event_ids_in_window once, across
three reader leases. In-memory item comparison/indexing, semantic changes,
sorting/grouping and missing-collector reconciliation are not SQL plans.
[Repository](../../src-tauri/src/storage/checkpoint_repository.rs) by_id reads
checkpoint metadata by integer primary key and decodes kind/manifests; items
filters snapshots by checkpoint_id, orders collector/kind/external identity
and fully decodes state JSON. Their expected indexed paths come from source/
[migration](../../src-tauri/migrations/005_checkpoint_items.sql), not actual
canonical capture. Error/missing/decode paths short-circuit; successful2/2/1
counts must not be required from errors. compare(None) reads current manifest
and durable baselines and is outside the accepted Some(right) timing cell.

Actual evidence currently covers only the window at canonical10k:245 IDs,
three binds, complete covering timestamp-index SEARCH and return parity.
[Receipt](qa-261005-checkpoint-window-capture-ledger.md). Legacy copied-SQL
perf_query_plans is not an actual-service canonical proof. P20 expects indexed
plans/no bare events scan for measured queries across10k/100k/1M.

One cohesive next proposal: record all five owner reads from one successful
compare(Some) scope, reuse the window evidence, add minimal typed checkpoint
metadata/snapshot plan evidence, keep SQL/binds/order/decode/errors and service/
diff algorithm unchanged. No generic trace framework, schema/index changes,
PerfFixture edit or immediate workload is proposed by this read-only map.
Counsel GO authorizes preparation and ordinary verification of this cohesive
envelope, not ignored workloads or scale runs. Preserve existing window API/
historical receipt, distinct operation identities and invocation order. Record
each of the three calling-thread reader leases independently; do not imply one
shared database snapshot. Metadata/items complete only after fallible domain/
JSON decode succeeds. Retain temporary ordering operations: indexed filtering
does not prove fully covered sorting. Unknown plans remain incomplete evidence
without changing the real result. Execution still needs fresh counsel after
review/tests/lint; the old window smoke is not whole-comparison proof.

Prior full-count recurrence canonical captures exist at their old revision;
current rolling full/delta has actual-bound unit proof but no canonical service
capture receipt. This distinction remains separate from checkpoint coverage.

Unresolved questions: actual metadata/items plans, complete Some(right) SQL
coverage and larger canonical scales; full p95/native/release gates remain open.
