import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { EventSummary } from '../../lib/tauri/bindings';
import { formatDayHeader, formatHour } from '../../lib/formatting/time';
import TimelineList from './TimelineList.svelte';
import { clusterIdentity } from '../../lib/timeline/identity';

const event: EventSummary = { id: 1, timestamp: Date.now(), category: 'network', event_type: 'opened', entity_type: 'port', entity_id: 'port:25565', entity_display_name: '25565', severity: 'info', importance: 'notable', confidence: 'high', source_collector: 'synthetic', before: null, after: null, field: null, user_friendly_summary: 'Port 25565 opened', occurred_at_is_uncertain: false };

describe('TimelineList', () => {
  it('shows an already-redacted before and after delta without changing the fixed row height', async () => {
    const eventWithDelta = {
      ...event,
      before: { port: 3000 },
      after: { port: 25565 },
    } as EventSummary;

    render(TimelineList, { snapshot: { items: [{ kind: 'event', event: eventWithDelta }], loadedRows: 1, totalCount: 1, hasMore: false }, onopen: vi.fn(), onexpand: vi.fn() });

    const row = await screen.findByTestId('timeline-row');
    expect(row.className).toContain('h-18');
    expect(screen.getByText('{"port":3000} → {"port":25565}')).toBeTruthy();
  });

  it('shows one-sided row deltas with an explicit missing value', async () => {
    render(TimelineList, {
      snapshot: {
        items: [{ kind: 'event', event: { ...event, before: null, after: 'running' } }],
        loadedRows: 1,
        totalCount: 1,
        hasMore: false,
      },
      onopen: vi.fn(),
      onexpand: vi.fn(),
    });
    expect(await screen.findByText('Not recorded → running')).toBeTruthy();
  });

  it('renders a fixed-height virtual row and opens details from a real button', async () => {
    const onopen = vi.fn();
    render(TimelineList, { snapshot: { items: [{ kind: 'event', event }], loadedRows: 1, totalCount: 128_431, hasMore: true }, onopen, onexpand: vi.fn() });
    const row = await screen.findByRole('button', { name: 'View details: Port 25565 opened' });
    expect(row.className).toContain('h-18');
    await fireEvent.click(row);
    expect(onopen).toHaveBeenCalledWith(event, row);
    expect(screen.getByText('Showing 1 of 128,431')).toBeTruthy();
  });

  it('keeps the visible-hour header sticky and advances it at the scroll boundary', async () => {
    const newer = new Date(2024, 0, 15, 15, 0).getTime();
    const older = new Date(2024, 0, 15, 13, 0).getTime();
    render(TimelineList, {
      snapshot: {
        items: [
          { kind: 'event', event: { ...event, id: 1, timestamp: newer } },
          { kind: 'event', event: { ...event, id: 2, timestamp: older } },
        ],
        loadedRows: 2,
        totalCount: 2,
        hasMore: false,
      },
      onopen: vi.fn(),
      onexpand: vi.fn(),
    });

    const stickyHeader = () => screen.getAllByTestId('timeline-group')
      .find((header) => header.classList.contains('sticky'));
    expect(stickyHeader()?.textContent).toContain(`${formatDayHeader(newer)} · ${formatHour(newer)}`);

    const viewport = screen.getByTestId('timeline-viewport');
    // 32px sticky-flow offset + 32px group + 72px row + 1px into
    // the following group. This is a user-observable hour transition.
    viewport.scrollTop = 137;
    await fireEvent.scroll(viewport);
    await waitFor(() => {
      expect(stickyHeader()?.textContent).toContain(`${formatDayHeader(older)} · ${formatHour(older)}`);
    });
  });

  it('renders a cluster as one expandable row', async () => {
    const onexpand = vi.fn();
    const cluster = { key: 'docker:burst', category: 'docker' as const, coarse_entity: 'docker', bucket: 1, first_ts: 1, first_id: 1, last_ts: 2, last_id: 2 };
    const item = { kind: 'cluster' as const, key: cluster.key, cluster, label: '40 Docker-related changes', count: 40, first_ts: 1, last_ts: 2, highest_importance: 'notable' as const, category: 'docker' as const };
    render(TimelineList, { snapshot: { items: [item], loadedRows: 40, totalCount: 40, hasMore: false }, onopen: vi.fn(), onexpand });
    await fireEvent.click(await screen.findByRole('button', { name: '[Expand]' }));
    expect(onexpand).toHaveBeenCalledWith(item);
  });

  it('announces an expanded cluster and exposes a valid controlled target', async () => {
    const cluster = { key: 'docker:burst', category: 'docker' as const, coarse_entity: 'docker', bucket: 1, first_ts: 1, first_id: 1, last_ts: 2, last_id: 2 };
    const item = { kind: 'cluster' as const, key: cluster.key, cluster, label: '40 Docker-related changes', count: 40, first_ts: 1, last_ts: 2, highest_importance: 'notable' as const, category: 'docker' as const };
    render(TimelineList, { snapshot: { items: [item], loadedRows: 40, totalCount: 40, hasMore: false }, expanded: new Set([clusterIdentity(cluster)]), onopen: vi.fn(), onexpand: vi.fn() });

    const button = await screen.findByRole('button', { name: '[Collapse]' });
    expect(button.getAttribute('aria-expanded')).toBe('true');
    expect(document.getElementById(button.getAttribute('aria-controls') ?? '')).toBeTruthy();
  });

  it('renders same-key clusters with distinct exact-boundary identities', async () => {
    const first = { key: 'docker:burst', category: 'docker' as const, coarse_entity: 'docker', bucket: 1, first_ts: 1, first_id: 1, last_ts: 2, last_id: 40 };
    const second = { ...first, first_ts: 2, first_id: 41, last_ts: 3, last_id: 80 };
    const item = (cluster: typeof first) => ({ kind: 'cluster' as const, key: cluster.key, cluster, label: '40 Docker-related changes', count: 40, first_ts: cluster.first_ts, last_ts: cluster.last_ts, highest_importance: 'notable' as const, category: 'docker' as const });
    render(TimelineList, { snapshot: { items: [item(first), item(second)], loadedRows: 80, totalCount: 80, hasMore: false }, onopen: vi.fn(), onexpand: vi.fn() });

    const buttons = await screen.findAllByRole('button', { name: '[Expand]' });
    expect(buttons).toHaveLength(2);
    expect(buttons[0]?.getAttribute('aria-controls')).not.toBe(buttons[1]?.getAttribute('aria-controls'));
  });

  it('shows the live pill without replacing existing rows', async () => {
    const show = vi.fn();
    render(TimelineList, { snapshot: { items: [{ kind: 'event', event }], loadedRows: 1, totalCount: null, hasMore: false }, newCount: 3, onopen: vi.fn(), onexpand: vi.fn(), onshownew: show });
    expect(screen.getByText('Port 25565 opened')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '3 new changes' }));
    expect(show).toHaveBeenCalledOnce();
  });
});
