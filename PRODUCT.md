# Product

What Changed answers one question: **"It worked before. What changed?"**

It is a change recorder with an investigator attached. It watches the parts of Windows that break
software, writes a plain-language history of every change, and — when you tell it what stopped
working — ranks the changes around that moment by how much evidence connects them to your symptom.

## What it is not

| Not | Why it matters |
|---|---|
| Not an antivirus or a system cleaner | It changes nothing on your machine. Every data source is opened read-only. |
| Not a monitoring dashboard | No CPU graphs, no gauges. The unit of the product is a change, not a metric. |
| Not a log viewer | Raw provider text is the fallback, not the product. Every event has a human sentence. |
| Not a diagnoser | It never says a change caused your problem. See [ANALYSIS.md](ANALYSIS.md). |
| Not a cloud service | No account, no sync, no telemetry, no outbound request at runtime. |

## The worked example

The plan's reference scenario: a local API stops responding on port 3000. The investigation window
contains a Chrome update, a Spotify restart, dozens of routine background changes, and one small
chain — a `.env` file in the watched project folder changed its `PORT` key, the `node.exe` process
that was listening on 3000 exited, and port 3000 stopped being listened on. The Chrome update is more
recent and more visible. The chain is what matters, so the chain outranks it, and the product says so
in evidence terms rather than in cause terms.

## Core loop

1. Collectors observe change (see [COLLECTORS.md](COLLECTORS.md)).
2. Each observation is normalised into an event with a human summary and a technical summary.
3. The event passes the redaction chokepoint before it can be written (see [PRIVACY.md](PRIVACY.md)).
4. It lands in the local SQLite database and appears in the timeline within about half a second.
5. You either browse the timeline, or you describe a symptom and get a ranked, explained shortlist.
6. A registered project root can act as a recovery workspace: mark it working, scope the next investigation to recorded project evidence, and copy a redacted Markdown handoff to any AI assistant.
7. Checkpoints let you mark "it worked here" and compare the machine against that marker later.

For people building with AI, Home also offers a direct **Open My apps** path into the
owner workspace. There you can keep the app's intent, confirmed rules, change requests,
reviewed brief and evidence together without leaving the local desktop workflow. The
legacy symptom investigation remains available beside it.

## Screens

Seven sidebar destinations, reachable with `Ctrl+1` through `Ctrl+7`, collapsing to an icon rail below
1040 px. Window opens at 1280x820 and stays usable at its 900x600 minimum.

| Route | Purpose |
|---|---|
| Home | Status, today's counts, important changes, and the symptom prompt |
| Timeline | The full history, grouped by hour, filterable by range and category |
| Investigations | Ranked evidence for a described symptom, plus past investigations |
| Checkpoints | "It worked here" markers and comparisons against them |
| System Map | What depends on what: the relationship graph, for "what else touches this?" |
| Settings | Collectors, watched folders, recovery workspaces, retention, privacy, theme, startup |
| Diagnostics | Collector health, database state, export for a bug report |

Four surfaces are not routes: the event **details drawer** (opens over any screen), the `Ctrl+K`
**search palette**, the first-run **onboarding** flow, and the contextual **Change Map** opened from an
Investigation or the current Timeline window. Change Map preserves its origin and returns focus there;
it has no sidebar item or global number shortcut.

## Home

The screen that has to be right, because it is the one a user sees when they are already annoyed.

| Element | Copy |
|---|---|
| Status pill | `● Active` or `Paused` |
| Headline (calm) | `Everything looks normal.` |
| Headline (attention) | `N recent changes may need attention.` |
| Prompt | `What stopped working?` |
| Input placeholder | `Example: Discord stopped detecting my microphone this morning` |
| Primary action | `Investigate` |
| Today's counts | `37 changes · 2 important · 0 critical` |
| Last checkpoint | The newest named checkpoint and its creation time, when one exists |
| Quick ranges | `Today` / `Since last restart` / `Yesterday` |
| Privacy line | `Your history stays on this PC.` |
| First run | `We just started watching. Come back after something changes.` |

