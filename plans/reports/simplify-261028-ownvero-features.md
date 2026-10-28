# Ownvero feature simplification review

Status: DONE

Scope: read-only review of the newly added preview-runner UI/store, owner graph and map code, release/runner checks, and wiki source requested by the controller. README and the owning Ownvero phases were read first. No source, test, build, or plan-status changes were made.

## Concrete simplifications

1. `src/lib/owner-graph/projection.ts:21-48` — derive relationship indexes once after the rule/scenario maps are built. The current capability and rule loops repeatedly materialize `rules.values()` / `scenarios.values()` and rescan the full collections for `hasRules`, selected preserved rules, scenario coverage, and observed scenario revisions. Build small sets keyed by capability/rule/scenario revision (for example capabilities with rules, capabilities selected by a preserved rule, covered rule node IDs, and observed scenario node IDs), then use `Set.has` in node construction. This removes nested scans and temporary arrays while making the exact-revision relationship rules more explicit. Keep the existing node/edge construction, gap text, ordering, freezing, backend status, freshness, and review references unchanged.

2. `src/lib/owner-graph/comparison.ts:10-22` — index each attempt's scenario rows by scenario ID before iterating the sorted union. Replace the per-ID `before.scenarios.find(...)` and `after.scenarios.find(...)` calls with two `Map<string, OwnerScenarioEvidenceDto>` lookups. This preserves sorted output and every comparability reason, but removes quadratic lookup behavior and makes the one-row-per-scenario assumption visible. Keep `ruleContent` based on each frozen manifest; do not normalize or weaken the exact revision/recipe/rule-content comparisons.

3. `scripts/check-runner-bundle.mjs:42-44` — retain one `lstat(path)` result per inventory entry. The same path is currently statted twice in consecutive checks. Store the result, use it for regular-file/symlink and byte-size validation, then continue with `realpath` and SHA-256 exactly as now. This is a direct reduction in filesystem work without changing the fail-closed ownership or inventory checks.

## Intentionally retained

- The generation checks and late-grant cancellation in `src/lib/stores/preview-runner.svelte.ts` are lifecycle/authority guards, not removable duplication.
- Separate consent acknowledgements, explicit route scope, screenshot review, backend verdict/freshness axes, provenance details, 2D/list fallback, and renderer cleanup are contract-bearing and should not be collapsed for brevity.
- No concrete simplification was found in the wiki templates that outweighed the readability cost of another abstraction; the synthetic demo's repeated lookups are bounded to six nodes.

## Verification

Not run by instruction: controller already owns tests/build. Recommendations were checked against the focused projection, comparison, layout, runner-bundle, and preview-runner tests only by source inspection.

Summary: three behavior-preserving edits remove repeated collection scans or filesystem calls. No consent, freshness, authority, provenance, release-gate, or renderer-lifecycle invariant needs changing.

Unresolved questions: none.
