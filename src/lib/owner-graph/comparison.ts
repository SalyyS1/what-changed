import type { OwnerEvidenceReviewDto, OwnerScenarioEvidenceDto } from '../tauri/bindings';

export interface ScenarioComparison {
  readonly id: string;
  readonly before: OwnerScenarioEvidenceDto | null;
  readonly after: OwnerScenarioEvidenceDto | null;
  readonly comparable: boolean;
  readonly reasons: readonly string[];
}
export function compareOwnerRuns(before: OwnerEvidenceReviewDto, after: OwnerEvidenceReviewDto): ScenarioComparison[] {
  const ids = new Set([...before.scenarios, ...after.scenarios].map((row) => row.scenario.id));
  return [...ids].sort().map((id) => {
    const old = before.scenarios.find((row) => row.scenario.id === id) ?? null;
    const next = after.scenarios.find((row) => row.scenario.id === id) ?? null;
    const reasons: string[] = [];
    if (!old || !next) reasons.push('Scenario is absent from one frozen attempt.');
    else {
      if (old.scenario.revision !== next.scenario.revision) reasons.push(`Scenario revision changed: ${old.scenario.revision} → ${next.scenario.revision}.`);
      if (JSON.stringify(old.scenario.rules) !== JSON.stringify(next.scenario.rules)) reasons.push('Linked rule revisions changed.');
      if (JSON.stringify(old.scenario.recipe) !== JSON.stringify(next.scenario.recipe) || old.scenario.expected_behavior !== next.scenario.expected_behavior) reasons.push('Expected behavior or assertions changed.');
      const ruleContent = (review: OwnerEvidenceReviewDto, row: OwnerScenarioEvidenceDto) => review.run.manifest.rules.filter((rule) => row.scenario.rules.some((ref) => ref.rule_id === rule.rule_id && ref.revision === rule.revision)).map((rule) => [rule.rule_id, rule.revision, rule.expected_behavior]);
      if (JSON.stringify(ruleContent(before, old)) !== JSON.stringify(ruleContent(after, next))) reasons.push('Frozen rule content changed.');
    }
    if (before.run.manifest.semantics_version !== after.run.manifest.semantics_version) reasons.push('Verification semantics changed.');
    return { id, before: old, after: next, comparable: reasons.length === 0, reasons };
  });
}
