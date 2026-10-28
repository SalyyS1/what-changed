import { describe, expect, it } from 'vitest';
import { fixtureEvidenceReview } from '../../features/acceptance/owner-evidence-fixtures';
import { fixtureRule } from '../../features/owner-workspace/owner-workspace-fixtures';
import { ownerGraphFixture } from './test-fixtures';
import { projectOwnerGraph } from './projection';
import { ownerGraphWindow, filterOwnerNodes, reconcileSelection } from './selection';
import { compareOwnerRuns } from './comparison';
import { layoutOwnerGraph, clampOwnerPan, clampOwnerZoom } from './layout';
describe('Owner semantic projection', () => {
  it('preserves authoritative evidence axes and exact record identity without changing source', () => {
    const input = ownerGraphFixture(); const original = JSON.stringify(input);
    const graph = projectOwnerGraph(input);
    expect(graph.nodes.map((node) => node.kind)).toEqual(['capability', 'rule', 'scenario', 'evidence']);
    expect(graph.edges.map((edge) => edge.kind).sort()).toEqual(['observed_by', 'requires', 'verifies']);
    const evidence = graph.nodes.find((node) => node.kind === 'evidence')!;
    expect(evidence.status).toBe(input.reviews[0]!.scenarios[0]!.status);
    expect(evidence.freshness).toBe('unknown');
    expect(evidence.review).toBe(input.reviews[0]);
    expect(graph.reviews[0]!.counts).toBe(input.reviews[0]!.counts);
    expect(graph.nodes.filter((node) => node.kind !== 'evidence').every((node) => node.status === null)).toBe(true);
    expect(JSON.stringify(input)).toBe(original);
    expect(Object.isFrozen(graph.nodes)).toBe(true);
  });
  it.each(['failed', 'blocked', 'not_run', 'passed'] as const)('retains backend %s even with unknown freshness and owner decisions', (status) => {
    const input = ownerGraphFixture(); input.reviews[0]!.scenarios[0]!.status = status;
    const graph = projectOwnerGraph(input);
    expect(graph.nodes.find((node) => node.kind === 'evidence')!.status).toBe(status);
    expect(graph.nodes.find((node) => node.kind === 'evidence')!.freshness).toBe('unknown');
  });
  it('exposes unmapped references and missing coverage without inventing relationships', () => {
    const graph = projectOwnerGraph({ ...ownerGraphFixture(), scenarios: [], reviews: [], capabilities: [] });
    expect(graph.edges).toEqual([]);
    expect(graph.gaps.some((gap) => gap.startsWith('Unmapped requires'))).toBe(true);
    expect(graph.nodes[0]!.gaps).toContain('Coverage unknown: no scenario references this rule revision.');
    expect(graph.gaps.some((gap) => gap.includes('Impact outside'))).toBe(true);
  });
  it('isolates app ownership and uses explicit revision links for request overlays', () => {
    const input = ownerGraphFixture();
    const graph = projectOwnerGraph({ ...input, rules: [...input.rules, fixtureRule(2), fixtureRule(3, 'another-app')], reviews: [fixtureEvidenceReview('another-app')] });
    expect(graph.nodes.some((node) => node.revision === 3)).toBe(false);
    expect(graph.nodes.find((node) => node.id === 'rule:rule-a:1')!.selectedChange).toBe(true);
    expect(graph.nodes.find((node) => node.id === 'rule:rule-a:2')!.selectedChange).toBe(false);
    expect(graph.reviews).toHaveLength(0);
  });
  it('bounds visible geometry while retaining failures, global summaries and every list node', () => {
    const input = ownerGraphFixture();
    const rules = Array.from({ length: 260 }, (_, index) => ({ ...fixtureRule(), rule_id: `rule-${index}` }));
    const graph = projectOwnerGraph({ ...input, rules, scenarios: [], reviews: [] });
    const first = ownerGraphWindow(graph, graph.nodes); const last = ownerGraphWindow(graph, graph.nodes, 100);
    expect(first.nodes).toHaveLength(120); expect(first.edges.length).toBeLessThanOrEqual(240);
    expect(first.reason).toContain('list includes every'); expect(last.page).toBe(2);
    expect(last.nodes).toHaveLength(21); expect(graph.nodes).toHaveLength(261);
    expect(reconcileSelection(last.nodes, first.nodes[0]!.id)).toBe(last.nodes[0]!.id);
    expect(layoutOwnerGraph(first)).toEqual(layoutOwnerGraph(first));
    expect(clampOwnerZoom(100)).toBe(2.5); expect(clampOwnerZoom(0)).toBe(0.35); expect(clampOwnerPan(-100, 20)).toBe(-20);
  });
  it('filters by the backend stale/failure axes while keeping exact source rows', () => {
    const input = ownerGraphFixture(); input.reviews[0]!.freshness = 'stale'; input.reviews[0]!.scenarios[0]!.status = 'failed';
    const graph = projectOwnerGraph(input);
    expect(filterOwnerNodes(graph, 'failed', null).map((node) => node.status)).toEqual(['failed']);
    expect(filterOwnerNodes(graph, 'stale', null)[0]!.freshness).toBe('stale');
    expect(filterOwnerNodes(graph, 'all', 'missing')).toEqual([]);
  });
});
describe('Frozen scenario comparisons', () => {
  it('compares unchanged scenario expectations across named candidates', () => {
    const before = fixtureEvidenceReview(); const after = structuredClone(before);
    after.run.id = 'after'; after.run.manifest.candidate.identity = 'Another named build';
    expect(compareOwnerRuns(before, after)[0]!.comparable).toBe(true);
  });
  it('marks changed revision, assertions and rules incomparable and keeps absent scenarios', () => {
    const before = fixtureEvidenceReview(); const after = structuredClone(before);
    after.scenarios[0]!.scenario.revision = 2;
    after.scenarios[0]!.scenario.rules = [{ rule_id: 'rule-a', revision: 2 }];
    after.scenarios[0]!.scenario.recipe.steps[0]!.assertions = [];
    const row = compareOwnerRuns(before, after)[0]!;
    expect(row.comparable).toBe(false); expect(row.reasons).toHaveLength(4);
    after.scenarios = [];
    expect(compareOwnerRuns(before, after)[0]!.reasons).toContain('Scenario is absent from one frozen attempt.');
  });
});
