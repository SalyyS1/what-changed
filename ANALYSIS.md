# Analysis

How What Changed decides which recent change is worth showing you first, and why
it will not tell you that change caused your problem.

Phase 15 is implemented. The accepted intent and gates remain in
[the phase record](plans/260829-0437-what-changed-desktop-mvp/phase-15-analysis-engine-investigation.md);
the current contracts are owned by the [pure engine](src-tauri/src/analysis/mod.rs),
[investigation service](src-tauri/src/services/investigation_service.rs),
[storage repository](src-tauri/src/storage/investigations.rs), and
[generated IPC bindings](src/lib/tauri/bindings.ts). Constants and wording below are navigation aids;
those executable owners decide current behavior.

## The claim it makes, and the one it refuses

You describe a symptom. The engine returns a ranked list of recent changes with
the evidence for each, in plain language. That is the whole contract.

It does not diagnose. A ranked list is an answer to *"what changed around then,
and which of those changes touches the thing you named?"* — not to *"what broke
your PC?"* Those are different questions, and the first is the one a local
change recorder can honestly answer.

This is not modesty for its own sake. A tool that says "the driver caused your
crash" is right sometimes and confidently wrong the rest of the time, and a
confidently wrong troubleshooting tool costs you more time than no tool. So the
strongest thing the product ever says is that a change *directly affected* the
thing you named, and it only says that when it can point at a recorded
dependency.

## No LLM required

The engine is deterministic: the same history and the same question always
produce the same ranking, byte-for-byte. There is no model, no network, no
scoring learned from data.

An optional AI layer may be added later to rephrase explanations. It will be off
by default, and it will only ever be handed facts the deterministic engine
already retrieved — it does not get to discover anything, and its output stays
visually separated from the evidence.

## How an investigation runs

