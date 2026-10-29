---
title: Phase 09 process and port collector hardening
date: 2026-09-19
summary: "Fixed identity confidence, durable batching, churn, parser, and performance evidence before accepting Phase 09."
---

# Phase 09 process and port collector hardening

**Date**: 2026-09-19 19:28
**Severity**: High
**Component**: Windows process and TCP/UDP listener collectors
**Status**: Resolved

## What Happened

Phase 09 delivered non-admin Win32 process and listener polling, but review exposed failures hidden by happy-path tests. `src-tauri/src/collectors/process/state.rs` reused an acknowledged instance wholesale: after path access was denied, stale `path_available` and creation evidence could still publish High confidence. Worse, overwriting or failing to refresh creation time made PID reuse across denied snapshots ambiguous. Durable pending batches also lost provider target, original observation time, and downtime uncertainty across restart. Churn summaries could split or strand events after a backlog crossed the 60 s episode boundary. Socket parsers accepted malformed over-counted or truncated tables. The initial performance test timed only DTO conversion under a frozen clock, not a real snapshot cycle.

## The Brutal Truth

The frustrating part was that focused green tests were measuring convenient code paths, not the failure boundaries users depend on. Live-idle assumptions were equally brittle: RuntimeBroker, Edge, WMI and security software produced genuine churn while the gate expected a quiet machine. Disk pressure made every broad Cargo rerun expensive; the Phase 09 target alone approached 2.7 GB while free space hovered near 6 GB.

## Technical Details

Identity now separates stable creation evidence from current availability, keeps structural entity IDs stable, and downgrades confidence immediately when path/time cannot be read. Pending batches preserve their target and timestamps through serialization and chunks above 200 records. Churn backlog remains durable until one bounded summary is acknowledged. TCP/UDP v4/v6 parsing fails closed on partial buffers, impossible row counts, or truncated rows. Reusable provider buffers and indexes replaced whole-table allocation. The corrected Windows gate measures provider enumeration, state/index work, and diff together: process p95 11.8723 ms; listener p95 353.5 microseconds.

## What We Tried

We rejected recomputing identity from whichever path happened to be readable because access changes would fabricate new entities. We rejected partially accepting malformed socket tables because a partial snapshot would emit false closes. Real-machine idle retries were retained as environmental evidence, not substituted for deterministic transition tests.

## Root Cause Analysis

The design conflated durable identity with current read confidence, treated atomic delivery metadata as transient, and optimized/testing sub-steps instead of the complete collector cycle.

## Lessons Learned

Preserve identity facts while separately recording whether they were observable now. A snapshot is all-or-nothing. Performance gates must include the actual hot path. A live Windows desktop is not a deterministic fixture.

## Next Steps

The implementation lead owns Phase 10 next. Phase 20 must repeat allocation profiling under production load. Phase 05 still owns installer and full-app memory evidence. Manual Notepad plus `python -m http.server 4000` smoke passed; final scoped review scored 9.5/10 with zero findings.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
