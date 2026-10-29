# Actual summary query-plan gap

Read-only review by `soak_review`, 2026-10-27, accepted `--auto --advise` scope.
No source changes or acceptance pass from this inspection.

`src-tauri/tests/perf_query_plans.rs` explains copied summary edge/daily SQL.
Production `event_repository::counts_for_range` instead prepares one six-bind
statement combining both edges and daily counts; its final edge is inclusive,
while the copied test uses `<`. The same-day inclusive branch is also absent.
The full summary service additionally calls `top_important` and checkpoint
`latest`; their copied/narrowed statements do not bind the service to the plans.

Recommended next bounded repair: devtools-only capture at actual prepare/bind/
decode points in the summary owners, using the existing page/checkpoint capture
lifecycle. Invoke `summary_service::summary` and replace only the copied summary
assertions. No raw SQL/parameter logging, new exported SQL constants or changed
budgets. Prove captured/uncaptured full DTO equality, same/multi-day and empty
branches, invalid range behavior, existing checkpoint, complete decode, capture
cleanup, and index use on actual events/daily queries.

Other actual-bound proofs already exist for page, complete checkpoint reads and
rolling recurrence. Reuse them. Search, details, non-recurrence investigation,
privacy and canonical corpus/scale correspondence remain full-gate work.

Root preserves product source while the approved-notice installer compiles;
this finding is queued for subsequent source work, not silently applied mid-build.

Unresolved: measured plans for actual combined/same-day summary; remaining
service paths and canonical fixture parity. Full P20 checkbox remains open.