Below that, an "Important changes" list — only events the importance classifier rated `important` or
`critical`, never a firehose.

If local history is unavailable or its database is not ready, `Recovery needed` replaces ordinary history
screens and links to Diagnostics. The global status cannot say `Active` while recovery is required.

## Timeline

Range tabs: `Today` / `Yesterday` / `Last 7 days` / `Custom`. Category chips: `All` / `Apps` /
`Windows` / `Network` / `Drivers` / `Files` / `Development` / `Docker` / `Important only`. Rows are
grouped by local hour with sticky headers.

Each row carries the time, a human title, a `before -> after` pair when one exists, and — only when a
correlation actually exists — a relative note such as `34 minutes before first game crash`. Every row
ends in `View details`. The row's bounded summary payload already contains its scrubbed `before` and
`after` values, so rendering a visible delta does not trigger a details request for every row.

Bursts do not flood the list. Events that share a category, a coarse entity key, and a 60-second
bucket collapse into one clustered row with a count and an `[Expand]` action that loads its members
inline.

While you are scrolled down, new events never move the viewport; a `N new changes` pill appears
instead and scrolls to the top on click. At the top of a live range, new rows animate in.

## Details drawer

Progressive disclosure, in this order: the human explanation, the timestamp as both absolute and
relative, an importance chip, a confidence badge, the before/after table, related events you can click
through to, the affected entities, and `Detected by <collector>`. Raw metadata and identifiers live
behind a collapsed `Show technical details` section that remembers whether you like it open.

`Esc` closes the drawer and focus returns to the row that opened it.

## Search

`Ctrl+K` anywhere. Debounced, grouped by kind — Applications, Files, Ports, Services, Drivers, Docker,
Checkpoints, Events — navigable with the arrow keys. `Enter` either opens that event's drawer or
filters the timeline to that entity. Typing a port number finds the port. Empty state:
`No matches. Try an app name, a port, or a file name.`

## Investigations

You describe the symptom in your own words. The engine extracts keywords, picks a time window, scores
every candidate change in it, and returns a ranked shortlist where each item states its evidence and
its confidence class:

Choose **All recorded changes** for the existing machine-wide view, or choose an explicit recovery
workspace created from a registered project root. Workspace scope is limited to project associations
that were actually recorded; it does not claim to capture every external dependency. A workspace's
known-good baseline becomes the default window for a new investigation only when you have not chosen
another window. Changing the selector never relabels an existing result.

| Class | Sentence |
|---|---|
| Confirmed | `This change directly affected <entity>.` |
| Likely | `This change likely affected <entity>.` |
| Correlated | `This changed around the same time.` |

When nothing clears the reporting threshold it says so and explains why, rather than promoting the
best of a bad set. Very large windows are downsampled with the disclosure
`Analysed the 5,000 most important changes in this window`. The scoring rules, the weights, and the
forbidden vocabulary are in [ANALYSIS.md](ANALYSIS.md). **Copy context for AI** places a bounded,
sanitized Markdown pack in the clipboard; it contains evidence and correlation, never a causal claim,
and never sends data over the network. A workspace with no recorded evidence says so explicitly.

## Checkpoints

A checkpoint is a named "it worked here" marker plus a full snapshot of the machine's observable state:
processes, listening ports, services, installed applications, drivers, network configuration, and
startup entries. Default name follows the shape `Checkpoint Friday 3:14 PM`; you can rename it.

