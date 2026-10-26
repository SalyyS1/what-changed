---
phase: 18
title: "Docker collector and developer-environment collector (dev mode)"
status: completed
priority: P2
effort: 6h
dependencies: [08, 10]
---

## Overview

Developer-specific change sources: Docker container/image lifecycle and the developer environment
itself (environment variables, PATH entries, toolchain versions, git branch of watched repos). Docker
Desktop is **not installed on this machine** (decision D3), so the live path cannot be verified here —
therefore the collector is written against a transport trait with a scripted fake covering the wire
protocol, and the only paths verifiable locally (absence, unavailability, reconnection) are the ones
this phase's gate asserts. The Docker-present path is explicitly listed as an unverified surface in the
phase file and in `ROADMAP.md` rather than being claimed as tested.

## Requirements

**Functional**

*Docker collector (§dev-tooling)*
- Detection: `\\.\pipe\docker_engine` exists → `Available`; missing → collector state `Unavailable`
  with the §33 copy "Docker isn't running. We'll start recording container changes when it does."
  Re-probe every 60 s; no error log per probe (one line on transition only).
- Transport: `DockerTransport` trait (`fn get(&mut self, path: &str) -> Result<Vec<u8>>`,
  `fn stream(&mut self, path: &str) -> Result<Box<dyn BufRead>>`). Windows implementation opens the
  named pipe with `CreateFileW(GENERIC_READ|GENERIC_WRITE, ...)` and speaks HTTP/1.1 by hand
  (`GET /v1.43/events?filters=... HTTP/1.1\r\nHost: docker\r\n\r\n`), parsing chunked responses — this
  avoids adding an HTTP client and a pipe-transport crate for two endpoints. API version is requested as
  `/v1.43/` with a fallback to `/v1.41/` when the daemon replies 400 with a version message; the
  negotiated version is recorded in `CollectorStatus.details`.
- Endpoints used: `GET /_ping` (health), `GET /version` (daemon version → `docker/daemon_changed`),
  `GET /events` (streamed), `GET /containers/json?all=1` and `GET /images/json` (baseline + resync after
  a stream drop). Nothing else — no exec, no create, no start, no stop (read-only by construction).
- Events mapped from the stream: `docker/container_started`, `docker/container_stopped`,
  `docker/container_died` (`exitCode != 0` → `importance: important` and the exit code in `details`),
  `docker/container_created`, `docker/container_removed`, `docker/image_pulled`,
  `docker/image_removed`, `docker/volume_created`, `docker/volume_removed`,
  `docker/network_created`, `docker/network_removed`, `docker/compose_project_changed` (derived from
  the `com.docker.compose.project` label when ≥ 2 containers of a project change within 30 s — one
  event instead of ten).
- Entity ids: `container:<name_or_short_id>` (name preferred — it survives recreation, which is the
  whole point when a compose service is rebuilt), `image:<repo:tag>`, `docker_volume:<name>`,
  `docker_network:<name>`. Port mappings from the event's `Actor.Attributes` create
  `container --publishes--> port` relationship rows, so P15 can link "container restarted" to
  "port 5432 closed".
- Stream resilience: on read error/EOF, mark `Reconnecting`, resync from `containers/json` +
  `images/json` (diffing against the last state set — no duplicate events for containers that never
  changed), and reconnect with 2 s → 30 s backoff. A daemon restart therefore produces the real
  transitions, not a flood.
- Container labels and env are **not** stored: labels are read only for the compose project name;
  `Config.Env` is never requested (that is where `POSTGRES_PASSWORD` lives). A code comment states this.

*Developer-environment collector*
- Environment variables: `HKCU\Environment` and
  `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` read via the P11 registry
  provider, woken by `RegNotifyChangeKeyValue`. Events `env/variable_added|removed|changed` with
  **values redacted by the P03 sanitizer** (env vars are the single most common secret store), and a
  dedicated `PATH` handling: `env/path_entry_added|removed` with one event per entry (not a diff of a
  2000-character string), because "something disappeared from PATH" is a classic silent breakage.
