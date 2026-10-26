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
  it('clamps the active option when a large graph is replaced by a smaller graph', async () => {
    const onfocus = vi.fn();
    const view = render(SystemMapNodeList, { nodes: nodes(400), selectedId: null, onhover: vi.fn(), onfocus, onescape: vi.fn() });
    const list = screen.getByRole('listbox', { name: 'Explore system map nodes' });
    await fireEvent.keyDown(list, { key: 'End' });
    expect(list.getAttribute('aria-activedescendant')).toBe('system-node-399');

    await view.rerender({ nodes: nodes(3), selectedId: null, onhover: vi.fn(), onfocus, onescape: vi.fn() });
    expect(list.getAttribute('aria-activedescendant')).toBe('system-node-2');
    expect(document.getElementById('system-node-2')).not.toBeNull();
    await fireEvent.keyDown(list, { key: 'Enter' });
    expect(onfocus).toHaveBeenLastCalledWith(expect.objectContaining({ entity_id: 'process:2' }));
  });
});
