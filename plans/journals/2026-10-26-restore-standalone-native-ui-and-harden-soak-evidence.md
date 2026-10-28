---
title: Restore standalone native UI and harden soak evidence
date: 2026-10-26
summary: Verified current-account native UI; repaired standalone build and kept unmet acceptance gates open.
---

# Restore standalone native UI and harden soak evidence

## What happened

The user authorized native QA on the current Windows account. Verified online and full offline backups preserved the original profile. Fresh-profile directory isolation failed before any move; bounded existing-profile QA was used instead.

A raw Cargo release executable attempted localhost:5183 and displayed WebView's error page. Added a standalone build command with tauri/custom-protocol and rebuilt the frontend. Fixed two wrapper defects: Get-FileHash module discovery under nested PowerShell 5.1, and stale artifact selection with configured Cargo targets. Exact Cargo artifact selection and disposed .NET SHA256 now pass 14 focused checks and independent review. Corrected full build exited 0.

The native-tested standalone binary rendered seven primary routes with the real backend and no Vite server. Search focus/Escape, workspace control visibility and Privacy access passed. Three 900×600 emulated viewports had no main/document overflow. QA exited 0 via real tray Quit; backed up the old engineering executable, restored the native-tested version at the local monitoring path, and relaunched minimized without debugging. No original data was moved or overwritten.

Backend soak hardening passed 8 unit / 5 observer / 1 join tests; library 571 passed / 3 ignored, strict Clippy and focused PowerShell protocol checks passed. Live bounded diagnostic rejected unavailable Docker as intended; this was not a healthy soak or native acceptance.

## Decisions

After the paired diagnostic, advisory review authorized one unchanged original acceptance invocation. It passed at 13.7265 ms process / 0.3222 ms listener, 2 passed / 1 diagnostic ignored. Targeted strict Clippy passed. No further repeat or production change followed. P09 stays open because its recorded requirement says one calm pass does not establish repeatability.

Retain the unchanged 15 ms process budget and all four failed measured results. Preserve earlier mutex-poison recovery so one failure cannot mask listener results. Keep backend diagnostics, native engineering checks, installed acceptance and publication claims separate. No release/publication, threshold relaxation, production caching or personal data export.

## Remaining

Plan 316/369 checked, 53 open. Installer, four-hour all-collector soak, full manual accessibility, notice/release and other performance gates remain open. The reviewed ignored diagnostic compiled and completed once: all 50 cycles Noop; total p95 12.6546 ms, provider 12.6185 ms, remaining snapshot 0.0406 ms. This identifies the quiet sample's dominant stage, not the cause of earlier failures or an acceptance pass. See plans/reports/qa-261026-continuation.md and process-cost-scout-261026.md. AgentWiki publish skipped.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