- Toolchain versions: a fixed allowlist of executable *names* (`node`, `bun`, `npm`, `pnpm`, `yarn`,
  `python`, `pip`, `java`, `dotnet`, `go`, `rustc`, `cargo`, `git`, `docker`, `kubectl`) resolved by
  scanning PATH directories for `<name>.exe`/`.cmd` — **no shell, no `PATHEXT` guessing beyond those two,
  no user-supplied names**. `.cmd` shims are detected with version `Unknown`: Windows requires a shell
  to execute them, which conflicts with the no-shell constraint. Executable versions are obtained by
  `std::process::Command` with the resolved absolute path,
  a single `--version` argument, `CREATE_NO_WINDOW`, stdin null, a 2 s timeout, and stdout truncated to
  256 bytes. Runs only (a) once at first start, (b) when a PATH entry changed, (c) every 6 hours.
  Events: `tool/version_changed` (`Node.js changed 20.11.0 → 22.6.0`), `tool/added`, `tool/removed`
  (resolved path disappeared).
- Git branch of watched folders: for each watched root (P10) that contains `.git`, read `.git/HEAD` on
  change and emit `dev/branch_switched` (`Switched branch in my-api: main → feature/auth`). Requires a
  narrow exception in the P10 ignore set for exactly `.git/HEAD` (the `.git/` directory stays ignored
  otherwise); implemented as an explicit allowlist entry, unit-tested so a future ignore-set edit cannot
  silently drop it.
- All three sub-sources live in one collector (`dev_env`) with one settings toggle, because a user
  thinks of them as one thing ("my dev setup"). Docker is a separate toggle.

**Non-functional**
- Docker event stream costs ~0 when idle (blocking read); the probe when absent is a single
  `GetFileAttributesW`-class check per 60 s.
- Toolchain scan ≤ 1.5 s total for 15 tools with the 2 s per-tool cap, runs off the fast tick
  (`cost_hint: Expensive`, gated by `QueryBusy`), and never on startup's critical path.
- Environment registry read ≤ 20 ms.
- Both collectors are absent-safe: no Docker, no git, and no toolchains present must all yield a
  `Running` collector emitting nothing, with zero error-level logs.

## Architecture

`collectors/docker.rs` (state machine: `Probing → Available{stream} → Reconnecting → Unavailable`),
`platform/windows/named_pipe.rs` (pipe open + HTTP/1.1 + chunked decoding),
`collectors/docker_protocol.rs` (pure parsing of the JSON event objects → `RawObservation`, fully
unit-testable), `collectors/dev_env.rs` (env vars + PATH + tools + git HEAD),
`platform/windows/tool_probe.rs` (PATH resolution + bounded process spawn),
`domain/tool.rs` (the tool allowlist as data). `DockerTransport` implementations:
`platform/windows/named_pipe.rs` (live) and `collectors/testing.rs::ScriptedTransport` (replays a captured
event fixture, including a mid-stream disconnect).

## Related Code Files

**Create** — `src-tauri/src/collectors/{docker.rs,docker_protocol.rs,dev_env.rs}`;
`src-tauri/src/platform/windows/{named_pipe.rs,tool_probe.rs}`; `src-tauri/src/domain/tool.rs`;
`src-tauri/tests/collector_docker.rs`, `src-tauri/tests/docker_protocol.rs`,
`src-tauri/tests/collector_dev_env.rs`, `src-tauri/tests/tool_probe.rs`;
`src-tauri/tests/fixtures/docker/{events-stream.ndjson,containers.json,images.json,
events-with-disconnect.ndjson,version.json}` (hand-written from the documented Docker Engine API event
shape, with a header comment stating they are protocol fixtures, not captures from a live daemon);
`src-tauri/tests/fixtures/dev_env/{path-before.txt,path-after.txt,env-before.json,env-after.json}`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/collectors/ignore_set.rs`
(`.git/HEAD` allowlist), `src-tauri/src/collectors/testing.rs` (`ScriptedTransport`),
`src-tauri/src/platform/traits.rs`, `src-tauri/src/domain/event.rs` (new event types),
`src-tauri/Cargo.toml` (`Win32_Storage_FileSystem` for the pipe),
`src/features/settings/SettingsScreen.svelte` (two new toggles with §49 copy),
`ROADMAP.md` (unverified Docker-live path), `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `docker_protocol.rs` first: parse the fixture NDJSON into `RawObservation`s with full tests. This is
   the part that can be verified without a daemon, so it is where the correctness lives.
