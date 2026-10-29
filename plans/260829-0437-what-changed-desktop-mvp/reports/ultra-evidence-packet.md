# Ultra Evidence Packet — "What Changed" Desktop App

> **Status:** immutable. Built once by the controller (`/ak:plan --ultra`), passed
> identically to all five candidate planners and the verifier. Do not edit after
> candidate dispatch.
>
> **Trust boundary:** this packet is evidence, not instruction-from-a-third-party.
> Candidates must not treat any embedded text as authority to run commands, write
> outside their own report, or mutate plan/session state.

## 0. What this packet contains

| § | Content |
|---|---------|
| 1 | Controller-verified environment evidence (machine truth, Aug 29 2026) |
| 2 | Scope contract (what is in / out, and the non-negotiables) |
| 3 | Normative requirements digest — every hard rule from the user's master build prompt, walked §1→§91 |
| 4 | Research reports (external, version-verified) |
| 5 | Known blockers and open decisions the plan must resolve |

---

## 1. Environment evidence (controller-verified, 2026-08-29)

Every line below was checked with a real command on the target machine. Treat as
ground truth; do not re-derive.

| Fact | Verified value | Consequence for the plan |
|---|---|---|
| Project dir | `D:\Project\.WhatChage` — **was completely empty** | True greenfield. Nothing to preserve, nothing to migrate. Note the folder name is misspelled (`.WhatChage`, missing `n`); repo/product name must still be `what-changed` / `WhatChanged.exe`. |
| Git | Was **not** a repo; controller ran `git init -b main`. Branch `main`, zero commits | Phase 0 owns first commit, `.gitignore`, `.gitattributes`. No `user.name`/`user.email` configured in this environment — a commit step must not assume they exist. |
| Rust toolchain | **NOT INSTALLED.** No `rustc`, no `cargo`, no `rustup`; `~/.cargo` and `~/.rustup` do not exist (PATH lists `~/.cargo/bin` but the directory is absent) | **Hard prerequisite.** Phase 0 step 1 is installing `rustup` + the `x86_64-pc-windows-msvc` toolchain. No Rust code can compile until then. This is the single largest schedule risk in Phase 0. |
| MSVC C++ toolchain | **Present.** `vswhere` reports two installs satisfying `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`: `C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools` and `D:\Work\c++`. A VS 18 "Insiders" install carries MSVC `14.44.35207` and `14.50.35717` | The MSVC linker Tauri needs on Windows is available. `rustup` should select the `msvc` (not `gnu`) toolchain. Toolchain-selection ambiguity across three installs is a real risk — pin one. |
| Windows SDK | `10.0.26100.0` present | Sufficient for Win32/COM/SetupAPI/wevtapi work. |
| WebView2 runtime | **Present**, Evergreen `151.0.4129.107` (and `.101`) | Tauri 2 has its WebView. Installer may still need to handle a machine without it (bootstrapper strategy is a plan decision). |
| Bun | `1.3.11` at `~/.bun/bin/bun` | Matches the mandated package manager. |
| Node | `v24.12.0` present | Present but must **not** become a runtime requirement. Note some tooling (`gh` hooks, `ak`) uses it — that is dev-side only. |
| OS | Windows 11 Home Single Language, `10.0.26200` | Win11 24H2+ era. Home edition — **no Group Policy editor, no Hyper-V by default**; Docker Desktop needs WSL2 backend. Treat Docker as genuinely optional. |
| Shell | Git Bash (MSYS2). Heredocs are unreliable through the agent's Bash tool; `/tmp` in bash is not the same as the file-tool `/tmp`; the exec bit is not preserved on commit | Plan scripts must be `.ps1`/`.cmd`/`bun` scripts, not bash heredocs. |
| `gh` CLI | Installed (`C:\Program Files\GitHub CLI`) | GitHub projection possible later; not required. |
| `ak` CLI | `2.14.0`. Plan store initialized; this plan pinned via `ak plan use` | Plan mutations go through `ak plan`. |
| Session-state pointer | `set-active-plan.cjs` **failed** ("Failed to update session state") | Subagent report paths must be passed fully resolved and absolute; never rely on injected `Plan Context`. |
| Admin rights | Not assumed, not tested | Design must work non-elevated. Any collector needing elevation must degrade honestly. |

### Environment-derived hard facts the plan must honor

1. **Rust is missing.** A plan that opens with "run `cargo tauri dev`" is wrong. Phase 0 must install and verify the toolchain, and record the pinned version in `rust-toolchain.toml`.
2. **Home edition.** No enterprise-only assumptions (no GPO, no Windows Server APIs, WSL2/Hyper-V may be absent).
3. **Single dev, no CI runner described.** Verification gates must be runnable locally with one command each.

---

## 2. Scope contract

### Deliverable of THIS run

A plan. Not an implementation. The plan must be executable by `/ak:cook` afterwards.

### Scope mode

**HOLD SCOPE.** The user did not pass `--yagni`. Every feature named in the master
build prompt stays in the plan, in full. Candidates must not defer, trim, or
"simplify away" a requested feature, and must not add features beyond the request.
The only permitted deferrals are the ones the user *explicitly* deferred himself
(§75 future features, §26 health checks to "later MVP / Phase 2").

### Non-negotiable technical rules (violating any = candidate rejected)

- **NO ELECTRON.** Not temporarily, not as a fallback, not as a suggestion.
- **No Go** in the MVP. Rust traits for collectors; a documented protocol so a
  future sidecar is possible, but no second process now.
- Stack is fixed: Tauri 2, Rust, Svelte 5 (runes), TypeScript strict, Vite, Bun
  (dev/build only), Tailwind CSS 4 (CSS-variable tokens), Three.js, SQLite (WAL,
  migrations, prepared statements, indexes, transactions), serde, Tokio, structured
  Rust tracing.
