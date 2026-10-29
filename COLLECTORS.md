# Collectors

Accepted observation contracts and non-elevated source limitations. Rollout and acceptance evidence
belong to [the implementation plan](plans/260829-0437-what-changed-desktop-mvp/plan.md). Read-only
access, no elevation and absence safety are product constraints on every source.

Ten sources are implemented and active: `process`, `listeners`, `files`, `services`, `applications`,
`updates`, `drivers`, `network`, `startup`, and `eventlog`. Docker and development-environment entries
remain visible but unavailable until their implementation phases, so the catalog is never mistaken for
coverage.

## The contract

The [collector trait](src-tauri/src/collectors/mod.rs),
[stateful snapshot contract](src-tauri/src/collectors/snapshot.rs) and
[descriptor types](src-tauri/src/domain/collector.rs) own the API. Descriptor copy belongs in the
[canonical registry](src-tauri/src/collectors/registry.rs), so product surfaces cannot acquire competing
descriptions of the same source. [Runtime boundaries](ARCHITECTURE.md#collector-runtime) explain the
isolation, durability and scheduling decisions, and
[the platform boundary](ARCHITECTURE.md#the-platform-boundary) explains how an operating-system call
reaches a collector without making it untestable.

## Inventory

This table routes the accepted source coverage. The [registry](src-tauri/src/collectors/registry.rs)
owns current descriptor values; [the scheduler](src-tauri/src/collectors/schedule.rs) owns profile
scaling and expensive-work policy. A catalog entry is not evidence that its platform factory is active —
the **Active** column is.

| Collector | Watches | Mechanism | Cadence | Cost | Active |
|---|---|---|---|---|---|
| `process` | Programs starting and stopping | Process snapshot diff | 1 s poll | Cheap | yes |
| `listeners` | TCP and UDP listening ports | Extended TCP/UDP tables | 2 s poll | Cheap | yes |
| `files` | Config files in folders you choose | Filesystem watcher, debounced | Notify | Moderate | yes |
| `services` | Windows services starting, stopping, failing | Service enumeration | 5 s poll | Cheap | yes |
| `applications` | Installed and updated programs | Uninstall registry hives | Notify + sweep | Moderate | yes |
| `updates` | Windows Update installs and failures | Update Agent history | 15 min | Cheap | yes |
| `drivers` | Device drivers added, removed, version-changed | Device enumeration | Notify + 6 h sweep | Expensive | yes |
| `network` | Adapters, addresses, routes, DNS, proxy | Address and route tables, proxy keys | Notify + 30 min sweep | Expensive | yes |
| `startup` | Startup programs and logon tasks | Run keys, Startup folders, Task Scheduler | Notify + sweep | Expensive | yes |
| `eventlog` | Crashes, service failures, unexpected shutdowns | Event log subscriptions on four channels | Notify | Cheap | yes |
| `docker` | Containers, images, volumes, networks | Docker engine named pipe, read-only | Stream | Cheap | no |
| `dev_env` | Environment variables, PATH, tool versions, git branch | Registry, PATH probing, `.git/HEAD` | Notify + 6 h | Expensive | no |

## Processes

Windows exposes process *start* events cheaply only to an elevated listener. Both elevated routes were
tested on the development machine and rejected: the process-start trace class returns access denied to a
plain user token, and the WMI instance-creation subscription registered successfully but captured zero
events. So this collector diffs a full process snapshot once per second.

The consequence is stated in the product itself, in the collector's own description:
`Programs starting and stopping (checked every second — programs that live less than a second may be
missed).` A build script, a short-lived `npx` invocation, or a crashing helper can slip through the gap.
That is a real blind spot and it is not papered over.

Processes are keyed by pid **plus** creation time, never pid alone, because Windows reuses pids and a
recycled pid would otherwise merge two unrelated programs into one entity. Where a handle cannot be
opened there is no creation time to key on, and the image name decides continuity instead — a weaker
answer, which the event reports as lower confidence rather than presenting as the same certainty. A
reused pid running the *same* executable is still a restart: creation time is authoritative whenever both
sides have one, and a stop and a start are recorded in that order. Both halves of a reuse commit as one
transition, so a crash between them cannot leave behind a program that stopped and never came back.

About 46% of running processes deny an executable-path read to a non-elevated caller — 197 of 368
processes exposed a readable path when measured. Those events are recorded with a null path, an explicit
flag saying the path was unavailable, and low identity confidence. They are not guessed from the window
title or the image name, and they are not shown as "unknown": the recorder knows what the program is
called even when it cannot see where it lives. Nothing is logged per denial, because that would be dozens
of lines a second saying nothing new — one aggregate count an hour, at DEBUG.

An exit is classified as normal, unexpected, or unknown, and `unexpected` requires corroboration from the
event log within a few seconds. The storage writer reconciles either arrival order, so a crash record that
lands just after the process stop still enriches that stop. Without matching evidence the value remains
`unknown`: a diff-poll can observe that a pid is gone and cannot observe why.

A program that starts and stops more than five times in a sixty-second episode earns one churn summary.
The summary is *appended beside* the raw events, never substituted for them — the individual starts and
stops are already written and already visible, and rewriting history the user has been shown is not what
"collapse" can mean. The window is anchored at the episode's first completed pair rather than rolling, it
closes before its count is published, and its timestamp is the window's close rather than pretending to
be one more process start. Only a stop whose start this recorder also observed counts as a pair, so a
program that was already running cannot open an episode on the strength of an exit nobody watched begin.

A browser spawning its own image — `chrome.exe` from `chrome.exe` — is recorded at `informational` and
excluded from notifications. The rule is deliberately narrower than "anything a browser spawns": a crash
handler or a GPU helper with its own executable name is a distinct program and stays undamped, because
guessing at helper naming is how a recorder starts hiding the thing someone is looking for.

Command lines are out of scope for this release. They are the single richest source of accidental secrets
(tokens passed as arguments) and the value does not justify the risk yet.

Implementation: [the collector](src-tauri/src/collectors/process/mod.rs), its
[diff rules](src-tauri/src/collectors/process/diff.rs), its
[churn window](src-tauri/src/collectors/process/churn.rs), the
[identities it persists](src-tauri/src/collectors/process/state.rs), and the
[Win32 enumeration](src-tauri/src/platform/windows/process.rs) behind it.
Executable acceptance lives in [the process collector suite](src-tauri/tests/collector_process.rs) and
[the real-cycle performance gate](src-tauri/tests/collector_performance.rs).

## Listening ports

Full TCP and UDP tables for both IP versions, polled every two seconds, filtered to listeners. Port
numbers are byte-swapped from network order — the classic bug in this API, called out explicitly in the
plan and pinned by a test written from a fixed byte pattern, because a swapped port is still a perfectly
plausible port and so fails quietly. A link-local IPv6 endpoint keeps its interface scope in its
identity; without it two adapters both carrying `fe80::1:4000` would merge into one endpoint, and the
next diff would report a phantom owner change on a socket nobody touched.

Relationships are emitted in both directions so the analysis engine can connect "this process restarted"
to "port 5432 stopped listening" from either end.

An endpoint is keyed by protocol, address and port, and its owner set rides along as the value. So a port
that is still bound but has changed hands is **one modification**, not a close and an open that nothing
corroborates — inventing that pair would put two events in the timeline for a socket that never stopped
listening. A close, by contrast, carries no owner claim at all: the process may already be gone, and the
edge written when the port opened still stands on its own evidence.

Owners are resolved through [the shared process map](src-tauri/src/collectors/process_map.rs), never by a
second enumeration. The map is session-local, it expires, and it stops answering when its publisher is
cancelled or paused, so an inactive process collector yields unresolved owners and no edges rather than a
stale attribution. A port whose owner cannot be named still records — it really is listening — at low
confidence, with the gap named in its details so the row cannot be read as a port with no owner.

Measured on the development machine: 37 TCP listeners, 67 UDP endpoints.

Implementation: [the collector](src-tauri/src/collectors/listeners/mod.rs), its
[endpoint diff](src-tauri/src/collectors/listeners/diff.rs), the
[events and graph evidence](src-tauri/src/collectors/listeners/observations.rs) it produces, the
[table reads](src-tauri/src/platform/windows/sockets.rs) and the
[byte-order conversions](src-tauri/src/platform/windows/socket_rows.rs) behind them.
Executable acceptance lives in [the listener collector suite](src-tauri/tests/collector_listeners.rs),
[the relationship suite](src-tauri/tests/relationships_process_port.rs) and the shared
[real-cycle performance gate](src-tauri/tests/collector_performance.rs).

## Files and config

The highest privacy risk in the product, and therefore the most constrained collector. It watches only
folders you explicitly add. Each root is canonicalised, must be a directory, and is rejected if it is a
reparse point or junction, if it lives under the Windows directory, either Program Files tree, the
Microsoft ProgramData tree, another user's profile, or the application's own data directory. There is a
cap on the number of roots and a depth cap inside each one.

An ignore set keeps build noise out: version-control internals, `node_modules`, `target`, `dist`, `build`,
framework caches, virtual environments, vendored directories, log and temporary files, editor swap files,
and anything above a couple of megabytes. Lockfiles are ignored except the three that mean something to a
developer. The plan's acceptance test is that a routine dependency install inside a watched folder produces
at most three events.

For structured formats — `.env`, JSON, YAML, TOML, INI, `.properties`, XML — the collector produces one
event per provably changed key rather than one event per file touch, capped with an "and N more keys
changed" summary. To do that it compares against a **sanitized shadow copy**: key names and non-sensitive
evidence values are retained, while every recognised secret-bearing value is the literal `[REDACTED]`.
Two retained redacted values are deliberately indistinguishable and produce only a file-level uncertainty
notice. File content is read only long enough to parse, and the source buffer is zeroised afterwards.

The UI copy for this collector is a promise: `We record what changed, never the secret values.`

The collector also records **which entity a config file belongs to**, because a config change that is not
attached to anything can only ever rank as a coincidence. The accepted design permits three kinds of
evidence and no others: the watched root is a registered project whose process was seen running from that
directory; a port-shaped key's value matches a port currently recorded as listening; or a service's binary
path lives under the watched root. Phase 10 implements the live-port path; the service and registered-project
paths are completed only when Phases 11 and 18 provide those evidence owners. Each implemented edge carries
the id of the event that justified it, so it is inspectable in the System Map and citable in an explanation.
A file that matches no available evidence gets no edge and ranks on category and timing alone — the honest
answer rather than a guessed connection.
This edge is what lets the analysis engine reach its strongest confidence class for the case the product
exists to solve: a port changed in a `.env`, the service restarted, and the old port stopped listening.

## Services

The service control manager is opened with enumerate-only rights and each service queried with
config-read rights. Nothing in this collector can start, stop, or reconfigure a service — the handles it
asks for cannot do it. 301 services were enumerated on the development machine, 299 of them exposing a
readable binary path.

Enumeration is cheap enough that the collector simply polls every five seconds. The push-notification API
for service status is treated as an optimisation to be attempted, never as a requirement, because a poll
that is already fast is a poll that cannot break.

## Installed applications

Three uninstall registry hives, woken by a registry change notification with a long-timeout wait acting as
its own fallback poll. Measured hive sizes on the development machine: 88, 148, and 38 keys, scanned in
roughly 50 ms. Entries flagged as system components or lacking a display name are skipped, because they are
not applications a user recognises.

Version changes read as a sentence: `Docker Desktop updated 4.35.1 -> 4.36.0`.

The baseline is **persisted**, not held in memory. That distinction is the difference
between a recorder and a recorder that lies about its own downtime: if the app is
closed when an installer runs, the first cycle after the next start diffs against the
stored set and reports the change, instead of silently adopting the new state as
normal. A restart with nothing actually changed diffs to zero events, so the storm
control that an in-memory baseline was protecting is preserved either way.

Changes found this way carry an explicit uncertainty marker, because they were
*detected* at startup but *occurred* at an unknown point since the last shutdown. The
UI says so rather than stamping them with the moment they were noticed — a fabricated
timestamp would corrupt exactly the signal the analysis engine leans on hardest.

Where an application clearly owns a service, a relationship is inferred, which is what lets an
investigation connect "this program updated" to "that service failed".

## Windows Update

Read from the Windows Update Agent's own history through COM on a dedicated apartment-threaded thread, with
no COM pointer escaping that thread. Queried every 15 minutes and on resume from sleep. The obvious
alternative — the WMI quick-fix-engineering class — is **forbidden** in this codebase and carries a code
comment saying why: on the development machine it returned 4 rows where the Update Agent returned 133. It
is not a smaller view of the same data, it is the wrong data.

Copy shape: `Windows Update installed KB5034123` and
`Windows Update failed: KB5034123 (error 0x80070002)`. A failed update is classified important, because a
failed update is the single most common answer to "what changed last night".

## Drivers

The measured device-enumeration cost — about 2.1 seconds for 215 driver rows on the development machine —
is why the [registry](src-tauri/src/collectors/registry.rs) classifies this source as `Expensive`.
[Runtime scheduling](ARCHITECTURE.md#collector-runtime) owns the foreground-query gate; it is not a
machine-load detector. Notification fallback, provider time limits and negotiated-mode reporting remain
the [driver collector's accepted target](plans/260829-0437-what-changed-desktop-mvp/phase-12-update-driver-collectors.md).

Names are humanised from the device metadata: `NVIDIA graphics driver updated 566.36 -> 566.45`. A driver
update is classified important — it is one of the few change classes that reliably breaks working software
overnight.

The collector persists its own baseline, so restarting the application diffs against
the stored driver set rather than replaying every driver on the machine as new — and,
more importantly, a driver that rolled forward while the app was closed is still
reported, flagged with an uncertain occurrence time.

## Network

Adapter and address enumeration plus the route table, plus the per-user proxy configuration keys. Woken by
interface and route change notifications and a registry notification for the proxy keys, with a 30-minute
sweep as the floor. Full sweep measured at about 1.44 s on the development machine, with a budget above it.

Proxy and DNS changes are classified important. They are also the two settings most likely to silently
break every network-dependent tool a developer owns while leaving the machine apparently healthy.

Known gap: the machine-wide WinHTTP proxy is not read. Only the per-user configuration is.

## Startup

Four Run-style registry keys, both Startup folders, the enable/disable approval keys, and the Task Scheduler
through COM. On the development machine: 194 tasks of which 28 are logon-triggered, 17 Run values, and 6
Startup folder items, enumerated in about 1.33 s. Something new starting with Windows is a change worth
recording; something quietly disabled is worth recording too.

## Event log

Subscriptions on exactly four channels: `Application`, `System`, the Windows Update client operational
channel, and `Setup`. Each subscription carries an XPath filter for a named set of event ids — application
crashes and hangs, service start failures and unexpected terminations, unexpected shutdowns, dirty
shutdowns, and the shutdown-initiated record. It is not a log tailer; it is a small set of specific
questions.

The collector subscribes to future events and does not replay ordinary records written while the app was
closed. One narrow exception is the newest System 6005 record, queried after subscriptions are installed
to establish the current boot boundary for “Since last restart.” This does not claim downtime coverage
for crashes, service failures, updates, or setup events.

Identical `(provider, event id, first Data value)` records inside a 60-second window update the first
stored row's aggregate count. Process exits and Windows Update rows cite matching persisted log evidence;
absence remains absence, never a guessed cause.

**The `Security` channel is not subscribed.** It was measured as unauthorised for a non-elevated reader on
the development machine. The code carries a comment saying so, so that nobody re-adds it, discovers it
fails, and concludes the application needs to run as administrator.

Near-identical entries collapse within a minute so a service failing in a retry loop is one event, not
sixty. This collector also serves the analysis engine: when an investigation needs to know whether a
process exit was a crash, the corroborating log entry is what answers it. Restart boundaries used by the
"since last restart" range are re-derived from boot events rather than from the application's own uptime.

## Docker

Read-only over the engine's named pipe, speaking HTTP/1.1 directly rather than adding an HTTP client and a
pipe-transport crate for a handful of endpoints. Exactly five endpoints are ever called: ping, version, the
event stream, the container list, and the image list. **No endpoint that can create, start, stop, exec, or
remove anything is called.** The pipe is opened for read and write only because the protocol requires
writing a request line.

Container environment variables are never requested. That is where database passwords live, and the code
says so at the call site. Labels are read for exactly one purpose: the compose project name, which lets six
containers of one project restarting become one event instead of ten.

Containers are keyed by name rather than id, because a name survives a rebuild and an id does not — and
"my compose service was rebuilt" is precisely the change a developer needs to see.

When the pipe is absent the collector is `Unavailable` and says
`Docker isn't running. We'll start recording container changes when it does.` It re-probes every minute and
logs only on transition. If the stream drops it resyncs from the container and image lists, diffing against
its last known state so a daemon restart produces real transitions instead of a flood.

**Docker Desktop is not installed on the development machine.** The protocol parser is fixture-tested and
the absence, unavailability, and reconnection paths are exercised, but the live daemon path has not been run
against a real daemon. It is recorded as an unverified surface in [ROADMAP.md](ROADMAP.md) rather than
claimed as working.

## Developer environment

Three sub-sources behind one toggle, because a user thinks of them as one thing:

- **Environment variables** from the per-user and machine environment keys, woken by registry notification.
  Values are redacted before storage — environment variables are the most common secret store on a
  developer machine. `PATH` is special-cased: one event per added or removed entry, because "something
  disappeared from PATH" is a classic silent breakage and a diff of a 2000-character string tells a user
  nothing.
- **Toolchain versions** for a fixed allowlist of 15 executable names, resolved by scanning PATH
  directories for that exact filename. The resolved absolute path is spawned with a single `--version`
  argument, no shell, no window, a short timeout, and truncated output. Nothing outside the allowlist can
  be spawned, and a spawn-recording test is the guard. A shadowing PATH entry becomes *visible* rather than
  dangerous, because the resolved path is stored in the event — which is the whole point of the product.
- **Git branch** of watched folders, read from `.git/HEAD`. The version-control directory is otherwise
  ignored; this is a single exact-path exception with its own test, so a future edit to the ignore set
  cannot silently drop it. `Switched branch in my-api: main -> feature/auth`.

## Measured cost

Every number below was measured on the development machine under a non-elevated token and is recorded in
[research/verified-facts-report.md](plans/260829-0437-what-changed-desktop-mvp/research/verified-facts-report.md).
They were taken through PowerShell, so they are **upper bounds**: the Rust implementations are expected to be
faster, and the plan uses these figures as budget ceilings rather than as expected values.

| Operation | Observed | Scale on that machine |
|---|---|---|
| Process enumeration | 6.2 ms warm (46.1 ms first call) | 368 processes, 197 with a readable path |
| Service enumeration | 3.3 ms warm (15.2 ms first call) | 301 services |
| Service config read (all) | 392.6 ms | 299 readable binary paths |
| TCP listener table | 204.0 ms warm (636.2 ms first call) | 37 listeners |
| UDP endpoint table | 104.5 ms | 67 endpoints |
| Uninstall hive scan | about 50 ms | 88 + 148 + 38 keys |
| Windows Update history | 38.2 ms | 133 entries |
| Driver enumeration | 2128.0 ms warm (2268.7 ms first call) | 215 rows, 214 present devices |
| Network configuration sweep | 1440.7 ms | 5 adapters |
| Scheduled task enumeration | 1325.7 ms | 194 tasks, 28 logon-triggered |

The sum of the full sweeps is what a checkpoint costs — roughly five seconds — which is why checkpointing
is an explicit user action with a progress state rather than something that happens silently.

## Privilege reality

Of 35 capabilities probed under a plain `BUILTIN\Users` token, **31 worked and 4 did
not** — and the four fail for three different reasons, which matters because only two
of them are about privilege at all:

| Capability | Result | Consequence |
|---|---|---|
| Process-start trace class | Access denied — needs elevation | Process detection is a 1 s diff poll; sub-second processes can be missed |
| `Security` event log channel | Unauthorised — needs elevation | Not subscribed; no security-audit-derived events exist |
| WMI instance-creation subscription | Permitted, but delivered **0 events in 3 s** while a short-lived process ran and exited | Rejected on reliability, not privilege. The same diff poll covers it |
| Docker named pipe | Absent — Docker is not installed on this machine | Only the no-Docker path is verified; the connected path ships untested here |

A fifth is worth naming even though it passed: the WMI quick-fix-engineering class
reads without elevation but returned 4 rows where the Update Agent returned 133. It is
permitted and useless, which is a more dangerous category than denied.

Neither elevation gap is solved, neither is being worked around, and elevation is not
on the table — the application's promise is that it never needs administrator, and that
promise is worth more than a sub-second process window.

## Blind spots, in one place

| Gap | Why it exists |
|---|---|
| Processes living under one second | The push API for process starts requires elevation |
| Security-audit events | The channel is unauthorised without elevation |
| Executable path for roughly 46% of processes | A non-elevated token cannot read it; recorded as low confidence, never guessed |
| Process command lines | Deliberately out of scope: highest accidental-secret risk, lowest current value |
| Machine-wide WinHTTP proxy | Only the per-user proxy configuration is read |
| The live Docker path | Docker Desktop is not installed on the development machine |
| XML config diffing | Structurally harder than the other formats; degradation is recorded rather than hidden |
| Exact timing of anything that changed while the app was closed | Persistent state (services, applications, drivers, updates, network config, startup entries) is diffed against a stored baseline on the next start, so the change *is* reported — but the moment it happened is unknowable and is marked uncertain rather than guessed. Processes and listening ports are diffed against a stored baseline too, for the same reason and carrying the same marker: without one, resuming from a pause would either invent a start for every program already running or silently swallow everything that changed during it. What is *not* claimed for them is the downtime-detection promise above, which names persistent state only — a program that came and went entirely inside a gap leaves no current state to diff, and so is not reported at all |
| A socket attributed to a recycled pid | The listener collector reads the socket table and then resolves owners against the process map. Those are two reads, not one atomic kernel join, and the socket table carries no creation time to check against. If a pid was recycled between them the port is attributed to the wrong program. Low identity confidence narrows this and cannot remove it; joining in the kernel would need the elevation this app never asks for |
| Event log records written while the app was closed | The event-log collector subscribes to *future* events rather than reading history, so records written during downtime are not recovered. This is the one downtime gap a persisted baseline cannot close, because there is no "current state" to diff — an event either arrived while we were listening or it did not. Named here rather than left to be discovered as a missing crash report |

## Absence safety

The rule every collector obeys: no Docker, no git, no toolchains, no watched folders, and no unusual
hardware must all produce a running collector that emits nothing and logs nothing repeatedly. A missing
dependency is a state with copy attached, never an error banner and never a retry loop in the log. When any
collector is genuinely failed or unavailable, the product says
`Some things aren't being watched right now.` once, non-modally, with a link to Diagnostics.
