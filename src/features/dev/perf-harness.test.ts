import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TimelineList from '../../components/timeline/TimelineList.svelte';
import { performanceTimeline } from './perf-timeline-fixture';

afterEach(() => vi.restoreAllMocks());

describe('performance timeline fixture', () => {
  it('loads 100k unique events and renders expected rows at later real virtual-scroll positions', async () => {
    // TanStack measures offsetWidth/offsetHeight, which jsdom leaves at zero.
    vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(900);
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(360);

    const snapshot = performanceTimeline(100_000);
    expect(snapshot.items).toHaveLength(100_000);
    expect(snapshot.loadedRows).toBe(100_000);
    expect(snapshot.totalCount).toBe(100_000);
    expect(snapshot.items[0]?.kind).toBe('event');
    expect(snapshot.items[99_999]?.kind).toBe('event');
    if (snapshot.items[99_999]?.kind === 'event') {
      expect(snapshot.items[99_999].event.id).toBe(100_000);
      expect(snapshot.items[99_999].event.user_friendly_summary).toContain('Fixture #100000:');
    }

    render(TimelineList, { snapshot, onopen: vi.fn(), onexpand: vi.fn() });
    const viewport = screen.getByTestId('timeline-viewport');
    const canvas = await waitFor(() => {
      const element = viewport.querySelector<HTMLElement>(':scope > div.relative');
      expect(Number.parseFloat(element?.style.height ?? '')).toBeGreaterThan(7_000_000);
      return element as HTMLElement;
    });
    const totalHeight = Number.parseFloat(canvas.style.height);

    for (const [fraction, minimumIndex] of [[0.5, 45_000], [0.9, 85_000]] as const) {
      viewport.scrollTop = totalHeight * fraction;
      await fireEvent.scroll(viewport);
      const button = await waitFor(() => {
        const laterRows = [...viewport.querySelectorAll<HTMLElement>('[data-index]')]
          .filter((element) => Number(element.dataset.index) > minimumIndex);
        expect(laterRows.length).toBeGreaterThan(0);
        const rowButton = laterRows
          .map((element) => element.querySelector<HTMLElement>('[data-testid="timeline-row"]'))
          .find((element) => element?.getAttribute('aria-label')?.includes('Fixture #'));
        expect(rowButton).toBeTruthy();
        return rowButton!;
      });
      const label = button.getAttribute('aria-label') ?? '';
      const id = Number(label.match(/Fixture #(\d+):/)?.[1]);
      expect(id).toBeGreaterThan(minimumIndex - 2_000);
      const expected = snapshot.items[id - 1];
      expect(expected?.kind).toBe('event');
      if (expected?.kind === 'event') expect(label).toBe(`View details: ${expected.event.user_friendly_summary}`);
    }
  }, 30_000);
});