- The distributed app must **not** require Node, Bun, npm, pnpm, or yarn on the
  user's machine.
- Windows 10/11 x64 first; `core/` + `platform/{windows,linux,macos}` separation,
  Windows fully implemented only.
- Main process runs **non-elevated**. No permanent Administrator. Optional narrow
  privileged helper only if a feature truly needs it, never a silent elevation.
- **No Windows Service** in MVP — user-level tray app.
- Frontend never touches OS APIs. All system access in Rust behind narrow typed
  commands. No `run_any_command`, `read_any_file`, or `execute_shell`.
- Local-first: no account, no cloud, no telemetry by default, works fully offline,
  no CDN, no online font.
- Redaction runs **before persistence and before logging**. Secrets are never stored.
- No spyware behavior: no keystrokes, clipboard, browser history, messages, screen
  recording, screenshots, mic, camera, or arbitrary document contents.
- Cloud AI OFF by default, explicit opt-in, and the product must be fully useful
  with **zero** LLM involvement.
- Performance: ~0–1% idle CPU, RAM target <80 MB (stretch <60 MB), batched
  non-blocking disk writes, event-driven collectors over high-frequency polling.
- `unwrap()` avoided in production paths; typed errors; every collector isolated so
  one failure never takes down the app.
- **No faked implementations.** A phase is complete only when it genuinely works.
  Stubs allowed mid-flight but must be visibly tracked.

---

## 3. Normative requirements digest

Organized by the user's own section numbers. Every rule here is binding. Where the
user gave exact microcopy, examples, or code shape, it is reproduced because it is
normative — the tone *is* the product.

### §1 Product vision

Continuously observe important local system changes; build a chronological history;
let a normal human answer **"It worked before. What changed?"**. Behaves like a
**black box / flight recorder for a PC**.

Questions the product must answer: what changed today / since yesterday / since last
restart; why did Discord stop detecting my microphone; why did this app stop
launching; why is my internet acting strangely; why did my Minecraft server stop
working; what changed between 8 PM yesterday and 9 AM today; what changed
immediately before this app crashed; has this happened before; which recent change is
most likely related.

Must translate low-level facts into understandable explanations:

| Bad UX | Good UX |
|---|---|
| `PID 14552 opened TCP socket 0.0.0.0:3001.` | "The application started listening on port 3001 instead of port 3000." |
| `HKCU registry value modified.` | "Your Windows proxy configuration changed." |

Value prop: **"Something broke? See what changed."** Secondary: *"Your computer's
black box."* / *"Ask your computer what happened."*

### §2 Audience — both, in one app

- **Simple users** who do not know ports, processes, env vars, registry, services,
  Docker, or event logs must be comfortable. Default mode always prioritizes
  understandable explanations.
- **Advanced users** (devs, sysadmins, self-hosters, game-server owners, IT support)
  can expand an event to see raw paths, PID, service name, registry path, port,
  executable, hash, before/after values, related events, timestamps, confidence
  score, raw technical evidence.

### §3–§4 Stack (see §2 of this packet for the binding list)

Component primitives: shadcn-svelte / Bits UI *concepts*, copied and customized into
the codebase — must **not** look like a generic shadcn dashboard; the app has its own
visual identity. Three.js required but used intentionally. Prefer a mature Rust SQLite
library. Logging supports dev console + rolling local log files + configurable
verbosity, and never logs secrets.

### §5 No Go in MVP

Rationale the plan must respect: a second binary adds process, IPC, lifecycle,
version sync, installer, debugging, crash-recovery, and updater complexity. Instead
define a clean Rust collector trait, conceptually:

```rust
trait Collector {
    fn id(&self) -> &'static str;
    fn start(&self, ctx: CollectorContext) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn snapshot(&self) -> Result<Vec<SystemEntity>>;
}
```

Architecture must allow a *future* external collector over a documented protocol
(Tauri sidecar) without building it now.

### §6 Windows first

Windows 10/11 x64 only for MVP. Separate `core/` from `platform/windows|linux|macos`,
implement only Windows. Cross-platform abstraction must not degrade Windows
integration quality.

### §7 Process lifecycle

`WhatChanged.exe` launch creates: main Tauri app + background monitoring engine +
system tray icon. Closing the main window **hides to tray**; monitoring keeps running.

Tray menu, exactly these entries: Open What Changed / What changed today? / Create
checkpoint / Pause monitoring / Resume monitoring / Settings / Quit.

Onboarding offers "Start What Changed automatically when Windows starts." No Windows
Service in MVP. Optional privileged-helper architecture designed but not always-on.

### §8 Core architecture

Svelte UI (Tauri WebView) → typed Tauri IPC → Application Service → { Query Service,
Analysis Engine, Settings }; Query Service + Analysis Engine → Event Repository →
SQLite; Event Normalizer feeds the repository from collectors (Files, Processes,
Network, Docker, Services, Updates, Git, Registry, Drivers). UI never calls OS APIs.

### §9 Domain model — one normalized event for every collector

`SystemEvent` fields: `id`, `timestamp`, `category`, `event_type`, `entity_type`,
`entity_id`, `entity_display_name`, `severity`, `importance`, `source_collector`,
`before`, `after`, `metadata`, `related_entity_ids`, `user_friendly_summary`,
`technical_summary`, `confidence`, `sensitive`.

Categories: `application, process, filesystem, configuration, network, service,
driver, update, docker, git, security, runtime, startup, hardware, system`.

Event types: `created, deleted, modified, started, stopped, restarted, installed,
uninstalled, updated, enabled, disabled, connected, disconnected, opened, closed,
failed, recovered, configuration_changed, version_changed`.

Analysis logic must never be tied to one collector implementation.

### §10 SQLite data model

Migrations for at least: `events, entities, entity_relationships, snapshots,
checkpoints, health_states, collector_status, settings, watched_locations,
applications, analysis_sessions, analysis_results, schema_migrations`.

