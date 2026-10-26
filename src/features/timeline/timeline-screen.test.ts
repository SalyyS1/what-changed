import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ready } from '../../lib/types/async-state';
import TimelineScreen from './TimelineScreen.svelte';

describe('Timeline Change Map navigation', () => {
  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    history.replaceState(null, '', '#/');
  });

  it('uses one history entry and restores trigger focus when the map is closed', async () => {
    history.replaceState(null, '', '#/timeline?range=today&filter=all');
    vi.spyOn(history, 'back').mockImplementation(() => {
      history.replaceState(null, '', '#/timeline?range=today&filter=all');
      window.dispatchEvent(new HashChangeEvent('hashchange'));
    });
    render(TimelineScreen, {
      data: ready({ items: [], loadedRows: 0, totalCount: 0, hasMore: false }),
    });

    const trigger = screen.getByRole('button', { name: 'Map' });
    await fireEvent.click(trigger);
    expect(location.hash).toContain('view=map');

    await fireEvent.click(await screen.findByRole('button', { name: 'Switch to list' }));
    expect(history.back).toHaveBeenCalledOnce();
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Map' })));
  });
});
