---
title: "Storage lands, and a coloured log line gives up a password"
date: 2026-09-06
summary: "Phase 04 verified green, and the new raw-bytes leak assertion exposed a real URI-credential leak on the console sink"
---

# Storage lands, and a coloured log line gives up a password

## What happened

The persistence layer landed: sixteen tables in one migration, a checksummed
migration ledger with drift detection, one writer thread with a bounded queue and
250 ms / 200-event coalescing, a four-connection read-only pool, keyset paging,
FTS5 search over an external-content table, retention that spares
checkpoint-owned snapshots, corruption as a state with three user-driven recovery
actions, and a crash-loop guard. Eight commits on `main`, 133 tests green, clippy
clean under `-D warnings`.

Two things worth remembering came out of the verification, not the writing.

**A coloured log line gave up a password.** The tester's report said the phase's
last success criterion — no fixture secret in the raw database bytes — had no test
at all. Writing it was easy and it passed on the first run. What it also did was
print the leak suite's log lines to a captured console, and there sat
`postgres://whatchanged_app:FixtureOnlyNotReal99@db.internal.example:5432/...`
in the clear, three lines above the same fixture's mongodb sibling with its
credential correctly replaced. The rule was
`(?i)\b[a-z][a-z0-9+.-]*://([^\s/@:]+:[^\s/@]+)@`. The development console layer
formats with ANSI, and its reset sequence ends in `m`, which lands directly
against the value — so `\b` never holds and the match can never start.
`mongodb+srv://` escaped the bug only because the `+` in its scheme offered the
regex a legal starting position a few characters later. The file sink, formatted
without colour, redacted both. Dropping the anchor fixes it; the leak suite now
replays all twelve fixtures through a coloured formatter as well as a plain one,
which is the assertion that would have caught it a phase earlier.

The lesson is narrower than "test more": a redaction rule must not depend on what
precedes the value, because the thing that precedes it is chosen by a formatter,
not by the code under test.

**The Bash tool died mid-phase.** Every call, including `pwd`, started failing
with `bash: -c: line 199: unexpected EOF while looking for matching '"'` — a fault
in the harness-generated wrapper, not in any command. It reproduced with no quotes
in the command, with the sandbox disabled, and inside freshly spawned subagents,
while the shell snapshots on disk were clean and the user has no shell profile.
The workaround that got the phase finished: a `.cmd` script in `%TEMP%` that
appends each command's output and exit code to a log file, registered as a Claude
Terminal quick action, fired through MCP, then read back with the file reader.
Four cargo runs, two commits and the frontend gates all went through that path.

## Decisions

- **D-12** — a checkpoint's snapshots are exempt from the seven-day snapshot
  window. Otherwise "compare with checkpoint" breaks exactly one week after the
  user thought they had saved something.
- **D-18** — the data folder comes from the platform known-folder API, not the
  framework's path resolver, because logging and the database are up before the
  app object exists and a corrupt database has to be a state the first window can
  render.
- **D-19** — dependencies compile without debug info in the dev profile. The
  profile had reached 4.7 GB on a volume that ran out of space four times, and the
  failures look like linker faults rather than disk faults.

## Measurements

Ten thousand events insert in 1.74–2.44 s idle (4,107–5,737 events/s in batches
of 200) and 5.09 s with a second build running. A page 5,000 rows deep returns 200
rows in 1.08–4.14 ms. Storage is 1,030 bytes per event including the write-ahead
log. All four runs are in `docs/PERFORMANCE.md` with the machine state, because
the 2.9× spread is contention and averaging it would hide that.

## Next steps

- Phase 05 (typed IPC, tray, lifecycle) is next, and its first real boot is what
  proves the one criterion this phase could only assert structurally: the database
  landing under the resolved `%APPDATA%` folder rather than a temporary root.
- Restart the CLI to get the shell back; the quick-action path works but costs a
  round trip per command.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