1. **Resolve the window.** Explicit ranges are used as given. Otherwise a time
   hint in your wording is honoured ("this morning", "yesterday", "after the
   update"), and failing that the window is today.
2. **Find what you named.** Your text is matched against known entity names, and
   symptom words are mapped to categories through a small fixed lexicon:
   *microphone* reaches audio drivers, applications and services; *dns / wifi /
   proxy* reaches network configuration; *port / localhost / refused* reaches
   ports, processes and config files. This is keyword extraction, not language
   understanding — and when nothing matches, nothing is boosted. It never
   guesses at a category it did not find.
3. **Build the graph** over the events in the window, capped at 5,000. Beyond
   that, informational events are dropped first and the UI says so rather than
   pretending it analysed everything.
4. **Score each candidate** (below).
5. **Detect chains** — cascades of structurally connected changes, bounded to depth four and 200 paths.
6. **Rank**, keep at most twenty suspects, then explain.

Only four kinds of edge count as *structural*, and every one is something a
collector recorded rather than something the engine inferred: a process listening
on a port, an application providing a service, a container publishing a port, and
a config file belonging to the thing that reads it. Two events on the same entity
are structural too, trivially. Everything else — closeness in time, category
affinity — is circumstantial, and the confidence table below is what keeps the two
from being confused.

The config-file edge is worth naming separately because it is the one the product's
own worked example depends on: without it, "a port changed in a `.env`" is just a
file that happened to change near the symptom, and the strongest confidence class
would be unreachable for the exact case this tool was built for.

The service reads its event corpus, boundary context, entities, relationships and 90-day recurrence
counts inside one SQLite read transaction. That consistency boundary matters because collector writes can
continue while an investigation runs: a ranking must not combine events from one database moment with
relationships from another. The six command entry points are
`start_investigation`, `get_investigation`, `list_investigations`, `delete_investigation`,
`analyze_window`, and `get_related_events`; their registration and frontend parity are build-gated.

The event-details neighbour path scans the bounded incident window and its recorded relationships instead
of materializing the complete pairwise graph: O(n + relationships) incident-neighbour construction before
the bounded result is ordered. The owning source is
[`graph::neighbour_ids`](src-tauri/src/analysis/graph.rs).

## Scoring

Before investigation ranking, ingestion assigns each event a calm display
importance from an explicit fact table in `domain/importance.rs`. Routine
application updates are always informational. Failed updates, relevant
GPU/audio/network/storage/input driver updates, network configuration changes
(including proxy and DNS),
automatic-start services stopping, and checkpoint-known ports closing are
important; an unexpected shutdown is critical. These labels control attention
and notification eligibility, while the causal score below remains a separate
evidence calculation. A deterministic 2,000-event day guards the product's calm
default: at least 80% informational, at most 2% critical, and every routine
application update informational.

Six terms, summed with fixed weights:

```
score = 1.00 · temporal
      + 3.00 · relationship
      + 1.50 · category
      + 1.00 · importance
      + 1.25 · rarity
      + 2.00 · failure_proximity
```

| Term | What it measures |
|---|---|
| `temporal` | how close before the symptom the change happened — decaying over about an hour and a half, and **zero for anything after the symptom** |
| `relationship` | whether the change touches an entity the symptom named: full weight if directly connected, less at two hops, nothing otherwise |
| `category` | whether the change's category is one the symptom words point at |
| `importance` | the event's classified attention level: informational, notable, important or critical |
| `rarity` | how unusual this change is — a first-ever driver update scores full; an application that has updated eighteen times in ninety days scores almost nothing |
| `failure_proximity` | whether a crash, service failure or unexpected process exit followed this change *and* shares an entity or a dependency with it |

Two of those deserve their weights explained, because they are what make the
ranking useful rather than merely chronological:

**`relationship` is weighted highest (3.0)** because a recorded dependency is
the only evidence in the system that is not a coincidence. Everything else is
circumstantial by construction.

**`rarity` exists to demote the update treadmill.** Your browser updates
constantly. On any given day it is one of the most recent changes on the machine,
and on almost every one of those days it is irrelevant. Recency alone would put
it at the top every time.

**A change after the symptom scores zero on time and can never rank.** It cannot
have preceded what you noticed. This is a hard rule with its own test, not a
weighting.

**Chains get a bonus, and are presented as one suspect.** When a cascade is
detected, the head of the chain is boosted (0.75 per additional link, capped at
+2.25) and the rest of the chain is attached to it as steps. A three-step
cascade should read as one story with three moments, not as three suspects
competing for your attention.

Ranking is total-ordered and stable: score, then severity, then time, then id.
Two runs over the same data produce identical output — asserted, not assumed.

## Confidence

Three classes, computed from the *evidence*, never from the score:

| Class | Requires | What it says to you |
|---|---|---|
| **Confirmed** | a structural chain reaching what you named, **and** a corroborating failure signal | "This change directly affected \<thing\>." |
| **Likely** | a structural connection, without that corroboration | "This change likely affected \<thing\>." |
| **Correlated** | timing and category only | "This changed around the same time." |

Deriving the class from evidence rather than from the total is deliberate. A
score can be inflated by three weak circumstantial terms agreeing; a recorded
dependency cannot. So a change can rank first and still be labelled
*Correlated* — and that combination is honest, not a bug.

The copy is selected by class through a single constructor, so a `Correlated`
suspect physically cannot render `Confirmed` wording. On top of that, a test
generates explanations across hundreds of fixture suspects and fails if any
non-`Confirmed` explanation contains "caused", "because of", "the problem is",
"this broke", "definitely" or "root cause".

## What you see

Each suspect shows its plain-language title, when it happened relative to your
problem ("34 minutes before you noticed the problem"), the confidence sentence,
and a *why this ranked here* list built directly from the score terms that
contributed:

```
Happened 34 minutes before the problem
Changed a file that this service reads
First time this driver changed in 90 days
```

That list is generated from the same numbers the ranking used, so the
explanation cannot disagree with the ranking. Showing the reasoning is the point:
a ranked list you cannot audit is just an opinion.

Where a next step is genuinely mechanical, a fixed advice table supplies one
("Check your proxy setting under Windows Settings → Network → Proxy"). The table
contains advice text only. The app never changes your configuration.

The unsent problem draft remains memory-only because it may contain a pasted credential. Starting an
investigation sends it through the backend sanitizer; only the sanitized completed result is persisted.
The selected window may be remembered separately because it contains no problem text. This boundary is
owned by the [investigation store](src/lib/stores/investigation.svelte.ts).

Completed investigations are stored as immutable, sanitized presentation snapshots with their score audit
rows. They can therefore be reopened after their source events expire without silently changing rank or
wording. Investigation history has its own fixed 90-day retention class, participates in delete-all, and
can be deleted individually. The persistence contract and retention test live in
[`storage/investigations.rs`](src-tauri/src/storage/investigations.rs).

## When there is no answer

If the best candidate does not clear a minimum score, you get no suspects and a
reason instead of a guess:

- Nothing meaningful changed in this window.
- Only routine changes here — updates and background tasks.
- Try widening the time range.
- We started watching after this window. Try a more recent range.

Saying nothing is always preferred to promoting the least-bad candidate. The
first time this tool points at an innocent change is the last time you believe
it.

## Uncertain timestamps

The app is not always running. A change made while it was closed is detected on
the next start, and its true time is unknown — somewhere between the last
shutdown and now. Those events are marked, and the engine treats the mark as
real uncertainty rather than scoring them as if they happened at the moment they
were noticed. Reasoning over a fabricated timestamp would corrupt exactly the
term the ranking leans on most.

Concretely, a marked event scores zero on the timing term — and is also exempt
from the rule that discards anything recorded after the symptom. Both follow from
the same fact: the stored time is when we *noticed*, so it can neither earn
recency credit nor rule the change out by ordering. Such an event can still rank
on a recorded dependency, on category, on importance, and on rarity, which is the
point — a change made while the app was closed is often exactly what the user is
asking about. What it can never be is *Confirmed*, because that class requires a
failure signal within a time window, and there is no trustworthy time to measure
the window from. Its explanation says it was detected when the app started,
rather than inventing "34 minutes before".

## Tests that are allowed to fail

The scoring model is only trustworthy if its tests can catch a bad rebalance.
Two are load-bearing:

**The discriminating fixture.** A configuration change that moves a port, then a
service restart, then the old port closing and a new one opening — must outrank
an unrelated browser update **even when the browser update is moved to two
minutes before the symptom and the chain is forty minutes earlier**. A fixture
where the right answer is also the most recent event proves nothing. This one
fails if relationship and rarity stop dominating raw recency.

**The worked scenarios**, each with a declared expected top three: a driver
update against an unrelated install; a proxy change against a service restart;
two competing drivers where only one is on the symptom's device; a window
containing nothing but routine events (expect *no cause*); an oversized window
(the planted cause must survive downsampling); a cascade whose head falls outside
the window (expect *Likely*, never *Confirmed*); and an event after the symptom,
which must never appear at all.

## Boundaries

The engine takes a list of events and a list of relationships, and returns a
result. It imports nothing from the collectors and nothing from the platform
layer — a rule with its own architecture test. That is what makes it testable
from fixtures, and it is why the ranking logic can be reasoned about without a
Windows machine in the loop.

The explicit performance gate in
[`analysis_perf.rs`](src-tauri/tests/analysis_perf.rs) passed both budgets: a 5,000-event pure-engine
window at p95 ≤ 400 ms and a graph-rich 100,000-row database investigation at p95 ≤ 1.5 s. The
passing run did not retain narrower p95 values, so none are claimed here.