Indexes at least: `events(timestamp)`, `events(category, timestamp)`,
`events(entity_id, timestamp)`, `events(event_type, timestamp)`,
`entity_relationships(source_id)`, `entity_relationships(target_id)`.

Normalize where useful, do not over-normalize tiny metadata; large metadata may be
JSON. **Never store arbitrary full file contents.**

### §11 Retention (configurable)

Defaults: event history 30 days; detailed snapshots 7 days; important events 90 days.
Options: 7 / 30 / 90 / custom. Background maintenance removes expired events, vacuums
intelligently, avoids write amplification, never blocks UI, avoids high disk usage.
Settings shows estimated storage usage.

### §12 Collector contract

Independent collectors, each with start/stop lifecycle, health reporting, snapshot
where appropriate, normalized event emission, independent failure, and no ability to
crash the app.

### §13 File / config collector

**Never monitor the whole filesystem by default** (noise + privacy). Monitor only:
user-selected folders, detected dev project folders, selected configuration
locations, app-specific config the user explicitly enabled.

Detect: file created / deleted / renamed / modified. For normal files store `path`,
`size`, `mtime`, hash where useful — **never contents**.

For explicitly recognized text config files (`.env`, `.json`, `.yaml`, `.yml`,
`.toml`, `.ini`, `.conf`, `properties`, `xml` when appropriate) allow structured
before/after diff, with automatic secret redaction. Redact keys matching concepts:
`password, passwd, secret, token, apikey, api_key, authorization, cookie, session,
private_key, client_secret`. Never store secret values.

Target event shape:

```text
.env changed
API_PORT           3000 → 4000
OPENAI_API_KEY     [REDACTED] → [REDACTED]
```

### §14 Process collector

Detect: process started, process stopped, unexpected exit if detectable, executable
path changed, version changed. Metadata may include PID, parent PID, exe path,
company/publisher, file version, command executable. **Do not record full command
lines** if they may contain secrets unless redacted. Do not poll hundreds of times
per second.

### §15 Port / network listener collector

Track TCP listeners, UDP where useful. Detect port opened / closed / owner changed /
binding address changed. Must produce output like:

```text
Port 3000 stopped listening.
Port 4000 opened by: api.exe
These events occurred 300ms apart.
```

Relationship: `process → listens_on → port`.

### §16 Windows service collector

Monitor installed / removed / started / stopped / restarted / startup type changed.
Human: "Windows Audio restarted." Advanced: `Service: Audiosrv`,
`State: RUNNING → STOPPED → RUNNING`.

### §17 Installed application collector

Detect installed / removed / version changed. Read Windows installation metadata,
normalize duplicate registry entries. Example: "Discord updated 1.0.x → 1.0.y".

### §18 Windows Update collector

Detect updates installed, update failure, reboot related to update; link events
temporally. Example: "Windows installed KBxxxxxx. Your computer restarted 4 minutes
later."

### §19 Driver collector

Meaningful driver version changes. Priority: GPU, audio, network, storage, input.
Human-friendly naming essential — "NVIDIA graphics driver updated 581.xx → 582.xx",
never merely a device GUID.

### §20 Network configuration collector

Track network adapters, DNS servers, routes, Windows proxy, default gateway, VPN
adapter appearance/disappearance, IP configuration where meaningful. Example: "Your
DNS server changed 1.1.1.1 → 192.168.1.1". Normal DHCP address rotation must **not**
be treated as critical unless context makes it relevant.

### §21 Startup collector

Track new startup application, removed, enabled/disabled, important scheduled startup
tasks where feasible. Example: "Adobe Creative Cloud began launching automatically
with Windows today."

### §22 Windows Event Log collector

Only relevant channels — **never ingest the whole Event Viewer database**. Focus:
application crashes, service failures, driver failures, unexpected shutdown, Windows
Update events, network failures, hardware errors. Normalize; raw event IDs live under
technical details.

### §23 Docker collector — optional module

Enable only if Docker detected; connect locally. Track container created / started /
stopped / restarted / removed, image changed, health changed, port mapping changed,
volume mapping changed. Must support this causal sequence:
`docker-compose.yml changed → container api restarted → port 3000 disappeared → port
4000 appeared`. **Never require Docker.** When absent, show "Docker integration — Not
installed" with **no error spam**.

### §24 Git / development mode

Opt-in only. User adds project directories. Track branch changed, commit changed,
working tree became dirty, package manifests changed, runtime config changed,
dependency lockfile changed. Do not continuously parse enormous repositories.

### §25 Checkpoint system (important)

User clicks "Create checkpoint". Example labels: "Everything is working", "Before
installing driver", "Before changing server config", "Before Windows update".
Checkpoint records timestamp, label, summary snapshot, important entity states.
Analysis can then compare `Checkpoint → N important changes → Current state`.
User-facing button: "Compare with checkpoint".

### §26 Health checks — explicitly "later MVP / Phase 2"

Advanced users configure a lightweight monitor: HTTP GET `localhost:3000/health`
expects 200; TCP port 25565 expected open; process `java.exe` expected running;
Windows service expected running. System records `13:10 HEALTHY` / `13:42 UNHEALTHY`
and automatically inspects changes between last healthy and first unhealthy state.
Enables "What changed right before it broke?".

### §27 Analysis engine — the core intellectual value

Do **not** just display every event. Deterministic relevance engine. LLM may come
later; the product must remain useful without one. Given problem description + time
window + affected entity, the engine must: (1) determine relevant timeframe,
(2) identify affected entities, (3) traverse relationships, (4) retrieve candidate
events, (5) score relevance, (6) group related event chains, (7) rank probable causes,
(8) generate human-readable explanations.

### §28 Relevance scoring — transparent

