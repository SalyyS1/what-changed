# Ultra Verifier Verdict — `ak:plan --ultra`, "What Changed"

Five candidates were scored blind against `ultra-verifier-rubric.md`, which was fixed before any
candidate was read. Candidates are referred to as **A–E** throughout; the dispatch-slot numbering in
the source filenames carries no information and is not used or reproduced here.

**Winner: D.** No reject-all condition was met.

---

## 1. Hard constraints

| Candidate | H1 Electron | H2 Go/sidecar | H3 stack swap | H4 Rust install early | H5 invented API/version | H6 privilege claims | H7 generic IPC | H8 permanent elevation | H9 silent drops | H10 §5 as fact |
|---|---|---|---|---|---|---|---|---|---|---|
| A | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| B | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| C | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| D | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS |
| E | PASS | PASS | PASS | PASS (note) | PASS (note) | PASS | PASS | PASS | PASS | PASS |

No violation was found in any candidate. Two notes that were checked and cleared rather than failed:

- **E, H4.** E's phase 01 is documentation/repo genesis; the toolchain install lands in phase 02. It
  still precedes every phase that compiles anything, so the install is an explicit early step and H4
  holds. It is a sequencing weakness, scored under C9, not a constraint breach.
- **E, H5.** E specifies the driver collector as SetupAPI (`SetupDiGetClassDevsW` +
  `SetupDiEnumDeviceInfo` reading `DEVPKEY_Device_DriverVersion` …), annotated "probe #10 PASS
  non-elevated, 2128 ms warm". The APIs are real and the mechanism is the one research §3 recommends
  ("prefer SetupAPI from Rust over WMI"), but 2128 ms was measured for `Win32_PnPSignedDriver`
  (matrix row 29), not for SetupAPI, and "probe #10" is not that row. This is citation drift that
  over-budgets the cost — conservative in effect — so it is scored under C2/C10, not H5.

