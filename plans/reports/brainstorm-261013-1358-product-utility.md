# Product utility brainstorm — 2026-10-13

## Outcome

Turn What Changed from a passive local history viewer into a practical recovery and context tool: when something changes, the user understands the impact, can hand grounded evidence to an AI, and can follow safe next steps without unsupported causal claims.

## Confirmed core

- Local event history with redaction and no runtime network.
- Deterministic symptom investigation with confidence classes.
- Checkpoints and before/after comparison.
- Relationships between files, processes, ports, services, Docker and developer environment.
- Diagnostics/export and calm notification policy.
- No administrator, automatic machine mutation, cloud account or telemetry.

Current gap: value arrives mostly after failure. There is no useful AI handoff, project boundary or bounded recovery loop.

## Candidate directions

### A. AI Troubleshooting Pack — recommended first

One-click Copy context for AI from an investigation or event. Include symptom, time window, ranked evidence, before/after deltas, relationship edges, timestamps, confidence, blind spots and safe verification questions. Exclude secrets, event firehoses, unnecessary private paths and causal language. Clipboard plus local Markdown/JSON; no AI API call.

Value: works with ChatGPT, Claude, Copilot, local models and human support while keeping the deterministic engine authoritative.

Acceptance: one action from Investigation and Details; redaction tests; versioned schema; explicit evidence-not-cause wording; useful for empty and partial investigations.

### B. Workspace / Project Baseline

Named scopes such as My API or Data Science containing watched folders, expected ports/processes, Docker and a known-good checkpoint. Home offers What changed in this workspace?

Value: turns generic machine history into a daily developer/AI workflow and reuses existing checkpoints, relationships and investigations.

Risk: setup can become a second configuration system; expected state must remain descriptive, not an invented health score.

### C. Incident Session / Focus Mode

Start Watch this for 30 minutes. Mark a start checkpoint, isolate relevant new changes, collapse noise, mark recovery, and export the session plus evidence pack.

Value: useful during the actual debugging moment, not only afterward. Risk: another mode and notification surface.

### D. Safe Next Steps / Recovery Ladder

For each high-ranked change show Verify, Reproduce, Revert manually and Escalate. Copy-only actions with source, risk and confidence; never execute destructive changes.

Value: closes the gap between evidence and action for mainstream users. Risk: platform instructions can become stale or unsafe.

### E. Personal Change Digest

Opt-in local daily/weekly digest of three novel changes worth knowing about, with evidence links. Routine churn stays silent.

Value: creates a reason to open the app before failure. Risk: false positives and alert fatigue.

## Trade-offs

| Approach | Assumption | First failure mode | Cost |
|---|---|---|---|
| AI pack | Users already bring a symptom to AI | Pack is noisy or too technical | Low |
| Workspace | Users define a boundary | Setup feels like work | Medium |
| Incident session | User notices failure while app runs | User opens app too late | Medium |
| Recovery ladder | Instructions stay safe/current | Implied unsupported fix | Medium/high |
| Digest | Novelty ranking is trusted | Alert fatigue | Medium |

## Recommendation

Build a Recovery Loop in this order: (1) AI Troubleshooting Pack, (2) Workspace limited to one baseline checkpoint plus watched folders/ports/processes, (3) Incident Session using that boundary, (4) copy-only verification steps, (5) digest after real usage proves attention thresholds.

Do not start with an embedded LLM, MCP server, cloud sync, automatic rollback or broad system-health score. They add complexity or weaken the offline/safety contract before the core loop proves useful.

## Proposed first vertical slice

User creates My API and clicks Mark working. The app captures the boundary and checkpoint. After the API stops responding, the user clicks Investigate My API, sees ranked deltas and relationship chain, then clicks Copy context for AI. If unresolved, they start a 30-minute incident session and export a compact report for an AI or teammate.

## Non-goals

- Automatic rollback, registry/service edits or administrator access.
- Always-on cloud/telemetry/AI API.
- Full system-health score.
- Release or public-v1 checkbox closure.

## Future acceptance

- Useful result in under three actions from Home.
- Result contains evidence, relationships, uncertainty and a next step.
- Same output is useful to a general user, AI assistant and human support engineer.
- Secrets and unnecessary private paths are redacted before copy/export.
- Existing investigation/checkpoint semantics and offline guarantees remain unchanged.

## Unresolved questions

- Primary launch wedge: AI/developer workflows first, or mainstream Windows recovery first?
- Clipboard-only pack, local Markdown/JSON, or both?
- Explicit user-created workspace or inferred boundary?
- Which one verification action is safe enough for the first ladder release?
