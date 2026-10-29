import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { ready } from '../../lib/types/async-state';
import type { SearchResultDto } from '../../lib/tauri/bindings';
import { SearchStore, groupSearchResults } from '../../lib/stores/search.svelte';
import SearchPalette from './SearchPalette.svelte';

const port: SearchResultDto = { kind: 'event', category: 'network', importance: 'notable', event: { id: 1, timestamp: 1, category: 'network', event_type: 'opened', entity_type: 'port', entity_id: 'port:25565', entity_display_name: '25565', severity: 'info', importance: 'notable', confidence: 'high', source_collector: 'synthetic', before: null, after: null, field: null, user_friendly_summary: 'Port 25565 opened', occurred_at_is_uncertain: false } };
const checkpoint: SearchResultDto = { kind: 'checkpoint', checkpoint: { id: 9, name: 'Before driver update', created_at: 1 } };

describe('SearchPalette', () => {
  it('opens with Ctrl+K and exposes an accessible search dialog', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    render(SearchPalette, { controller });
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(await screen.findByRole('dialog', { name: 'Search changes' })).toBeTruthy();
    expect(screen.getByRole('textbox', { name: 'Search history' })).toBeTruthy();
  });

  it('focuses the query immediately after Ctrl+K so typing is not lost', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    render(SearchPalette, { controller });
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    const input = await screen.findByRole('textbox', { name: 'Search history' });
    await waitFor(() => expect(document.activeElement).toBe(input));
    await fireEvent.input(document.activeElement as HTMLInputElement, { target: { value: '25565' } });
    expect(controller.query).toBe('25565');
  });

  it('uses Enter on a port result to deep-link the timeline entity', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    controller.open = true; controller.query = '25565'; controller.state = ready(groupSearchResults([port]));
    render(SearchPalette, { controller });
    const input = await screen.findByRole('textbox', { name: 'Search history' });
    expect(screen.getByRole('option', { name: /Port 25565 opened/ })).toBeTruthy();
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(window.location.hash).toContain('entity=port%3A25565'));
  });

  it('moves the active descendant with arrows and opens a checkpoint', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    controller.open = true; controller.query = 'before'; controller.state = ready(groupSearchResults([port, checkpoint]));
    render(SearchPalette, { controller });
    const input = await screen.findByRole('textbox', { name: 'Search history' });

    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(input.getAttribute('aria-activedescendant')).toBe('search-result-1');
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(window.location.hash).toBe('#/checkpoints?checkpoint=9'));
  });

  it('can focus an event entity in System Map', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    controller.open = true; controller.query = '25565'; controller.state = ready(groupSearchResults([port]));
    render(SearchPalette, { controller });

    await fireEvent.click(screen.getByRole('button', { name: 'Show 25565 in System Map' }));
    expect(window.location.hash).toContain('#/system-map?');
    expect(window.location.hash).toContain('focus=port%3A25565');
    expect(window.location.hash).toContain('type=port');
  });

  it('keeps listbox options as single, non-nested interactive controls', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    controller.open = true; controller.query = '25565'; controller.state = ready(groupSearchResults([port]));
    render(SearchPalette, { controller });

    const option = await screen.findByRole('option', { name: /Port 25565 opened/ });
    expect(option.tagName).toBe('BUTTON');
    expect(option.querySelector('button')).toBeNull();
    expect(screen.getByRole('button', { name: 'Show 25565 in System Map' }).closest('[role="option"]')).toBeNull();
  });

  it('offers useful alternatives when nothing matches', async () => {
    const controller = new SearchStore({ search: vi.fn().mockResolvedValue([]) });
    controller.open = true; controller.query = 'missing';
    render(SearchPalette, { controller });

    expect(await screen.findByText('No matches.')).toBeTruthy();
    expect(screen.getByText('Try an app name, a port, or a file name.')).toBeTruthy();
  });
});
