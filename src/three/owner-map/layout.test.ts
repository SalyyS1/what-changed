import { describe, expect, it } from 'vitest';
import { ownerGraphFixture } from '../../lib/owner-graph/test-fixtures';
import { projectOwnerGraph } from '../../lib/owner-graph/projection';
import { ownerGraphWindow } from '../../lib/owner-graph/selection';
import { layoutOwnerWorkbench } from './layout';

describe('Owner evidence workbench layout', () => {
  it('is deterministic with three semantic planes and preserves identity when separating layers', () => {
    const graph = projectOwnerGraph(ownerGraphFixture()); const bounded = ownerGraphWindow(graph, graph.nodes);
    const first = layoutOwnerWorkbench(bounded); const expanded = layoutOwnerWorkbench(bounded, 12);
    expect(first).toEqual(layoutOwnerWorkbench(bounded));
    expect(first.positions.get('rule:rule-a:1')!.y).toBe(12);
    expect(first.positions.get('scenario:scenario-a:1')!.y).toBe(6);
    expect(first.positions.get('evidence:run-a:scenario-a:1')!.y).toBe(0);
    for (const [id, point] of first.positions) {
      const next = expanded.positions.get(id)!;
      expect(next.x).toBe(point.x); expect(next.z).toBe(point.z); expect(next.y).toBe(point.y * 2);
    }
  });
});
