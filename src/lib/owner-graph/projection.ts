import type { OwnerRuleRevisionDto, ScenarioRevisionDto } from '../tauri/bindings';
import { capabilityNodeId, ruleNodeId, scenarioNodeId, evidenceNodeId, type OwnerGraph,
  type OwnerGraphInput, type OwnerGraphNode, type OwnerGraphEdge } from './types';

/** A view of stored relationships. Only backend evidence carries result/freshness. */
export function projectOwnerGraph(input: OwnerGraphInput): OwnerGraph {
  const nodes = new Map<string, OwnerGraphNode>();
  const edges = new Map<string, OwnerGraphEdge>();
  const gaps = new Set<string>();
  const preserved = new Set(input.change?.preserved_rules.map((ref) => ruleNodeId(ref.rule_id, ref.revision)) ?? []);
  const reviews = input.reviews.filter((review) => review.run.app_id === input.appId)
    .sort((a, b) => b.run.started_at - a.run.started_at || a.run.id.localeCompare(b.run.id));
  const rules = new Map<string, OwnerRuleRevisionDto>();
  const scenarios = new Map<string, ScenarioRevisionDto>();
  for (const rule of [...input.rules, ...reviews.flatMap((review) => review.run.manifest.rules)]) {
    if (rule.app_id === input.appId) rules.set(ruleNodeId(rule.rule_id, rule.revision), rule);
  }
  for (const scenario of [...input.scenarios, ...reviews.flatMap((review) => review.scenarios.map((row) => row.scenario))]) {
    if (scenario.app_id === input.appId) scenarios.set(scenarioNodeId(scenario.id, scenario.revision), scenario);
  }
  for (const capability of [...input.capabilities].sort((a, b) => a.sort_order - b.sort_order || a.id.localeCompare(b.id))) {
    if (capability.app_id !== input.appId) continue;
    const id = capabilityNodeId(capability.id);
    const hasRules = [...rules.values()].some((rule) => rule.capability_id === capability.id);
    nodes.set(id, { id, kind: 'capability', layer: 'intent', label: capability.label, capabilityId: capability.id,
      revision: capability.version, status: null, freshness: null, selectedChange: [...rules.values()].some((rule) => rule.capability_id === capability.id && preserved.has(ruleNodeId(rule.rule_id, rule.revision))),
      gaps: hasRules ? [] : ['No confirmed rule recorded.'], source: capability, review: null });
  }
  function connect(source: string, target: string, kind: OwnerGraphEdge['kind']): void {
    if (!nodes.has(source) || !nodes.has(target)) {
      gaps.add(`Unmapped ${kind} relationship: ${source} → ${target}.`); return;
    }
    const id = `${kind}:${source}:${target}`;
    edges.set(id, { id, source, target, kind });
  }
  for (const [id, rule] of rules) {
    const covered = [...scenarios.values()].some((scenario) => scenario.rules.some((ref) => ref.rule_id === rule.rule_id && ref.revision === rule.revision));
    nodes.set(id, { id, kind: 'rule', layer: 'intent', label: rule.expected_behavior, capabilityId: rule.capability_id,
      revision: rule.revision, status: null, freshness: null, selectedChange: preserved.has(id),
      gaps: covered ? [] : ['Coverage unknown: no scenario references this rule revision.'], source: rule, review: null });
    connect(capabilityNodeId(rule.capability_id), id, 'requires');
  }
  for (const [id, scenario] of scenarios) {
    const observed = reviews.some((review) => review.scenarios.some((row) => row.scenario.id === scenario.id && row.scenario.revision === scenario.revision));
    nodes.set(id, { id, kind: 'scenario', layer: 'scenarios', label: scenario.title, capabilityId: scenario.capability_id,
      revision: scenario.revision, status: null, freshness: null,
      selectedChange: scenario.rules.some((ref) => preserved.has(ruleNodeId(ref.rule_id, ref.revision))),
      gaps: [...(scenario.rules.length ? [] : ['No rule revision linked.']), ...(observed ? [] : ['No reviewed run recorded for this scenario revision.'])], source: scenario, review: null });
    for (const ref of scenario.rules) connect(ruleNodeId(ref.rule_id, ref.revision), id, 'verifies');
  }
  for (const review of reviews) {
    for (const row of review.scenarios) {
      const id = evidenceNodeId(review.run.id, row.scenario.id, row.scenario.revision);
      nodes.set(id, { id, kind: 'evidence', layer: 'evidence', label: `${row.scenario.title} · ${row.status.replaceAll('_', ' ')}`,
        capabilityId: row.scenario.capability_id, revision: row.scenario.revision, status: row.status,
        freshness: review.freshness, selectedChange: input.change?.id === review.run.change_id,
        gaps: [...(row.result ? [] : ['No observation recorded.']), ...review.freshness_reasons], source: row, review });
      connect(scenarioNodeId(row.scenario.id, row.scenario.revision), id, 'observed_by');
    }
  }
  for (const id of preserved) if (!nodes.has(id)) gaps.add(`Preserved rule unavailable: ${id}.`);
  if (input.change) gaps.add('Impact outside explicitly preserved rules is unknown. File changes do not establish business impact.');
  const order = { capability: 0, rule: 1, scenario: 2, evidence: 3 };
  return Object.freeze({ appId: input.appId, nodes: Object.freeze([...nodes.values()].map((node) => Object.freeze({ ...node, gaps: Object.freeze([...node.gaps]) })).sort((a, b) =>
    a.capabilityId.localeCompare(b.capabilityId) || order[a.kind] - order[b.kind] || a.id.localeCompare(b.id))),
  edges: Object.freeze([...edges.values()].map((edge) => Object.freeze(edge)).sort((a, b) => a.id.localeCompare(b.id))),
  reviews: Object.freeze(reviews), change: input.change ?? null, gaps: Object.freeze([...gaps]) });
}
