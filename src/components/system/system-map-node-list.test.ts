import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { SystemMapNode } from '../../lib/tauri/bindings';
import SystemMapNodeList from './SystemMapNodeList.svelte';

const nodes = (count: number): SystemMapNode[] => Array.from({ length: count }, (_, index) => ({
  entity_id: `process:${index}`,
  entity_type: 'process',
  display_name: `Process ${index}`,
  hop: index === 0 ? 0 : 1,
  degree: 1,
  last_changed_at: null,
  recent_event_count: 0,
}));

describe('SystemMapNodeList', () => {
  it('keeps keyboard navigation and activation on the named listbox', async () => {
    const onfocus = vi.fn();
    const onescape = vi.fn();
    render(SystemMapNodeList, { nodes: nodes(400), selectedId: null, onhover: vi.fn(), onfocus, onescape });
    const list = screen.getByRole('listbox', { name: 'Explore system map nodes' });
    for (const [key, index] of [['ArrowDown', 1], ['PageDown', 11], ['End', 399], ['PageUp', 389], ['ArrowUp', 388], ['Home', 0]] as const) {
      await fireEvent.keyDown(list, { key });
      const activeId = list.getAttribute('aria-activedescendant');
      expect(activeId).toMatch(new RegExp(`-system-node-${index}$`));
      expect(list.contains(document.getElementById(activeId!))).toBe(true);
    }
    await fireEvent.keyDown(list, { key: ' ' });
    expect(onfocus).toHaveBeenLastCalledWith(expect.objectContaining({ entity_id: 'process:0' }));
    await fireEvent.keyDown(list, { key: 'Escape' });
    expect(onescape).toHaveBeenCalledOnce();
  });

  it('clamps the active option when a large graph is replaced by a smaller graph', async () => {
    const onfocus = vi.fn();
    const view = render(SystemMapNodeList, { nodes: nodes(400), selectedId: null, onhover: vi.fn(), onfocus, onescape: vi.fn() });
    const list = screen.getByRole('listbox', { name: 'Explore system map nodes' });
    await fireEvent.keyDown(list, { key: 'End' });
    expect(list.getAttribute('aria-activedescendant')).toMatch(/-system-node-399$/);

    await view.rerender({ nodes: nodes(3), selectedId: null, onhover: vi.fn(), onfocus, onescape: vi.fn() });
    expect(list.getAttribute('aria-activedescendant')).toMatch(/-system-node-2$/);
    expect(list.contains(document.getElementById(list.getAttribute('aria-activedescendant')!))).toBe(true);
    await fireEvent.keyDown(list, { key: 'Enter' });
    expect(onfocus).toHaveBeenLastCalledWith(expect.objectContaining({ entity_id: 'process:2' }));
  });

  it('keeps active descendant identities within each of two map lists', async () => {
    const props = { nodes: nodes(400), selectedId: null, onhover: vi.fn(), onfocus: vi.fn(), onescape: vi.fn() };
    render(SystemMapNodeList, props);
    render(SystemMapNodeList, props);
    const [first, second] = screen.getAllByRole('listbox', { name: 'Explore system map nodes' });
    expect(screen.getAllByRole('listbox', { name: 'Explore system map nodes' })).toHaveLength(2);
    if (!first || !second) throw new Error('Both map lists must render');
    const firstActive = first.getAttribute('aria-activedescendant');
    const secondActive = second.getAttribute('aria-activedescendant');
    expect(firstActive).not.toBe(secondActive);
    second.focus();
    await fireEvent.keyDown(second, { key: 'End' });
    expect(first.getAttribute('aria-activedescendant')).toBe(firstActive);
    expect(first.contains(document.getElementById(firstActive!))).toBe(true);
    expect(second.contains(document.getElementById(second.getAttribute('aria-activedescendant')!))).toBe(true);
    const ids = [...document.querySelectorAll('[role="option"]')].map((option) => option.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
