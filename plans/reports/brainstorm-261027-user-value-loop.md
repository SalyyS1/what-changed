# Product direction: the user value loop — 2026-10-27

## Outcome

Make What Changed worth installing before a failure and worth reopening when a
failure happens. The product should turn important Windows changes into plain
language receipts, preserve a user-selected known-good state, and produce a
privacy-safe evidence handoff for support or an AI assistant.

## Current evidence

- The repository already has the difficult trust layer: local history,
  redaction, deterministic ranking, checkpoints, relationships, workspaces and
  an offline AI context pack.
- The current home flow is symptom-first. A user has to experience a problem
  before the product's strongest evidence becomes visible.
- “Workspace”, “collector”, and “Mark working” describe implementation rather
  than a job a normal user is trying to complete.
- The app is not a good fit for a daily dashboard habit. Troubleshooting is
  episodic, so passive capture and short change sessions must carry retention.

## User jobs

1. “Something changed on my PC and I need a trustworthy explanation.”
2. “I am about to install or update something; I want a before/after receipt.”
3. “I need to hand support or an AI assistant useful context without exposing
   secrets or dumping raw logs.”

The first launch audience should be Windows developers, creators, gamers and
technical users who install software, drivers, plugins or updates often. The
general user is a secondary audience reached through the plain-language
incident flow and support handoff.

## Options considered

### A — Troubleshooting utility

Keep the current symptom-first product as the front door.

- Strength: strongest pain when something is broken and little new architecture.
- Weakness: users must suffer first; “AI troubleshooting pack” sounds like an
  export feature rather than a product.
- First failure: most installs never reach the first investigation.

### B — Change receipts and sessions (recommended)

Make “what changed since a known-good state?” the front door. Let users start a
short session before an install, update, driver change or project change and
show one grouped before/after receipt.

- Strength: immediately understandable, creates value before failure and uses
  existing snapshots, events and relationships.
- Weakness: grouping and importance thresholds must avoid noisy receipts.
- First failure: a receipt that lists rows instead of explaining a change story.

### C — General PC health and safety monitor

Lead with alerts, risk scores and a daily digest for all Windows users.

- Strength: larger apparent market and more frequent surface area.
- Weakness: overlaps antivirus, optimizers and monitoring tools; false
  positives erode trust and conflict with the no-mutation contract.
- First failure: alert fatigue or unsupported “health” claims.

## Recommendation

Use B as the product front door, A as the deep investigation path and defer C
until real usage proves notification thresholds. Position the app as:

> Every important change on your PC gets a receipt. When something feels wrong,
> compare it with the last good one.

AI is an optional interpreter and handoff channel, not the reason to install the
app. The defensible asset is the longitudinal, local, redacted evidence trail.

## Required product loop

1. **Onboarding:** automatically capture a clearly named “Known-good state” and
   show real recent changes; do not begin with a collector configuration grid.
2. **Passive value:** Home opens to “Since your last visit” with three grouped
   change stories and a plain-language reason each may matter.
3. **Change session:** “Start a change session” → choose a label such as
   “installing an app” or “Windows update” → finish → receive a before/after
   receipt.
4. **Incident:** “Something feels wrong” → select the known-good state or time
   window → review ranked evidence and relationships.
5. **Handoff:** “Get help” shows a privacy preview, then copies a bounded
   Markdown pack for an AI assistant or human support.
6. **Trust:** no automatic fixes, deletion or causal claims; every suggestion
   has evidence, confidence and a safe verification step.

## Language and UX changes

- “Mark working” → **Save as known-good**.
- “Workspace” → **Project watch** or **Project** in user-facing copy.
- “AI Troubleshooting Pack” → **Get help**; explain that it creates a clean
  evidence handoff for any AI or support person.
- Make maps and raw collector status secondary. The primary unit is a grouped
  change story with “what changed”, “when”, “why it may matter”, and “review”.
- Keep notifications opt-in and grouped. A quiet, useful weekly digest is
  safer than a stream of low-confidence alerts.

## First implementation slice

1. Build a Change Inbox on existing dashboard queries: grouped stories since
   last visit, with important-only filtering and dismiss/ignore state.
2. Rename the checkpoint action and make the first known-good capture automatic
   after onboarding.
3. Add Change Session start/finish around existing checkpoint and comparison
   services; render a compact receipt and copy it to the clipboard.
4. Move the current investigation and AI pack behind the receipt actions.

## Success evidence

- A fresh user sees one real, understandable change story within 60 seconds.
- A user can save a known-good state or complete a change session in three
  actions or fewer.
- A session receipt explains a grouped change without raw-log dumping or causal
  overclaiming.
- From “something feels wrong” to a useful evidence pack takes under two
  minutes on a normal local history.
- Repeat usage comes from sessions and meaningful digests, not forced daily
  notifications.

## Non-goals

- Embedded chatbot or cloud AI API.
- Antivirus, registry cleaner, generic health score or auto-fix.
- Automatic rollback or administrator access.
- Account, telemetry or cloud sync.

## Unresolved questions

- Which first session label should be shipped: app install, Windows update or
  project change? The existing collector coverage can support all three, but one
  should be the onboarding example.
