---
title: The product plan grows past the installer
date: 2026-09-10
summary: Production-readiness audit extended the accepted What Changed plan from MVP implementation through an evidence-gated public v1.
---

# The product plan grows past the installer

## What happened

Reviewed the full What Changed codebase against its accepted plan. The foundation is real: shell/design system, privacy chokepoint, SQLite layer, typed IPC and tray lifecycle. The user-facing product loop is not complete: event commands remain tracked stubs and collectors, investigation, maps and trust/release surfaces still belong to phases 06–21.

Verification was honest. Frontend: 111 tests passed and the production build completed. The broad default-parallel Rust run reached `rustc-LLVM ERROR: out of memory` and then emitted damaged-artifact follow-on errors. Re-running the critical suites with `CARGO_BUILD_JOBS=1` passed 64 tests across architecture, capabilities, command contracts, recovery, migrations, redaction, repository and wiring. The plan now treats resource-bounded Cargo execution as a release requirement rather than hiding the OOM.

The audit identified concrete plan owners for atomic settings updates, recovery authorization, FTS-aware deletion proof, commit-loss telemetry, truthful recovery/degraded status, Change Map information architecture and resource-bounded release gates. Report: `plans/reports/code-review-260910-0023-production-readiness.md`.

## Decisions

The user chose to extend the accepted plan rather than create a competing roadmap. It now has 25 phases and 183 estimated hours. Phases 22–25 add a tested before/after brand system, English-canonical repository wiki, English plus Vietnamese launch story/media, and a public unsigned-v1 go/no-go.

The project direction is open source. The exact OSI-compatible license is unresolved and blocks public release, not implementation. The public target remains unsigned, offline at runtime, non-admin, manual updates and Docker live-path untested on this machine. D-15 remains unchanged: report total and private memory, gate on neither.

## Friction

All planner/reviewer subagent dispatches failed before starting because the runtime `WorktreeCreate` hook succeeded without returning a worktree path. The controller completed the evidence review and red-team roles locally and recorded that limitation; no fake parallel-review claim was made.

## Next steps

1. Resolve exact open-source license and Change Map route/surface decision.
2. Start Phase 06: atomic settings/ledger hardening, query services and synthetic evidence corpus.
3. Keep critical Rust verification serialized until Phase 20 measures a safe job count.
4. Do not publish wiki, media or release artifacts before the Phase 25 approval gate.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