Factors: temporal proximity, entity relationship distance, event severity, event
rarity, event category relevance, before/after magnitude, known dependency
relationship, same application / executable / port / service / device / configuration
file, historical association, health state transition proximity.

```text
score = temporal_score + relationship_score + semantic_category_score
      + severity_score + rarity_score + failure_proximity_score
```

Scoring must not be fully hidden. Advanced UI shows:

```text
Why this is ranked #1
+ occurred 2 minutes before failure
+ directly affected Discord
+ changed microphone permission
+ no similar changes occurred earlier today
```

This exists to create trust.

### §29 Causal chain builder

Identify sequences, e.g. `.env modified → Docker container restarted → application
changed port → nginx still points to old port → HTTP 502`. Must distinguish
**confirmed relationship / likely relationship / temporal correlation**. Never claim
certainty without evidence.

Allowed language: "This is the most likely related change." / "These events may be
connected." / "We found a strong temporal relationship."
Forbidden unless causality is actually known: "This definitely caused the problem."

### §30 Entity relationship graph (stored in SQLite)

`process → executable`, `process → parent_process`, `process → listens_on → port`,
`service → process`, `application → executable`, `docker_container → exposes → port`,
`docker_container → uses → image`, `project → contains → config_file`,
`config_file → configures → application`, `application → uses → runtime`,
`network_adapter → uses → DNS`.

### §31 "What's wrong?" input

Home screen features a prominent natural-language input, placeholder "Tell me what
stopped working…". Real user inputs to support: "Discord microphone stopped working
this morning." / "My internet became slow after I restarted." / "Minecraft server
worked yesterday but not today." / "This application suddenly stopped launching." /
"My monitor started flickering today." / "Something changed with Docker."

MVP parsing is deterministic: time extraction, application/entity matching, keyword
classification. Core operation must not depend on cloud AI.

### §32 Optional AI layer

Abstraction `ExplanationProvider`; first implementation
`DeterministicExplanationProvider`. Later: OpenAI-compatible endpoint, local model,
other provider. AI may summarize, translate to natural language, classify the problem
description, explain likely relationships. **AI must never be trusted to invent system
facts** — prompt context contains only facts already retrieved deterministically, and
LLM output is clearly separated from evidence. Cloud AI OFF by default, explicit
opt-in. Never upload file contents automatically.

### §33 Privacy — critical (reputation depends on it)

Default: local-first, no account, no cloud. Never collect keystrokes, clipboard
history, browser history, private messages, screen recordings, screenshots, microphone
audio, camera, or arbitrary document contents. No spyware behavior — all monitoring
must map directly to system troubleshooting. No telemetry by default; if ever added:
explicit opt-in, anonymous, documented, disableable.

### §34 Secret handling

Never persist secrets from `.env`, environment variables, command-line arguments,
config files, Docker environment, URLs, HTTP headers. Central redaction engine
detecting API keys, Bearer tokens, JWTs, passwords, session cookies, private keys,
database URLs containing credentials, OAuth secrets. **Redaction runs before
persistence and before logging.** Unit tests must actively try to leak secrets.

### §35 Tauri security

Use Tauri 2 capabilities/permissions carefully; expose only required commands; no
generic `run_any_command()` / `read_any_file()` / `execute_shell()`. Narrow commands
like `get_recent_events()`, `create_checkpoint()`, `get_analysis()`,
`update_setting()`, `get_collector_status()`. Validate all input in Rust. Frontend is
untrusted relative to Rust. Smallest possible capability scopes.

### §36 Privilege model

Main process non-Administrator. If a feature needs elevation, a narrowly scoped
privileged helper; UAC only when necessary; never silently elevate; never keep full
admin for convenience.

### §37 Performance budget

Idle CPU ≈ 0–1% average. RAM target < 80 MB, stretch < 60 MB. Disk writes batched,
minimal, non-blocking. Database bounded by retention. Collectors event-driven where
practical; avoid constant high-frequency polling; batch DB writes; expensive analysis
only on demand; pause expensive visual effects when the window is hidden.

### §38 UI performance

Target 60 FPS. Three.js: pause rendering when not visible, reduce frame rate for
ambient scenes, stop when minimized, respect `prefers-reduced-motion`, avoid excessive
draw calls and unnecessary post-processing, dispose geometry/materials/textures.
Graceful non-WebGL fallback required.

### §39 Three.js rule

Required, but **do not build the whole interface as a canvas**. Buttons, forms, cards,
lists, timeline, tables, dialogs, settings, tooltips, and text stay DOM-based.
Legitimate uses: (a) Home ambient visualization — small particles = ordinary changes,
larger nodes = important changes, connections = related events, not distracting;
(b) **causal graph — the primary meaningful 3D visualization**, event nodes with
hover, click, zoom, careful pan/orbit, focus node, highlight dependency chain;
(c) investigation transition from ambient particles into a focused event cluster.
Elegant, subtle, premium. No gaming UI, no cyberpunk overload, no particle spam.

### §40 Visual direction

Should feel like Linear / Raycast / Arc / modern observability software / a premium
native desktop utility — but with its own identity. Personality: dark, calm, precise,
technical, premium, trustworthy, slightly futuristic.

Explicitly avoid: generic admin dashboard, bootstrap look, huge gradient blobs,
excessive glassmorphism, neon cyberpunk, gamer HUD, random glowing borders, dozens of
colors. Restrained glass effects; depth used sparingly.

### §41 Design tokens

Required tokens: `background`, `surface-1/2/3`, `border-subtle`, `border-active`,
`text-primary/secondary/muted`, `accent`, `success`, `warning`, `danger`, `info`,
`shadow-sm/md/lg`, `radius-sm/md/lg/xl`, a spacing scale, motion durations, easing
curves. Support Dark / Light / System. Dark may be the MVP default if quality is
clearly stronger, but the architecture must support all three.