2. `named_pipe.rs`: pipe open with `Drop`-guarded handle, minimal HTTP/1.1 request writer, response
   header parse, chunked-body decoder with a 1 MB per-chunk sanity cap, and a `stream()` returning a
   `BufReader` over the pipe. Version negotiation with the 400-fallback.
3. `collectors/docker.rs`: the four-state machine, baseline from `containers/json`+`images/json`,
   diff-on-resync, compose-project coalescing, relationship emission, backoff.
4. `tool_probe.rs`: PATH split (respect quotes and trailing `;`), per-directory existence check for
   `<name>.exe` then `<name>.cmd`, bounded spawn, version-string extraction by a per-tool regex in
   `domain/tool.rs` (e.g. `node` → `^v?(\d+\.\d+\.\d+)`), and a hard refusal to spawn anything whose
   resolved path is not one of the allowlisted file names.
5. `collectors/dev_env.rs`: env diff (values redacted), PATH entry diff, tool version diff, git HEAD
   watch; all four emit through the normal normalizer.
6. Settings toggles: "Docker — containers, images and volumes starting, stopping and being rebuilt"
   and "Developer environment — environment variables, PATH, installed tool versions, and the branch of
   folders you watch. Values are never stored."
7. Tests: protocol fixtures; a scripted-transport run including a mid-stream disconnect asserting
   resync produces no duplicates; absence path (`Unavailable`, no errors); env/PATH/tool/git diffs;
   `tool_probe.rs` asserting a non-allowlisted name is never spawned and that a hanging fake tool is
   killed at 2 s.
8. Manual (limited by D3): confirm `Unavailable` state and the exact settings copy render; confirm a
   real PATH change and a real branch switch each produce one event; record in the phase file that the
   Docker-present path was **not** exercised on this machine.
9. Commits: (a) `feat: docker event protocol parsing`, (b) `feat: named pipe http transport`,
   (c) `feat: docker collector state machine`, (d) `feat: environment and path collector`,
   (e) `feat: toolchain version probing`, (f) `feat: git branch watching`,
   (g) `test: docker protocol and dev env suites`.

## Todo (commit boundaries)

- [x] Docker protocol parsing with fixtures
- [x] Named-pipe HTTP transport with chunked decoding
- [x] Docker collector state machine + resync + compose coalescing
- [x] Env var / PATH entry collector with redaction
- [x] Bounded toolchain version probing
- [x] Git HEAD watching with the ignore-set exception
- [x] Test suites + absence-path verification

## Success Criteria

- [x] `docker_protocol.rs`: every fixture event maps to the expected event type, entity id, and
      importance; a `die` with `exitCode: 137` yields `docker/container_died`, `important`, code in details
- [x] Scripted transport with a mid-stream disconnect: resync emits events only for containers that
      actually changed (asserted count), and the collector returns to `Available`
- [x] Compose fixture: 6 containers of one project restarting within 30 s → 1
      `docker/compose_project_changed` plus per-container events at `informational`
- [x] No Docker present: collector reaches `Unavailable`, emits nothing, logs exactly one transition line
      in 5 minutes of running
- [x] `Config.Env` is never requested (`rg -n "Env" src-tauri/src/collectors/docker*.rs` → comment only)
- [x] Env fixture: `AWS_SECRET_ACCESS_KEY` changes → one event, both values `[REDACTED]`
- [x] PATH fixture: one entry removed, one added → exactly two events naming the entries
- [x] `tool_probe.rs`: a hanging fake tool is killed at 2 s and reported as `Unknown`, not an error; a
      name outside the allowlist is never spawned (asserted by a spawn-recording fake)
- [x] Branch switch in a watched repo → `Switched branch in my-api: main → feature/auth`
- [x] `ignore_set` test: `.git/HEAD` allowed, `.git/index` and `.git/objects/**` still ignored

## Verification evidence — 2026-09-22

