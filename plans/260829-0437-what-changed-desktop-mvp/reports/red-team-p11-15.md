# Red team — phases 11-15

Provenance: the `## Working notes` sections below are the `rt-p11-15` subagent's raw output. It covered
phases 11 and 12 and then died on a stream-idle timeout before reformatting them or reaching 13-15. The
notes are left verbatim — they are legible and reformatting them would risk distorting them. The
controller section that follows completes the slice.

Its findings N1 and N8 turned out to be two halves of one structural defect, written up as F-C1 below.

<!-- WORKING NOTES (reordered into the final report at the end of this run) -->

## Working notes — phase 11

- N1 [HIGH] first-cycle baseline emits zero events, in-memory diff state only -> every service/app
  change that happens while the app is not running is silently lost. Fix: diff first cycle against
  persisted entities.
- N2 [HIGH] NotifyServiceStatusChangeW: APC/alertable-wait + one-shot re-registration unstated; risk
  signal `try_register_notify() returns false` cannot see the real failure (registers, never fires).
- N3 [MEDIUM] manual step 7 `net stop`/`net start` needs admin; named fallback (Docker service) is
  measured absent (matrix #35).
- N4 [MEDIUM] `SERVICE_DRIVER` in the enum filter contradicts the measured 301-row baseline and
  overlaps P12's driver collector.
- N5 [MEDIUM] `RegNotifyChangeKeyValue` omits `REG_NOTIFY_THREAD_AGNOSTIC` (named in research §5 row
  3); timeout==fallback means one dead thread kills detection with no health signal.
- N6 [?] application->service relationship (P15's headline chain) has no success criterion; depends on
  `InstallLocation`, whose presence was never measured.

## Working notes — phase 12

- N7 [HIGH] `CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE` without `ALL_INTERFACE_CLASSES`: registers, never
  fires; risk signal is "registration returns non-zero"; "three tiers each independently sufficient"
  contradicts research §5 row 5 (arrival/removal != version change).
- N8 [HIGH] `collector_state` row + `storage/collector_state.rs` attributed to P04; P04 creates
  neither; listed under **Modify**; restart-no-replay criterion rests on it.
- N9 [MEDIUM] driver events have no `occurred_at` source; 6 h sweep timestamps them at detection ->
  P15 pre-symptom window cannot see them.
- N10 [MEDIUM] WUA field list omits `HResult`/`UnmappedResultCode` yet the mandated summary and a
  success criterion embed `error 0x80070002`.
- N11 [MEDIUM] ±10 min WUA/event-log corroboration needs P13's subscription; `dependencies: [9, 11]`.
- N12 [LOW] "p95 over 5 sweeps" = max; 6 h vs research's "hourly" fallback suggestion.

## Controller findings

### F-C1 — the flight recorder is blind to everything that happens while it is not running  [CRITICAL]
- **Where:** `phase-11-service-application-collectors.md:51` and success criterion `:111`;
  `phase-12-update-driver-collectors.md:78,88,102`; contrast `phase-10-file-config-collector.md:41,61,95`
- **Category:** the product premise, plus a missing dependency
- **Defect:** two phases handle the app's own downtime in contradictory ways, and neither works.
  - **P11 keeps its baseline in memory only.** `:51` — "on app start, the first cycle of each collector
    is a baseline only (no events)", and `:111` makes that a gate: "First cycle after start emits zero
    events for both collectors". Nothing in P11 reads or writes a persisted set. So a service installed,
    removed or switched to `Disabled`, and an application installed, uninstalled or upgraded, **while the
    app is closed, is never reported at all** — not late, not at low confidence, never.
  - **P12 recognised the problem and reached for storage that does not exist.** `:78` — collectors
    "persist their last-seen set in a `collector_state` row (P04) so a restart does not re-emit history".
    P04 creates `collector_status` (`phase-04:28,101`), a per-collector *health* row from §10's table
    list — not `collector_state`. And `phase-12:88` lists `src-tauri/src/storage/collector_state.rs`
    under **Modify**, so an implementer following the plan literally opens a file no phase creates.
    P12's restart criterion (`:141`), its `collector_state_restart.rs` test (`:86`) and its DB-reset risk
    row (`:167`) all rest on that missing table.
  - **P10 already solved this correctly**, which is what makes the gap indefensible rather than merely
    unlucky: `config_snapshots` with `storage/config_snapshots.rs` (baseline get/put), redacted at write
    time, `delete(path)` on unwatch. The pattern exists in-plan and two phases did not adopt it.
- **Why it bites:** this is the product. "What Changed" is sold as "a black box / flight recorder for a
  PC" whose one job is answering "It worked before. What changed?". The single most common real scenario
  — user reboots, an installer or Windows Update changes a service, a driver, or an app version during
  that window, and the user then finds something broken — is precisely the scenario the app cannot see.
  It will show an empty timeline for the interval that matters and the user will conclude nothing changed.
  A recorder that is silent about its own downtime is worse than no recorder, because it is trusted.
  P09's in-memory baseline is *correct* by contrast — process and port state is genuinely ephemeral and
  has no meaning across a restart — so this is not a blanket problem, which is exactly why it will be
  missed in review.
- **Fix:** three parts, and the first is the load-bearing one.
  1. Give the persisted baseline a real owner. Add a `collector_baselines` table (or extend
     `collector_status` with a `last_seen_set` JSON column) to **P04's** migration, since P04 owns the
     schema, and move `storage/collector_state.rs` from P12's **Modify** to P04's **Create** under a name
     that cannot be confused with `collector_status`.
  2. Change the first-cycle contract from "emit nothing" to "diff against the persisted set". Storm
     control survives — a restart with no real change diffs to zero events, which is what
     `baseline_no_storm.rs` should actually assert — while a genuine downtime change is now caught.
  3. Timestamp those events honestly. They were detected at startup but occurred at an unknown point
     between last shutdown and now, so they need an explicit `occurred_at_is_uncertain` marker and UI
     copy to match ("detected when What Changed started; the change happened while it wasn't running").
     Without that, P15 will place them at detection time and reason about the wrong interval — which is
     rt-p11-15's N9 for drivers, the same defect one layer down.
  4. Apply the same review to P13's network and startup sweeps, which were not examined here.

### F-C2 — `collector_state` vs `collector_status`: a one-word difference that breaks three artifacts  [HIGH]
- **Where:** `phase-12-update-driver-collectors.md:78,86,88,102,141,154,167` vs `phase-04-domain-model-persistence.md:28,101`
- **Category:** missing dependency
- **Defect:** the mechanical half of F-C1, recorded separately because it can be fixed independently and
  because the near-identical names are why it survived five candidate plans and a verifier. §10's table
  list contains `collector_status`; P12 needs a different thing and calls it `collector_state`. A reader
  skims the two as the same word.
- **Why it bites:** `cargo test --test collector_state_restart` is in P12's verification gate. It cannot
  compile: the module is under **Modify**, so it is never created, and the table is not in any migration.
  The failure arrives at the end of a 7-hour phase, in the test that proves the phase's headline claim.
- **Fix:** rename in P12 to something unmistakable (`collector_baselines`), create it in P04 per F-C1
  step 1, and move the module to a **Create** entry in whichever phase owns it.

### Phases 13-15 — not reviewed in this slice
Phase 13 and 14 received no adversarial pass; phase 15 is under review by a dedicated single-phase agent
(`rt-p15`) because it carries the most weight in the plan. Known items for 13-14 from other reports:
phase 13's §22 mis-citation (`red-team-coverage.md` Y-4), phase 13's and phase 14's §48 mis-citations
(Y-6), and phase 14's unsatisfiable "P09–P13 suites still green" claim at its declared graph position
(`whole-plan-consistency-gate.md` C-3). F-C1 step 4 flags P13's sweeps as unexamined.

Status: DONE_WITH_CONCERNS
Summary: 12 subagent findings across phases 11-12 plus 2 controller findings, one CRITICAL — the app
cannot report service, application, update or driver changes that occur while it is not running, and the
one phase that tried to fix that points at a table no phase creates.
Concerns: phases 13 and 14 were not adversarially reviewed in this slice; phase 15 is delegated to
`rt-p15`.