### §42 Motion design

Animation communicates hierarchy and cause/effect: spring-like panel transitions,
timeline insertion animations, node focus transitions, number interpolation, subtle
hover feedback, shared element transitions where appropriate. Do not animate every
property. Respect reduced-motion.

### §43 Home screen — the emotional core

```text
┌───────────────────────────────────────────────┐
│ What Changed                         ● Active │
│           Everything looks normal             │
│       What stopped working?                   │
│ ┌───────────────────────────────────────────┐ │
│ │ Tell me what happened…                    │ │
│ └───────────────────────────────────────────┘ │
│ [ Investigate ]                               │
│ Today                                         │
│ 37 changes     2 important     0 critical     │
│ Important recent changes …                    │
└───────────────────────────────────────────────┘
```

Three.js ambient scene may sit behind/around hero content; **readability comes first**.

### §44 Timeline screen (major)

Ranges: Today / Yesterday / Last 7 days / Custom range.
Filters: All / Apps / Windows / Network / Drivers / Files / Development / Docker /
Important only.

Event presentation:

```text
10:42 AM
NVIDIA graphics driver updated
581.xx → 582.xx
34 minutes before first game crash
[ View details ]
```

Cluster related changes — never render 500 tiny low-level events individually. Use
"12 Docker-related changes [Expand]".

### §45 Investigation screen

For "Discord microphone stopped working today.":

```text
Investigation
Discord microphone stopped working today
We found 3 changes that may be related.

#1 Strong match
Microphone permissions changed        10:42 AM
Why this matters: Discord depends on microphone permission.
Timing: 8 minutes before the reported problem.
[ View evidence ]

#2  Discord updated            10:37 AM
#3  Windows Audio restarted    10:44 AM

What happened
10:37 Discord updated
        ↓
10:42 microphone permission changed
        ↓
10:44 Windows Audio restarted
        ↓
~10:50 issue observed
```

Plus a "Show technical details" affordance.

### §46 Event details drawer

Contains: human explanation, timestamp, importance, confidence, before, after, related
events, affected entities, source collector, technical details, raw identifiers.

```text
Network configuration changed
DNS
Before   1.1.1.1
After    192.168.1.1
Detected by  Windows Network Collector
```

### §47 Checkpoint screen

Cards like "Everything working — Aug 29, 1:20 AM", "Before NVIDIA update — Aug 26,
8:11 PM", "Stable Minecraft server — Aug 24, 3:42 PM". Actions: Compare / Rename /
Delete. Comparison view headline: "12 important changes since this checkpoint".

### §48 System Map (advanced)

Interactive relationship graph over Applications, Processes, Services, Ports, Docker
containers, Network adapters, Config files. Three.js allowed. Answers "What depends on
this?" and later enables the Impact feature. Must **not** be necessary for normal users.

### §49 Settings

Sections: General, Monitoring, Privacy, Storage, Appearance, Advanced, Integrations,
About. Monitoring toggles: Apps & updates, Processes, Network, Services, Drivers,
Windows updates, Startup apps, File/config monitoring, Docker, Development projects.
Each toggle explains what is collected, why, estimated cost, and whether admin access
is needed.

### §50 Onboarding (< ~1 minute)

1. "What Changed — When something suddenly stops working, What Changed helps you see
   what changed beforehand."
2. "Everything stays on this computer by default."
3. Choose monitoring — checked by default: Apps & updates, Windows settings, Network,
   Drivers, Services. Unchecked optional: Development tools, Docker.
4. "Start with Windows? — Recommended"
5. "Monitoring started." + a short animated example timeline.

### §51 Simple / Advanced experience

**One app, not two.** Default UI simple; technical details progressively disclosed.
"NVIDIA driver updated" expands to Device instance, Driver INF, Version, Provider,
Timestamp, Raw collector metadata.

### §52 Search

Global search over application names, event descriptions, file names, ports, services,
drivers, Docker containers, checkpoint labels. Example queries: `Discord`, `Java`,
`25565`, `NVIDIA`, `nginx`, `Docker`, `DNS`. Fast indexed SQLite queries.

### §53 Notifications — extremely conservative

Good: "An application you're monitoring became unhealthy." / "A graphics driver changed
after you created 'Before driver update' checkpoint."
Bad (must not fire): "Chrome updated." / "Spotify updated." / "Process started." /
"Process stopped." Default notification volume very low.

### §54 No false panic

Importance levels: `informational`, `notable`, `important`, `critical`. Most events are
informational. Do not mark every change red. The product should feel calm.

### §55 Project structure (preserve these modular boundaries; adjust only if necessary)

```text
what-changed/
├─ README.md  ARCHITECTURE.md  PRODUCT.md  SECURITY.md  PRIVACY.md  ROADMAP.md  CONTRIBUTING.md
├─ package.json  bun.lock  vite.config.ts  svelte.config.js  tsconfig.json
├─ src/
│  ├─ app/
│  ├─ components/{ui,timeline,investigation,graph,system}/
│  ├─ features/{home,timeline,investigation,checkpoints,search,settings,onboarding}/
│  ├─ lib/{tauri,stores,types,formatting,design}/
│  ├─ three/{ambient,causal-graph,system-map}/
│  └─ styles/
└─ src-tauri/
   ├─ Cargo.toml  tauri.conf.json  capabilities/  migrations/
   └─ src/
      ├─ lib.rs
      ├─ app/  commands/  domain/  storage/  analysis/  privacy/  redact/
      ├─ platform/windows/
      ├─ collectors/{mod.rs,files,process,network,services,applications,updates,drivers,startup,eventlog,docker,git}/
      └─ tests/
```

### §56 Frontend ↔ Rust API

