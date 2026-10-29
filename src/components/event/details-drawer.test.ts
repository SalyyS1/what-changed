import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ready } from '../../lib/types/async-state';
import type { EventDetailsDto } from '../../lib/tauri/bindings';
import { EventDetailsStore } from '../../lib/stores/eventDetails.svelte';
import DetailsDrawer from './DetailsDrawer.svelte';

const details: EventDetailsDto = {
  event: { id: 7, timestamp: Date.now() - 60_000, category: 'configuration', event_type: 'configuration_changed', entity_type: 'file', entity_id: 'file:C:/app/config.json', entity_display_name: 'config.json', severity: 'warning', importance: 'important', source_collector: 'synthetic', before: { port: 3000 }, after: { port: 25565 }, field: 'port', metadata: { path: 'C:/app/config.json' }, related_entity_ids: ['process:api.exe'], user_friendly_summary: 'API configuration changed', technical_summary: 'config.json port changed', confidence: 'high', sensitive: false, occurred_at_is_uncertain: false },
  related: [], prior_occurrences: 2,
};

describe('DetailsDrawer', () => {
  beforeEach(() => localStorage.removeItem('what-changed:technical-details-open'));

  it('shows human evidence first and keeps technical data collapsed', async () => {
    const controller = new EventDetailsStore({ fetchDetails: vi.fn() });
    controller.state = ready(details); controller.open = true;
    render(DetailsDrawer, { controller });

    expect(await screen.findByText('API configuration changed')).toBeTruthy();
    expect(screen.getByText('{"port":3000}')).toBeTruthy();
    expect(screen.getByText('{"port":25565}')).toBeTruthy();
    expect(screen.queryByText('file:C:/app/config.json')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Show technical details' }));
    expect(screen.getByText('file:C:/app/config.json')).toBeTruthy();
  });

  it('closes on Escape and returns focus to the invoking control', async () => {
    const opener = document.createElement('button');
    document.body.append(opener); opener.focus();
    const controller = new EventDetailsStore({ fetchDetails: vi.fn().mockResolvedValue(details) });
    render(DetailsDrawer, { controller });
    await controller.show(7, opener);
    await screen.findByTestId('drawer');
    await fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => expect(controller.open).toBe(false));
    await waitFor(() => expect(document.activeElement).toBe(opener));
  });

  it('renders a one-sided delta and labels the missing side as not recorded', async () => {
    const controller = new EventDetailsStore({ fetchDetails: vi.fn() });
    controller.state = ready({
      ...details,
      event: { ...details.event, before: null, after: { port: 25565 } },
    });
    controller.open = true;
    render(DetailsDrawer, { controller });

    expect(await screen.findByText('What changed')).toBeTruthy();
    expect(screen.getByText('Not recorded')).toBeTruthy();
    expect(screen.getByText('{"port":25565}')).toBeTruthy();
  });

  it('ignores an older details request that resolves after a newer selection', async () => {
    let resolveFirst!: (value: EventDetailsDto) => void;
    const first = new Promise<EventDetailsDto>((resolve) => { resolveFirst = resolve; });
    const newer = { ...details, event: { ...details.event, id: 8, entity_display_name: 'newer.json' } };
    const fetchDetails = vi.fn()
      .mockReturnValueOnce(first)
      .mockResolvedValueOnce(newer);
    const controller = new EventDetailsStore({ fetchDetails });

    const olderRequest = controller.show(7);
    await controller.show(8);
    resolveFirst(details);
    await olderRequest;

    expect(controller.state.status === 'ready' && controller.state.data.event.id).toBe(8);
  });

  it('renders the retained investigation summary when source-event retention has expired', async () => {
    const fetchDetails = vi.fn();
    const controller = new EventDetailsStore({ fetchDetails });
    controller.showSnapshot({
      id: 'event:77', event_id: 77, timestamp_ms: nowForSnapshot(), category: 'configuration',
      lane: 'config', importance: 'important', label: 'Saved configuration change',
      chain_ids: ['chain:1'], chain_depth: 0, is_suspect: true, snapshot_only: true,
    });
    render(DetailsDrawer, { controller });

    expect((await screen.findAllByText('Saved configuration change')).length).toBeGreaterThan(0);
    expect(screen.getByTestId('snapshot-retention-notice').textContent).toContain('saved investigation');
    expect(fetchDetails).not.toHaveBeenCalled();
  });
});

function nowForSnapshot(): number {
  return Date.now() - 31 * 24 * 60 * 60 * 1_000;
}