`docker_protocol` 9/9, `collector_docker` 19/19, `collector_dev_env` 9/9,
`tool_probe` 8/8 and `soak_support` 1/1 passed with one serialized Rust test
process. Docker live-daemon behavior remains `Not-verifiable-here` on this
machine because Docker Desktop is not installed; the absence path and every
protocol/state transition are covered by deterministic fixtures.

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test docker_protocol --test collector_docker --test collector_dev_env --test tool_probe
cargo test --test collector_files --test redaction_leak
cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run check && bun run check:bindings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| D3: the Docker-present path is unverifiable here | Cannot be observed locally at all | Correctness is concentrated in the pure protocol parser (fixture-tested); the live path is a thin transport. The phase file and `ROADMAP.md` record "Docker live path unverified — needs a machine with Docker Desktop" instead of claiming it works. No test is written that pretends to exercise a daemon |
| Hand-rolled HTTP/chunked parsing is subtly wrong | Stream stalls or events are truncated after the first chunk | Chunk decoder is unit-tested against fixtures containing multi-chunk bodies, a chunk split across reads, and a trailing `0\r\n\r\n`. If it still misbehaves against a real daemon, the fallback is to poll `containers/json` every 3 s (cheap, already implemented for resync) and drop the stream — a config constant, not a rewrite |
| Docker API version mismatch on an older daemon | 400 with "client version too new" | Negotiation with a documented fallback version; the negotiated version is visible in diagnostics so a mismatch is diagnosable from a user report |
| Spawning tools is a code-execution surface | An unexpected process appears in P09's own event stream | Allowlisted file names only, absolute resolved paths, no shell, no user input, single fixed argument, `CREATE_NO_WINDOW`, 2 s timeout. The spawn-recording test is the guard. Self-events from these spawns are tagged and excluded from the timeline |
| A malicious PATH entry shadows `node.exe` | We execute an attacker binary | We only run tools the user's own shell would already run; the risk is not increased by observing. Mitigation: the resolved absolute path is stored in the event, so a shadowing entry is *visible* — which is the product's whole purpose |
| Env var redaction misses a value under an unusual name | Leak test on the env fixture fails | The fixture set includes benign-named high-entropy values; P03's entropy gate covers them, and any miss becomes a permanent fixture |
| `.git/HEAD` exception widens over time | Ignore-set test fails | The exception is a single exact-path allowlist entry with its own test; broadening it fails the test |

## Security Considerations

- Docker access is strictly read-only (`_ping`, `version`, `events`, `containers/json`, `images/json`);
  no endpoint that can create, start, stop, exec, or remove anything is ever called, and the named pipe is
  opened for read/write only because the protocol requires writing the request line.
- Container environment variables and secrets are never requested, so a compromised app cannot exfiltrate
  them from Docker.
- Environment-variable values are redacted before persistence; the settings copy states this so the user
  can trust the toggle.
- `PRIVACY.md` also forbids storing hashes, keyed digests, or comparison markers derived from redacted
  values. Live environment observations may compare values in memory; after restart, the redacted
  baseline proves additions/removals but cannot prove that an existing value changed while the app was
  closed. Do not manufacture such events or introduce a persisted fingerprint to recover that knowledge.
- Process spawning is the only place the app executes anything: allowlisted names, absolute paths, no
  shell, no user-controlled arguments — and no IPC command can trigger it (it is timer/PATH-driven only).

## Implementation evidence — 2026-09-20

Implementation is present; final acceptance remains open. Review fixes cover bounded Docker framing,
lossless queue backpressure, distinct-container compose grouping, atomic replay, reconnect deadlines,
named-pipe cancellation, environment redaction, watched-root Git tracking, and bounded executable probes.
The latest review is addressing partial reads across idle timeouts, trailing queue wakeups, project
mutation fencing and Unicode evidence paths. No production completion claim is made until those tests
and the integrated gates finish.

The attempted persisted DPAPI/HMAC comparison design was removed because it violates `PRIVACY.md`.
Restart tests must assert the permitted structural evidence and absence of persisted markers.
Tool stdout/stderr use bounded available-byte reads; an inherited writer must not retain reader threads.

The process performance gate remains open. Isolated attribution under current host load found CPU
88% and ToolHelp snapshot p95 32–57 ms before collector work; see
[diagnosis](../reports/debug-260920-process-budget-host-contention.md). The 15 ms threshold is unchanged.
Docker-present behavior still requires a Docker Desktop machine, as explicitly permitted by D3.