Explicit typed request/response objects. Named commands: `get_dashboard_summary`,
`get_events`, `get_event_details`, `search_events`, `create_checkpoint`,
`delete_checkpoint`, `compare_checkpoint`, `run_investigation`, `get_collector_status`,
`set_collector_enabled`, `get_settings`, `update_settings`. Frontend never executes raw
SQL; no arbitrary Rust functions exposed.

### §57 Live events

Tauri events/channels: `system-event-added`, `collector-status-changed`,
`analysis-completed`, `monitoring-state-changed`. Throttle UI updates under load; never
re-render the entire timeline per event.

### §58 Error handling

Every collector isolated. Docker failure shows "Docker integration temporarily
unavailable." and the app continues. Never crash because Docker is absent, a registry
key is inaccessible, a file disappears, a process exits mid-inspection, or Event Log
access is denied. Typed errors; avoid `unwrap()` in production paths.

### §59 Testing

Rust: unit, integration, database migration, redaction, analysis ranking, collector
parser tests. Frontend: Vitest + Testing Library. E2E: Playwright where practical.

Synthetic fixture that must work: config port 3000 → 4000; container restart; port 3000
close; port 4000 open; HTTP health failure. Expected: the port/config chain ranks above
an unrelated Chrome update.

### §60 Redaction tests are mandatory

Test values resembling `sk-...`, `Bearer ...`, `postgres://user:password@host`,
`mongodb://...`, a JWT, AWS-style keys, a private key PEM, a session token, a cookie.
Assert raw values never reach persisted event metadata.

### §61 Performance testing

Benchmarks/stress for 10,000 / 100,000 / 1,000,000 historical events; rapid file event
bursts; rapid process events; timeline pagination; search; analysis ranking. Timeline
must virtualize. Never load all events into frontend memory.

### §62 Accessibility

Keyboard navigation, focus indicators, semantic HTML, screen reader labels, sufficient
contrast, reduced motion, scalable text. Three.js visuals must never hold essential
information without an accessible DOM equivalent.

### §63 Offline

Core app works fully offline. No CDN dependency, icons/assets bundled locally, no
required online font, no required cloud API.

### §64 Update system

Architecture ready for signed updates. No silent auto-update in early MVP. Provide
"Check for updates" now, "Automatically download updates" later. Updates must never
erase the event database; migration safety essential.

### §65 Installer

Windows installer artifacts targeting `WhatChanged-Setup-x64.exe` and/or appropriate
Tauri installer formats. Support Start Menu shortcut, optional desktop shortcut,
optional launch at startup, clean uninstall. User data must not be silently deleted
during upgrade. During uninstall, ask whether history should also be removed where
technically feasible.

### §66 Code signing

Development signing separate from production signing. Document how production Windows
code signing should be configured. **Never hardcode signing secrets in the repository.**

### §67 Development commands

Target workflow: `bun install`, `bun run dev`, `bun run tauri dev`, `bun run check`,
`bun run test`, `bun run build`, `bun run tauri build`. Package scripts must be
consistent and documented. No Node installation required if Bun is present in dev. The
distributed app must not require Bun or Node.

### §68 Documentation to maintain (and keep non-stale)

`PRODUCT.md` (vision + UX principles), `ARCHITECTURE.md`, `SECURITY.md` (threat model +
security decisions), `PRIVACY.md` (exactly what is and is not collected), `ROADMAP.md`
(current phases), `COLLECTORS.md` (collector contracts + implementation status),
`ANALYSIS.md` (relevance and causal-chain model).

### §69 Phased implementation — the user's prescribed order (binding)

The user explicitly said "Do not attempt everything simultaneously. Implement in the
following order." Candidate plans may **map** these onto plan phases (and may split a
user phase into multiple plan phases for reviewability), but must not reorder the
dependency logic or drop a phase.

**PHASE 0 — FOUNDATION.** Create: repo, Tauri 2 app, Svelte 5, TypeScript, Vite, Bun,
Tailwind 4, SQLite, Rust project structure, logging, migrations, settings, tray. Build
skeleton screens. Verify release build.
*Acceptance:* app launches; window works; tray works; close → tray; database created;
theme works; frontend ↔ Rust IPC works; installer builds.

**PHASE 1 — EVENT ENGINE.** SystemEvent model, Entity model, SQLite repositories, event
querying, pagination, retention, live frontend updates. Add a synthetic event generator
for frontend development; build polished Timeline UI using fake data first.
*Acceptance:* 10k synthetic events remain responsive; filtering works; pagination works;
details drawer works.

**PHASE 2 — FIRST REAL COLLECTORS.** Process lifecycle, ports/listeners, selected file
changes, installed applications, Windows services. Normalize all events.
*Acceptance:* controlled actions verified in the timeline — start app, stop app, open
port, close port, modify watched config, install/update detectable app, restart service.

**PHASE 3 — WINDOWS CHANGE INTELLIGENCE.** Windows update, drivers, network
configuration, startup entries, relevant Event Log events. Build before/after snapshots.

**PHASE 4 — CHECKPOINTS.** Create, rename, delete, compare. Polished comparison screen.

**PHASE 5 — ANALYSIS ENGINE.** Problem parsing, candidate retrieval, relevance scoring,
relationship traversal, ranking, explanation. Build Investigation screen. No LLM
required.
*Acceptance fixture:* config changes port → service restarts → old port disappears →
new port opens → health fails **must rank above an unrelated software update**.

**PHASE 6 — CAUSAL GRAPH + THREE.JS.** Ambient event field, causal graph, investigation
transitions. The app must already be fully usable without this phase; Three.js must not
block core functionality.

**PHASE 7 — DOCKER / DEVELOPMENT MODE.** Optional Docker, Git, project config, runtime
detection.

**PHASE 8 — POLISH / RELEASE.** Onboarding, performance, accessibility, error handling,
installer, updates, privacy, security, animations, empty states, loading states, visual
consistency. Full release audit.