All five explicitly forbid the three measured traps rather than tripping them. Representative:
A's global rule — "`Win32_ProcessStartTrace`, the `Security` event-log channel, and
`Win32_QuickFixEngineering` as an update source are forbidden"; C's gate — "`Win32_QuickFixEngineering`
appears nowhere in the codebase (grep-asserted)"; B — "the matrix measured 4 rows from QFE versus 133
from WUA `QueryHistory` … so QFE would silently under-report by 97 %". All five keep the ~2.1 s driver
sweep off short timers (E: "an hourly sweep, never a timer"; C: "the driver sweep costs ~2.1 s and
therefore must never sit on a short timer").

---

## 2. Scores (1–20 per criterion)

| Criterion | A | B | C | D | E |
|---|---|---|---|---|---|
| C1 Requirement fidelity | 15 | 18 | 18 | **19** | 18 |
| C2 Technical correctness | 16 | 17 | 17 | **18** | **18** |
| C3 Executability | 17 | 17 | 18 | **19** | 18 |
| C4 Verification rigor | 17 | 17 | 19 | **20** | 19 |
| C5 Architecture quality | 17 | 18 | 18 | 18 | **19** |
| C6 Security & privacy | 18 | **19** | 18 | **19** | **19** |
| C7 Risk realism | 18 | 18 | 18 | **20** | 18 |
| C8 Product/UX substance | 16 | 17 | 18 | **19** | 18 |
| C9 Sequencing & reviewability | 17 | **18** | 16 | 17 | 16 |
| C10 Plan-format compliance | **19** | 17 | 17 | 15 | 9 |
| **Total** | **170** | **176** | **177** | **184** | **172** |

---

## 3. Ranking

### 1st — D (184)

D is the only candidate whose mandated §59 fixture can actually fail if the weights are wrong: it
requires that "the config→restart→port chain must outrank a Chrome update **even when the Chrome update
is moved to 2 minutes before the symptom and the chain is 40 minutes before**", and its published
weights (`W_RELATIONSHIP = 3.0`, `W_RARITY = 1.25`, chain-head bonus up to `+2.25`, `temporal =
exp(-minutes/90)`) are numerically consistent with that outcome — the test discriminates rather than
passing on recency. Its eight ranking scenarios are written **first** ("They are the spec; the engine is
written to satisfy them") and include the cases the others omit: an event after the symptom that "must
never rank", and a cascade whose head is outside the window that must be `Likely`, "never `Confirmed`".
Its closing carries an eleven-row ledger covering every research §5 unverified assumption with a settling
phase, a falsification signal and a pre-decided response, and its B2 entry names the specific cheat it is
forbidding: "Never silently redefine the target, never disable collectors to make the number." Its one
real weakness is format — the plan body runs ~137 lines against the ~120 guidance.

### 2nd — B (176)

B tracks the normative spec most literally of the five: the §27 eight stages in order, the exact §28 sum,
the §29 `Confirmed / Likely / TemporalCorrelation` tiers, §30 edge types, and the §32 provider seam with a
fact-only context builder so a future provider "can only ever see deterministically retrieved facts". Its
chain rule — "A chain's confidence is its **weakest** link, which prevents one confirmed hop from
laundering a coincidence into a claim" — is the sharpest single honesty mechanism in the set. On privacy
it is the only candidate that blocks *acquisition* rather than only persistence: `guard-redaction.ts`
fails if `collectors/`/`platform/` reference `.cmd()`/`.environ()`/`GetCommandLine`, and
`privacy/policy.rs::NEVER_COLLECTED` renders into both `PRIVACY.md` and the Settings screen "so code and
copy cannot diverge". It is also the only plan that reasons about rollback and file ownership. It loses
points because its phase files are not fenced (a materialization hazard for the controller) and its §70
six-state enforcement is a review checklist rather than a mechanical artifact.

### 3rd — C (177 by total; placed 3rd on the rubric's tie-break)

C has the broadest test surface of any candidate: the §59 fixture plus a §73 NVIDIA fixture asserting
"rank #1, `Confidence::High`, and that the rendered explanation contains the driver version transition",
plus a negative control that must return "We didn't find anything that clearly explains this.", plus a
determinism test that runs "twice with a shuffled retrieval order", plus an assertion that contributions
sum to the total "for 500 generated cases". Its redaction gate adds a 10 000-iteration generated-secret
loop and a `SAFE_KEYS` allowlist so §13's `API_PORT 3000 → 4000` survives redaction. Its total is one
point above B, which is inside my measurement error, so the rubric's tie-break decided the order:
on C2+C6 B scores 36 to C's 35. The substantive reason to prefer B's honesty here is C's B2 stance — "M1
is the number this plan commits to (<80 MB backend RSS); M2 is reported to the user as-is" — which is a
disclosed but real narrowing of the user's metric, where B and D commit to the whole figure and escalate.
The 2nd/3rd order does not affect the winner.

### 4th — E (172)

E is the strongest candidate on architecture and on determinism analysis, and it is beaten mainly by its
own format defect. Its four invariants each have a gate in one `architecture-gates.rs` (sealed write path,
redacted logging, platform isolation, IPC allowlist); its redaction has two independent exits (manual
redacting `Debug` impls plus a `RedactingLayer`), so a stray `{:?}` cannot leak; and it is the only
candidate to eliminate the real sources of nondeterminism by name — "no `HashMap` iteration order in any
ranked output (`BTreeMap`/sorted vectors only) … no floating-point summation order ambiguity" — verified
over 50 runs plus a shuffled insert order. It also uniquely flags a live hazard on this machine
("`/usr/bin/link.exe` shadows the MSVC linker on this PATH … hazard confirmed present") and carries risks
for FTS5 in bundled SQLite and NSIS per-user uninstall hooks that no one else raises. Against that: it is
missing its `## Unresolved questions` section and its `Status:` line entirely, left an internal
`<!--APPEND-->` placeholder in the deliverable, self-reports spec-citation errors in its own
decomposition rationale, and defers its DB/log raw-byte leak scan to a later phase's gate.

### 5th — A (170)

A is well-built and honest — its `Sanitized<T>` chokepoint with a `pub(super) fn wrap` reachable only
from `redact/`, its raw-byte leak scan across the DB after `wal_checkpoint(TRUNCATE)`, the log file and
the diagnostics export, its `-products '*'` vswhere selection *without* `-prerelease` so the VS18
Insiders install is excluded plus a grep of `docs/link-gate.log` for the linker actually used, and its
global "API-shape rule" (read the vendored source under `~/.cargo/registry/src/`; "the vendored source
wins") are the best B1 and best leak-surface handling in the set. It ranks last because of where it
matters most under this rubric: its analysis engine substitutes its own signal list (temporal proximity,
category prior, severity, entity relevance, breadth, reversibility hint, noise suppression) for §28's
mandated six-term sum, dropping `rarity_score` and `failure_proximity_score` as named terms, and its
analysis phase's "flagship four" fixtures describe only "a watched `.env` edit before an app
misbehaving" — the port/config chain and the Chrome-update distractor appear only as a plan-level
acceptance line, not in the phase that must implement them. Its "`high` requires ≥ 70 *and* at least two
independent signals; asserted by a test that a lone temporal-proximity hit can never exceed `medium`" is
nonetheless the best confidence-inflation guard any candidate wrote.

---

## 4. Winner

**D.** Not a reject-all.

D wins on the four criteria the rubric's tie-break ranks highest, without needing the tie-break: it is
first or tied-first on C2 (technical correctness), C4 (verification rigor), C6 (security and privacy) and
C7 (risk realism), and also on C1, C3 and C8. The three things that decide it:

1. **Its mandated fixture can fail.** Every candidate carries the §59 ranking fixture. Only D hardens it
   against the trivial pass by moving the Chrome update to 2 minutes before the symptom while the real
   chain sits 40 minutes out, and only D's published weights are numerically consistent with the chain
   still winning. A fixture that cannot fail does not protect the product's core claim.
2. **Its risk register is falsifiable rather than narrative.** Eleven rows, one per research §5
   assumption, each with the phase that settles it, the observable signal that it is false, and the
   response decided in advance — including B2 measured at four points with "never disable collectors to
   make the number", which names the specific dishonest move a pressured executor would otherwise make.
3. **Its privacy chokepoint is the hardest to defeat by accident.** `Sanitized<T>` before persistence
   exists in all five, but D pairs it with the only serious false-positive discipline in the set (a
   40-benign-value fixture that "must never be weakened to make a pattern pass", an entropy rule that
   excludes pure hex and pure digits so hashes, versions, ports and KB numbers survive) and the strictest
   no-residue rule ("never a digest, never a salt, never a prefix"), with leak assertions that forbid any
   8+ character substring of a secret.

Its deficit is confined to C10: the plan body runs ~137 lines against the ~120 guidance. That is a
trimming task at materialization, not a defect in the plan's substance — the right weakness to accept in
a plan that must survive execution.

---

## 5. Note for the controller's red-team step

The winner is materialized unchanged; these are hints for later review, not edits.

**Primary — E's confirmed `/usr/bin/link.exe` MSVC-shadow hazard.** E is the only candidate that reports
this as verified on this machine rather than hypothetical: "`/usr/bin/link.exe` shadows the MSVC linker on
this PATH … Open, hazard confirmed present". D handles linker *ambiguity* well (vswhere selection plus a
`scripts/dev-shell.cmd` vcvars64 fallback as the documented build entry point) but that machinery assumes
the failure mode is "which MSVC install", whereas the actual first failure on this box is a Git Bash
coreutils `link` winning the PATH lookup before any MSVC install is consulted. Highest value-to-effort in
the set: it lands in phase 01, the phase every other phase depends on; the symptom (a cryptic link
failure that vswhere output says should be impossible) is expensive to diagnose and cheap to prevent; and
the fix is a single explicit `linker =` path or always entering through `dev-shell.cmd`, plus one line in
the phase-01 gate asserting the resolved `link.exe` is the MSVC one. A's `-products '*'` **without**
`-prerelease` (so the VS18 Insiders install is excluded) belongs in the same one-line hardening if the
reviewer is already in that file.

**Secondary — B's acquisition-blocking privacy guard.** D excludes process command lines deliberately and
surfaces the choice honestly as an unresolved question, but that exclusion lives in prose; nothing stops a
later collector phase from adding `GetCommandLineW` to improve the analysis engine, which is exactly the
temptation D's own unresolved question #5 documents ("would materially improve the analysis engine …
but are the single richest secret carrier"). B enforces the same policy mechanically — its guard fails if
`collectors/`/`platform/` reference `.cmd()`/`.environ()`/`GetCommandLine`. For D this is a few lines
added to the `tests/architecture.rs` it already runs per phase, and it upgrades D's one prose-only
privacy commitment to the same structural standard as the rest of its chokepoint.

---

## 6. Confidence note

### 6.1 What I actually read

Every candidate: `## Thesis`, `## Plan metadata`, the `## PLAN.MD BODY` in full, the phases table, and
the closing sections. Then the **analysis-engine phase in full in all five** — that is the one high-weight
phase where the comparison is genuinely like-for-like. Then one further implementation phase in full from
each candidate's foundation/redaction region, plus targeted grep windows (10–40 lines of context each) for
the redaction chokepoint, B2/memory, `Win32_QuickFixEngineering`, the driver-sweep cost, §28 term names,
`Chrome`/`4000`, §70 states and §41 tokens.

| Candidate | Continuous reads (line ranges) | File length | ≈ read |
|---|---|---|---|
| A | 1–141, 141–254, 613–733, 1894–2050, 2740–2763 | 2761 | ~23 % |
| B | 1–140, 140–254, 255–364, 1605–1750, 2311–2405 | 2403 | ~25 % |
| C | 1–158, 349–498, 2494–2685 | 3832 | ~16 % |
| D | 35–95, 494–642, 2521–2720, 3875–3990 | 3985 | ~18 % |
| E | 1–144, 743–848, 1882–2046, 3189–3258 | 3252 | ~18 % |

Grep-window-only coverage, on top of the above: C's B2/memory phase (~874–990), its update/driver phase
(~1985–2120) and **its closing register (~3676–3820)**; D's plan-body lines 53–68/135–139/167–168 and its
later phases at ~3384–3446, ~3517–3560, ~3609–3660; E at ~1531–1600, ~1649–1760, ~2666–2670, ~3238–3242.
The original three-phase strategy was therefore honored in full for one of the three phases (analysis),
partially for the second (one phase each, in full), and by grep only for the third. I did not do the two
additional continuous spot-check phases per candidate; grep windows replaced them.

### 6.2 What I could not check

- **Nothing was executed.** No Verification Gate command, no build, no test. C3 is a judgment about
  whether commands and paths are specific and plausible on this machine, not proof that they run.
- **H5 is "no violation in the pins I checked", not "no violation exists".** I spot-checked `sysinfo`
  0.39.6, `rusqlite` 0.40.2, `windows` 0.62.2, `tauri-specta` 1.0.2, `ts-rs` 12.0.1 against research §1.
  A full five-candidate × ~40-pin cross-check was not done.
- **H6 likewise.** I grepped the three measured-FAIL mechanisms and the QFE row and read the surrounding
  lines in all five. I did not audit all 35 matrix rows against all five candidates' privilege claims.
- **H9 was sampled, not audited.** §-coverage was checked through the phases tables, plan-body traceability
  and greps for high-risk sections — not by walking all 91 sections against five phase inventories.
- **Effort arithmetic was verified for D and B only.** A's 187 h, C's 254 h and E's total are as stated.
- **UI, timeline, visualization, Docker, onboarding and installer phases were not read in full for any
  candidate.**

### 6.3 Where the scores are least reliable

Ordered worst first.

1. **C1 (requirement fidelity)** — the weakest score in the table. It rests on sampling, per 6.2. A
   candidate could be missing a mid-numbered section I never grepped for and still score 18 here.
2. **C8 (product/UX substance)** — the UI phases were not read for anyone. C8 rests on plan bodies, phase
   tables, and §70/§41 grep sampling. Treat the C8 spread (16–19) as roughly one band wide, not four points.
3. **C9 (sequencing & reviewability)** — phase sizing was judged from phase titles, descriptions and stated
   effort, and effort was verified for only two candidates. "24 thin phases vs 18 executable ones" is a
   judgment from the phase inventories, not from reading all 24.
4. **C7 (risk realism)** — reliable for the closing registers, which I read in full for A, B, D and E, but
   **C's closing register was seen only through grep windows**, so C7 = 18 for C is the least evidenced
   single cell in the table.
5. **C2, C4, C6** — my most reliable scores, because the analysis-engine phase and the redaction surface
   were read continuously in all five and cross-checked against the research report.
6. **C10** — mechanically checkable from headings and line counts; high confidence.

### 6.4 Scores inferred from a plan-body claim rather than the implementing phase file

Stated explicitly, as asked:

- **A, C1/C4.** A's §59 credit exists at plan.md line 106. Its analysis phase (1894–2050) names only "a
  watched `.env` edit before an app misbehaving" as its fixtures. I resolved the contradiction in favor of
  the phase file, which **cost A points**. If the port/config chain is implemented in a phase I did not
  read, A's C1 and C4 are each understated by roughly 2.
- **C, C7/C10 and part of C6.** C's B2 stance ("M1 is the number this plan commits to … M2 is reported to
  the user as-is") and its release/audit handling came from grep context windows, not a continuous read.
  This is the weakest evidence behind the 2nd/3rd ordering — which is precisely why I placed that ordering
  on the rubric's C2+C6 tie-break rather than on the totals.
- **D, part of C8.** D's six-state and microcopy credit comes from its plan body plus grep windows into its
  later phases (~3384–3446, ~3517–3560, ~3609–3660), not from full reads of those phases.
- **D, the decisive weight-consistency claim in §4.** I checked D's weight constants and `exp(-minutes/90)`
  against the hardened fixture by hand, from the constants as written at 2521–2720. I ran no code. If those
  constants are redefined elsewhere in D's file, that check is void — though the fixture would then simply
  fail at execution, which is the behavior the rubric wants.
- **E, C10.** The missing `## Unresolved questions` and `Status:` line were confirmed by heading grep plus
  reading E's final lines — reliable. E's *self-reported* spec-citation errors are E's own claim about
  itself; I did not re-verify each citation against the packet.

### 6.5 Margins

Only one gap in the ranking exceeds my measurement noise: **D's 7 points over C and 8 over B.** Everything
else should be read as a tie:

- **C 177 vs B 176** — one point, inside measurement error; ordered by the mandated tie-break, not by total.
- **E 172 vs A 170** — two points, also inside error. The 4th/5th ordering is not a reliable result; what
  is reliable is *why* each lost (E: format defects; A: the §28/§59 substitution).
- **D's win does not depend on the tie-break** and does not depend on any single cell: removing D's best
  criterion still leaves it first. That is the claim I hold with the most confidence in this report.