Comparing a checkpoint against now — or against another checkpoint — yields three lists: what was
added, what was removed, and what changed, plus the events recorded in between. Snapshotting is slow
by nature (it is the sum of every collector's full sweep) so a collector that overruns its budget is
recorded as partial rather than silently dropped.

Checkpoints are also created automatically before a Windows Update install, when an installer process
starts, and on a weekly cadence. Automatic checkpoints are capped so they cannot accumulate, and
checkpoint snapshots are exempt from retention pruning — a checkpoint you named must still be there
when you need it.

Settings also lists explicit recovery workspaces. **Mark working** captures a manual checkpoint and
attaches it only when the capture is complete; a partial capture is saved globally but leaves the
previous workspace baseline unchanged.

## Change Map

One visualisation with one job: make the *shape* of a cascade visible when a list cannot. Time on X,
category lane on Y, chain depth on Z. Node size encodes importance; colour encodes category. Structural
relationships draw as bright edges, temporal proximity as faint ones.

Selecting or hovering a node dims everything outside its chain, which is the single interaction that
justifies the third axis. A translucent plane marks the symptom time, labelled
`You noticed the problem here`. Clicking a node opens the same details drawer the timeline uses.

The map is never the only way to reach the information. A `Switch to list` control is always present, a
keyboard-reachable ordered node list exists behind it, and a complete 2D canvas map ships alongside the
3D one — used automatically when WebGL is unavailable or lost, or when you set the visualisation
preference to 2D. If the context is lost at runtime you get
`Switched to the simple map because 3D stopped working.` Very large windows show
`Showing the 3,000 most important changes`; an empty one shows `Nothing to map in this window.`

## System Map

A different question, and therefore a different screen: not *what happened in sequence* but *what
depends on what*. The relationship graph the collectors have been recording all along — a process and
the port it opens or listens on, an application and the service it provides, a container and the port it
publishes, a config file and the application it configures — rendered so you can pick anything and see
its neighbourhood. The map never invents a process-to-executable or service-to-process edge that the
current collectors do not persist.

It answers "what else touches this?" before you change something, and it is the data the future Impact
and Can-I-Delete-This features will need. It is deliberately not on the path a non-technical user takes;
nothing on Home, Timeline or Investigations requires visiting it — and that is a test, not an intention:
a build gate walks those three route trees and fails if reaching any primary action requires this screen.

Every edge links to the event that recorded it, so a dependency is something you can check rather than
something the app asserts. A dependency kind with nothing recorded — no Docker containers, say — renders
as a disabled filter naming the reason, because an empty graph with no explanation is worse than no graph.
Focus defaults to two hops; the layout is concentric rings by distance and is deterministic, so the same
neighbourhood always draws the same way. Above 400 connected things it says
`Showing 400 of 1,832 connected things` rather than quietly truncating, and it shares the Change Map's
renderer, capability detection and 2D fallback rather than shipping its own.

## Settings and privacy

Collector toggles are self-describing: each names what it watches and why it matters. Two examples that
must stay verbatim, because they are promises:

- `Docker — containers, images and volumes starting, stopping and being rebuilt`
- `Developer environment — environment variables, PATH, installed tool versions, and the branch of
  folders you watch. Values are never stored.`

The privacy screen is inspectable rather than reassuring: live counts of what is stored by category,
database size, oldest record, log size and path; the plain statements (no account, no network calls, no
telemetry, no crash reporting); retention sliders; and deletion controls at three granularities —
one folder's history, one category's history, or everything. Deleting everything requires typing the
word `delete` and confirms with
`Your history has been deleted. We're watching again from now.`

A "verify for yourself" button opens the data folder in Explorer. That is the one shell-adjacent thing
the application does, and it exists precisely so the privacy claim is checkable.

## Diagnostics

Application and OS version, database state and size, migration version, and a per-collector table:
state, cadence, last run, last duration, events emitted, consecutive failures, and which wake mechanism
it negotiated. Plus write-queue depth, dropped-event counters, and the current and peak memory reading.

When any collector is failed or unavailable, a dismissible banner reads
`Some things aren't being watched right now.` and links here. Never a modal, never a blocking error —
the rest of the application keeps working.

`Export diagnostics` writes one plain text file you can paste into a bug report. It contains versions,
redacted settings, collector status, counts, recent log lines, and recent errors. It contains **no
event content, no file paths, and no entity names**, and that is enforced by a test rather than by
intent.

## Onboarding

Three steps, skippable at any point, shown once, and re-openable from Settings.

1. **What this does** — the headline `It worked before. What changed?`, one paragraph, and
   `Your history stays on this PC.` No signup, no email field, and no telemetry consent, because there
   is no telemetry.
2. **What we watch** — the collector list with the same descriptions Settings uses, each with a toggle.
   Everything on by default except Docker when the pipe is absent and the folder watcher when there is
   nothing to watch yet. Copy line: `We record that a value changed, never the value itself.`
3. **Watch your project folders** — pick zero or more folders, plus a "Start with Windows" toggle.

Finishing writes the completion timestamp and creates the first automatic checkpoint, so a comparison is
possible on day one.

## Tray and lifecycle

Closing the window hides it; monitoring continues; the first hide shows one balloon:
`What Changed is still recording. Open it from the tray.` Quit is the only way out. A second launch
focuses the existing window instead of starting a second copy.

The tray menu has exactly seven entries, in this order: `Open What Changed`, `What changed today?`,
`Create checkpoint`, `Pause monitoring`, `Resume monitoring`, `Settings`, `Quit`. Pause and Resume are
both always present; the inapplicable one is disabled rather than removed, so the menu never changes
shape under the cursor.

## Notifications: restraint as a feature

A change recorder that notifies is a change recorder you turn off. The notification set is an
allowlist, not a filter — a new event type is silent until someone adds it deliberately:

| Notifies | Never notifies |
|---|---|
| Anything classified `critical` | Application updates |
| A driver update | Routine process starts and stops |
| A proxy or DNS change | Port churn |
| A failed Windows Update | Config changes |
| A failed service you have a relationship with | Anything at all while monitoring is paused |
| A container that died with a non-zero exit | Anything in the first minute after launch |

On top of that: at most 3 per hour and 8 per day, a 30-minute cooldown per category, and optional quiet
hours. The plan's acceptance test replays a synthetic 24-hour stream of 4,000 events and requires
exactly three notifications, none of them from the 3,900 routine ones.

Clicking a notification focuses the window and opens that event's drawer.

## Voice

| Rule | Instead of |
|---|---|
| Say what changed, in a sentence a person would say aloud | Dumping the provider's message |
| Name the thing, then the direction: `Docker Desktop updated 4.35.1 -> 4.36.0` | `Version delta detected` |
| Describe evidence, not blame | `caused`, `because of`, `root cause`, `this broke` |
| Admit a gap in the same sentence as the feature | Silence about the blind spot |
| Absolute numbers, no false precision | `~`, `roughly`, invented percentages |

Two examples of the redaction voice, which appear in the product itself: `DATABASE_URL changed in .env`
and `AWS_SECRET_ACCESS_KEY changed` — both with `[REDACTED]` where a value would be.

## Every screen has six states

Not four. The plan treats this as a checkable rule, with a development-only harness that renders every
screen against every state, including inside a 900x600 frame.

| State | What it must do |
|---|---|
| Loading | Skeleton shaped like the content, never a spinner over an empty page |
| Empty | Explain why it is empty and what will fill it: `Nothing has changed yet. What Changed started recording at 1:20 AM.` or `No changes match these filters.` |
| Error | A sentence plus a retry, never a stack trace |
| Content | The normal case |
| Large data | Virtualised, with an honest count: `Showing 200 of 128,431` |
| Small window | Usable at 900x600: filters collapse into a dropdown, the drawer goes full width |

## Calm by default

The plan turns "calm" into numbers rather than a feeling. In the acceptance corpus, at least 80% of
events must classify as informational and no more than 2% as critical; a burst of near-identical events
must collapse into a single clustered row; a routine `bun install` inside a watched folder must produce
at most three events, not one per file.

## Not in the first release

Health checks and self-diagnosis, automatic update checking (a change recorder that promises no network
calls does not get to quietly contact an update server), command-line capture for processes, and a
signed installer. Each is recorded in [ROADMAP.md](ROADMAP.md) with its reason.
