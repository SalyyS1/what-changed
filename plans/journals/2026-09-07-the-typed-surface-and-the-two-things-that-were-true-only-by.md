---
title: "The typed surface, and the two things that were true only by accident"
date: 2026-09-07
summary: "Phase 05 landed the typed IPC surface, tray and event bus; the architecture gate and a tester pass each caught something green tests were hiding"
---

# The typed surface, and the two things that were true only by accident

# The typed surface, and the two things that were true only by accident

Phase 05 gave the shell a spine: 33 typed commands, generated TypeScript
bindings, the seven-entry tray, close-to-tray, single instance, and an event
bus that coalesces a burst of a thousand inserts into at most two messages.
208 Rust tests and 111 frontend tests, all green.

Three things are worth writing down, and none of them is the feature list.

## The plan's binding generator did not exist

The plan pinned `tauri-specta 1.0.2`. Its own dependency list declares
`tauri ^1.2.4` — it is the Tauri **v1** API, whose entry point is
`ts::export(collect_types![…], path)`, a shape Tauri 2 does not have. The
Tauri-2 line is `2.0.0-rc.25` and pins four pre-release crates to generate the
frontend's type definitions.

The plan had pre-decided the fallback, with a signal: "compile error in the
export path". The signal arrived before a line was written, which is the better
outcome and only happened because the risk table said what to look for.

So `ts-rs` generates the types and the command wrappers are hand-written. That
trade is the interesting part: `ts-rs` knows nothing about `#[tauri::command]`,
so the wrappers are a place the two sides can disagree. Recorded as D-14 with
that cost stated rather than buried.

## The architecture test caught me

`cargo test` failed on `app/settings_service.rs reaches for rusqlite directly`.
I had put the settings service in `app/`, where it read the settings table —
violating "SQL only under `storage/`", an invariant this project has had since
phase 04 and which I wrote code against without noticing.

The fix was not an exemption. Settings are domain state — the retention policy
decides what the maintenance pass deletes — so the vocabulary moved to
`domain/settings.rs` and the key/value-to-object assembly moved into
`storage/settings_repository.rs`, beside the table it reads. The DTO module
became a re-export. Settings now have one definition instead of two, which is
better than what I originally wrote, and I would not have found it by reading.

A gate that only fires on other people's mistakes is not a gate.

## Two things were true only by accident

A `tester` pass on the finished code found two failures that every gate was
green through.

**The settings round-trip was write-only.** `get_settings` was called and its
answer thrown away: the store's `current` had no readers, and the theme control
read `localStorage`. Rust stored the theme; nothing read it back. The criterion
"persists across restart" passed — because of the localStorage mirror that
exists for first paint, not because of anything the phase built. Clear the key
and the stored theme is ignored.

**Command parameter names were ungated.** The bindings emitted command *names*,
and the contract test compared names. But Tauri matches arguments by name, so
renaming `event_id` to `id` in Rust breaks the call at runtime while the build,
the bindings check and the contract test all stay green. That is exactly the
failure a typed IPC surface exists to prevent, and the surface did not prevent
it.

Both are fixed — the store hydrates the theme now, and the argument names are
read out of the command sources, converted to the camelCase Tauri puts on the
wire, generated into the bindings, and compared against the object literals the
client actually passes.

The lesson is the same one twice. A green suite says the assertions passed; it
does not say the assertions were about anything. Two of the phase's own tests
turned out to be tautologies — one asserted that `FORBIDDEN_PREFIXES` matches a
string built from `FORBIDDEN_PREFIXES` — and both are now built on the pattern
`tests/architecture.rs` already used: drive a deliberately bad input through the
same function the gate uses. If the bad input passes, the gate is decoration.

## What is not done

The NSIS build and memory measurement #2 both need a release profile. D: has
5 GB free against a 4.8 GB debug profile, so they run after
`cargo clean --profile dev`. The phase file says so rather than claiming green,
and the two criteria are unticked.

Criteria 2, 3 and 4 — close hides, seven tray items work, second launch
focuses — are verified structurally, not executed. The seams are tested; the
Tauri wiring around them needs a running window. That is stated in the phase
file too. The honest version of "done" has an asterisk on it, and the asterisk
is written down.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