### §70 UI quality bar

Unacceptable: plain cards on a black background, generic Tailwind dashboard, random
gradient buttons, inconsistent spacing, poor empty states, developer-looking forms,
unstyled tables.

**Every major screen must have all six states:** loading, empty, error, success,
large-data, small-window. Use real microcopy — never lorem ipsum.

### §71 Window responsiveness

Desktop-first. Support down to ~900 × 600. Main target 1200–1600 px wide. Do not design
like a stretched mobile webpage; use desktop density intelligently.

### §72 Sample Home microcopy (use this exact voice)

Status: "Everything looks normal." or "2 recent changes may need attention."
Prompt: "What stopped working?"
Placeholder: "Example: Discord stopped detecting my microphone this morning"
Timeline heading: "Important changes". Button: "Investigate".
Privacy line: "Your history stays on this PC."

### §73 Example investigation — this tone is the target

Input: "My game started crashing this morning."
Evidence: 07:52 Windows startup; 08:02 NVIDIA driver update completed; 08:07 PC reboot;
08:32 game launched; 08:34 Application Error event.

```text
Most likely related change
NVIDIA graphics driver updated

Why it stands out
• It changed shortly before the first crash.
• The affected application uses the GPU.
• No game executable/configuration changes were detected.
• Similar crashes were not detected before the driver update.

Confidence  High

This does not prove the driver caused the crash,
but it is the strongest recent change we found.
```

### §74 Example development investigation

Input: "My API returns 502 after I restarted Docker."
Evidence: 13:40 `.env API_PORT 3000 → 4000`; 13:41 container api restarted; 13:41 TCP
:3000 closed; 13:41 TCP :4000 opened; 13:42 nginx health request failed.

```text
Strong match
Your application moved from port 3000 to 4000,
but another component still appears to expect port 3000.

Evidence
.env       API_PORT 3000 → 4000
Container  api restarted
Network    3000 closed, 4000 opened

Possible next step
Check the upstream port used by your reverse proxy.
```

**Do not automatically modify config in MVP.**

### §75–§78 Future features — architect for, do NOT build

Keep in `ROADMAP.md`, do not scope-creep: "Can I Delete This?", Impact / blast radius,
FixMemory ("You've encountered something similar before" — stores problem, evidence,
probable cause, user-confirmed resolution), rollback, universal undo, team/cloud mode,
remote machines, Linux, macOS, mobile companion, browser extension, LLM agent,
automatic repairs. The relationship graph built now is what makes Impact possible later.

### §79 Coding standard

Rust: clippy clean, rustfmt, no unnecessary `unsafe` (document where unavoidable),
typed errors, small modules, dependency injection where useful, traits only when the
abstraction is valuable.
TypeScript: strict, no `any` unless documented, small components, derived state instead
of duplicated state, typed Tauri wrappers, no business logic buried in UI components.
CSS: design tokens, consistent spacing, no giant one-off style files.

### §80 Dependency policy

Before adding a dependency ask: can std/an existing dep solve this cleanly; is it
actively maintained; does it substantially increase binary size; does it require a
background runtime; does it create security risk. Avoid bloat; never add a library for
one trivial utility.

### §81 Source of truth

Rust domain types are authoritative for backend data. Maintain TypeScript types
carefully; do not let Rust and TS payload definitions silently diverge. Automate type
generation if practical.

### §82 Security threat model — document and deliberately mitigate

Malicious frontend script; malicious config file; symlink/path traversal; secret
leakage; database corruption; untrusted Docker metadata; malformed Windows Event Log
entries; large event floods; IPC abuse; update tampering; local privilege escalation.

### §83 Database safety

WAL, busy timeout, transactions, schema migrations, backup before risky migration.
Handle unclean shutdown. On corruption: do **not** crash-loop; offer "Repair database",
"Export diagnostics", "Reset local history".

### §84 Diagnostics screen

Show app version, database size, event count, collector health, last collector error,
WebView version, Rust backend health, monitoring status. Provide "Export diagnostics",
which must automatically redact private data.

### §85 Dev tooling

Hidden/dev-only simulation panel to inject events: driver update, port change, process
crash, DNS change, Docker restart, config diff. Makes UI development deterministic. Must
not ship enabled in production unless explicitly behind developer mode.

### §86 Implementation behavior expected of the executing agent

Inspect the existing repository; do not delete useful existing work; read current
dependencies; create/update architecture documents; create a concrete task list; begin
implementation. Do not repeatedly ask the user to confirm routine engineering decisions
— make sensible decisions and document them. Ask only when a required secret is missing,
a destructive action is unavoidable, or a product decision is genuinely impossible to
infer.

### §87 Do not fake implementations

Never write `// TODO implement real monitoring` and mark a phase complete. A phase is
complete only when functionality genuinely works. Temporary stubs allowed in earlier
phases but must be visibly tracked in TODO/ROADMAP. Never simulate a production collector
and present it as real.

### §88 Verify after every phase

Run formatting, linting, type checks, Rust tests, frontend tests, build. Then launch the
app if the environment permits. Document what was completed, what was tested, what
remains, and known limitations. Fix build errors before moving on.

### §89 Commit-sized work

Coherent chunks: foundation; database/event domain; timeline UI; process collector;
network listener collector; checkpoint engine; analysis engine; causal visualization. No
single enormous unreviewable file.

### §90 First implementation task

Start with Phase 0. Before implementation code, produce/update `PRODUCT.md`,
`ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`. Then scaffold the app and
build a visually polished initial shell containing: custom titlebar/window chrome where
appropriate, sidebar/navigation, Home, Timeline, Investigations, Checkpoints, System Map
placeholder, Settings, system tray, theme, SQLite initialization, typed Tauri IPC,
synthetic event feed. The initial UI must already communicate the intended premium
visual identity. Three.js ambient visualization only after the DOM layout is stable.

