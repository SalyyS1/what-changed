import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { ownerGraphFixture } from '../../lib/owner-graph/test-fixtures';
import { projectOwnerGraph } from '../../lib/owner-graph/projection';
import OwnerMapNodeList from './OwnerMapNodeList.svelte';
import OwnerMapFrame from './OwnerMapFrame.svelte';
import BeforeAfterReview from './BeforeAfterReview.svelte';

beforeEach(() => { vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} }); });
afterEach(() => { cleanup(); localStorage.clear(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
describe('Owner accessible view equivalence', () => {
  it('supports keyboard selection, page navigation, typeahead and Escape', async () => {
    const nodes = projectOwnerGraph(ownerGraphFixture()).nodes;
    const onselect = vi.fn(); const onescape = vi.fn();
    render(OwnerMapNodeList, { nodes, selectedId: nodes[0]!.id, onselect, onescape });
    const list = screen.getByRole('listbox'); list.focus();
    await fireEvent.keyDown(list, { key: 'End' }); expect(onselect).toHaveBeenLastCalledWith(nodes[3]!.id);
    await fireEvent.keyDown(list, { key: 'Home' }); expect(onselect).toHaveBeenLastCalledWith(nodes[0]!.id);
    await fireEvent.keyDown(list, { key: 'PageDown' }); expect(onselect).toHaveBeenLastCalledWith(nodes[3]!.id);
    await fireEvent.keyDown(list, { key: 'ArrowUp' }); expect(onselect).toHaveBeenLastCalledWith(nodes[2]!.id);
    await fireEvent.keyDown(list, { key: 'b' }); expect(onselect).toHaveBeenLastCalledWith(nodes[0]!.id);
    await fireEvent.keyDown(list, { key: 'Escape' }); expect(onescape).toHaveBeenCalledOnce();
  });
  it('scopes active IDs between instances and reconciles a shrinking dataset', async () => {
    const nodes = projectOwnerGraph(ownerGraphFixture()).nodes;
    const props = { nodes, selectedId: null, onselect: vi.fn(), onescape: vi.fn() };
    const first = render(OwnerMapNodeList, props); render(OwnerMapNodeList, props);
    const lists = screen.getAllByRole('listbox');
    expect(lists[0]!.getAttribute('aria-activedescendant')).not.toBe(lists[1]!.getAttribute('aria-activedescendant'));
    await fireEvent.keyDown(lists[0]!, { key: 'End' });
    await first.rerender({ ...props, nodes: nodes.slice(0, 1) }); await tick();
    const id = lists[0]!.getAttribute('aria-activedescendant')!;
    expect(document.getElementById(id)?.textContent).toContain('Bookings');
  });
  it('keeps authoritative totals, exact inspection and selection when switching to list', async () => {
    const input = ownerGraphFixture();
    render(OwnerMapFrame, { input });
    expect(screen.getByLabelText('Authoritative evidence totals').textContent).toContain('0 passed · 0 failed · 0 blocked · 1 not run');
    const list = screen.getByRole('listbox'); await fireEvent.keyDown(list, { key: 'End' });
    await fireEvent.click(screen.getByRole('button', { name: 'List' }));
    const details = screen.getByRole('region', { name: 'Selected evidence details' });
    expect(details.textContent).toContain('synthetic-manifest-digest');
    expect(details.textContent).toContain('Synthetic owner label');
    expect(details.textContent).toContain('not run / unknown');
    expect(screen.getByLabelText('Authoritative evidence totals').textContent).toContain('1 not run');
    expect(screen.getByText('Review not recorded')).toBeTruthy();
  });
  it('falls back visibly when WebGL is unavailable without dropping evidence', async () => {
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
    render(OwnerMapFrame, { input: ownerGraphFixture() });
    await fireEvent.click(screen.getByRole('button', { name: '3D' }));
    expect(screen.getByRole('status').textContent).toContain('WebGL 2 is unavailable');
    expect(screen.getByRole('button', { name: '2D' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('listbox')).toBeTruthy();
    expect(localStorage.getItem('owner-map-mode')).toBe('2d');
  });
  it('explains incomparable revisions and exposes both exact evidence targets', async () => {
    const before = ownerGraphFixture().reviews[0]!; const after = structuredClone(before);
    after.run.id = 'after'; after.scenarios[0]!.scenario.revision = 2;
    const oninspect = vi.fn(); render(BeforeAfterReview, { reviews: [after, before], oninspect });
    await fireEvent.click(screen.getByText('Before / after evidence review'));
    expect(screen.getByText(/Incomparable: Scenario revision changed/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Inspect before evidence' }));
    expect(oninspect).toHaveBeenCalledWith('run-a', 'evidence:run-a:scenario-a:1');
  });
});