At the end of Phase 0 report: project tree, architecture summary, commands to run, tests
performed, build status, screens/features implemented, next Phase 1 tasks. Then proceed
into Phase 1 unless genuinely blocked.

### §91 Final product principle

Every decision must support one moment: a user thinks *"This worked yesterday."*, opens
What Changed, and within seconds understands *"Oh. THAT changed."*

---

## 4. Research — REQUIRED READING

Read this file in full before writing your candidate plan:

```text
D:\Project\.WhatChage\plans\260829-0437-what-changed-desktop-mvp\research\verified-facts-report.md
```

It contains:

- **§1** Live version pins from crates.io and npm (fetched 2026-08-29). Use these
  exact versions; do not invent or guess a version number.
- **§2** A **35-row non-admin capability matrix measured on the actual target machine**
  under a plain `BUILTIN\Users` token. This is the authoritative answer to "does this
  collector need Administrator?" — 34 of 35 capabilities passed. Two hard gaps:
  push-based process events (`Win32_ProcessStartTrace` = Access denied) and the
  `Security` event-log channel (unauthorized).
- **§3** Measured snapshot costs, which dictate poll intervals. Notably the driver
  inventory takes **~2.1 seconds even warm** and must never sit on a short timer, while
  the process list is **~6 ms warm** and can be diff-polled.
- **§4** Nine stack decisions the evidence supports.
- **§5** Eleven explicitly **unverified** assumptions, each with a settle-it action.
  Your plan must carry these as real risks with signals and pre-decided responses —
  not restate them as facts.

Also available: the three re-runnable probe scripts in the same `research/` directory.

**A note on research honesty:** the `WebSearch` tool is unavailable in this session
(`web_search: no available accounts`), which is why two delegated researcher subagents
stalled and were stopped. No external documentation was consulted. Every version number
came from a live registry query; every privilege claim came from a probe executed on
this machine. Anything else is in §5 as an assumption. **Do not fabricate a citation,
a doc URL, or an API signature to fill a gap** — mark it unverified instead.

---

## 5. Known blockers and decisions the plan must resolve

### Blocker B1 — Rust is not installed (hard, schedule-critical)

No `rustc`, `cargo`, or `rustup`; `~/.cargo` and `~/.rustup` are absent. Nothing
compiles until this is fixed. Three MSVC installs exist, so linker selection is
ambiguous. The plan's first executable step must install `rustup`, select
`stable-x86_64-pc-windows-msvc`, pin it in `rust-toolchain.toml`, and prove it by
building a trivial Tauri app **before** any feature work.

### Blocker B2 — The <80 MB RAM target may be physically unreachable

§37 asks for <80 MB (stretch <60 MB) while §4 mandates Tauri 2, which means WebView2 and
its child processes. This was not measured. A candidate plan that simply repeats "<80 MB"
as an acceptance criterion without a measurement step and a stated fallback is not
credible. Handle it explicitly: measure early, separate backend RSS from WebView2 total,
and define what happens if the target cannot be met (report to the user as a scope
question — do not silently miss it, and do not quietly redefine the metric).

### Blocker B3 — Non-admin process events have a real blind spot

Diff-polling cannot see a process that starts and exits within one interval. §14 asks for
"unexpected exit if detectable" — the honest answer is partial detection. The plan must
say so in `COLLECTORS.md` rather than implying complete coverage.

### Decision D1 — Repo folder name is misspelled

The working directory is `D:\Project\.WhatChage` (missing the `n`). Product, binary, and
package names must still be correct (`What Changed`, `WhatChanged.exe`, `what-changed`).
Renaming the folder is the user's call; assume it stays and do not depend on the folder
name anywhere in code or config.

### Decision D2 — TypeScript 7 vs pinning back

`typescript@latest` is 7.0.2, a compiler rewrite. Toolchain support for `svelte-check`,
`vite-plugin-svelte` 7, and Vite 8 is unconfirmed. The plan needs a Phase 0 decision gate
with a stated fallback, not an assumption either way.

### Decision D3 — Docker is untestable here

Docker is not installed and this is Windows 11 **Home**. Only §23's "absent, no error
spam" path can be verified on this machine. Plan for that honestly.

---

## 6. What a candidate plan is being judged on

Summarized so candidates optimize for the right things (the verifier applies these
with 1-20 scores):

1. **Requirement fidelity** — all of §1–§91 delivered, nothing silently dropped, nothing
   unrequested added, §69's phase order respected.
2. **Technical correctness** — real APIs, real crates, the pinned versions, and
   privilege claims that match the measured matrix. An invented API is disqualifying.
3. **Executability** — exact paths, exact commands, per-phase file inventory, gates that
   actually run on this machine (Git Bash, Bun, MSVC, no admin).
4. **Verification rigor** — checkable per-phase criteria plus the mandated suites:
   redaction leak tests, the analysis-ranking fixture, 10k/100k/1M perf, migration tests.
5. **Architecture quality** — collector trait, normalizer, repository, analysis engine;
   `core/` vs `platform/`; narrow typed IPC; Rust-as-source-of-truth types; modular files.
6. **Security & privacy engineering** — redaction as an enforced chokepoint before
   persistence *and* logging, §82 threat coverage, minimal Tauri capabilities, DB
   corruption recovery, diagnostics redaction.
7. **Risk realism** — the hard parts named, with a signal and a pre-decided response.
   Hand-waving B1/B2/B3 is penalized heavily.
8. **Product/UX substance** — six states per screen, the exact microcopy voice, calm
   importance model, conservative notifications, a11y, 900×600, design tokens, and
   Three.js used intentionally rather than as a canvas app.
9. **Sequencing & reviewability** — commit-sized chunks, sane ordering, honest effort.
10. **Plan-format compliance** — see the dispatch prompt for the exact required shape.

