import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import type { EventSummary } from '../lib/tauri/bindings';
import { loading, ready } from '../lib/types/async-state';
import { checkpoints } from '../lib/stores/checkpoints.svelte';
import { eventDetails } from '../lib/stores/eventDetails.svelte';
import { investigation } from '../lib/stores/investigation.svelte';
import { monitoring } from '../lib/stores/monitoring.svelte';
import { settings } from '../lib/stores/settings.svelte';
import { timeline } from '../lib/stores/timeline.svelte';
import App from './App.svelte';

const event: EventSummary = {
  id: 48,
  timestamp: Date.now(),
  category: 'network',
  event_type: 'opened',
  entity_type: 'port',
  entity_id: 'port:25565',
  entity_display_name: '25565',
  severity: 'info',
  importance: 'notable',
  confidence: 'high',
  source_collector: 'synthetic',
  before: null,
  after: null,
  field: null,
  user_friendly_summary: 'Port 25565 opened',
  occurred_at_is_uncertain: false,
};

/** Observe the actual hash navigation, rather than relying on route-table strings. */
function observeRoutes() {
  const visited = [window.location.hash];
  const record = (event: HashChangeEvent) => {
    // Hashchange is queued. Reading location here can miss a short-lived hop
    // when another navigation already replaced it before this callback runs.
    visited.push(event.newURL ? new URL(event.newURL).hash : window.location.hash);
  };
  window.addEventListener('hashchange', record);
  return {
    visited,
    stop: () => window.removeEventListener('hashchange', record),
    expectNoSystemMap: () => {
      expect(visited.some((hash) => /^#\/system-map(?:[/?]|$)/.test(hash))).toBe(false);
      expect(window.location.hash).not.toMatch(/^#\/system-map(?:[/?]|$)/);
    },
  };
}

describe('§48 primary routes stay independent of System Map', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      value: (query: string) => ({
        matches: query.includes('dark'),
        media: query,
        addEventListener() {},
        removeEventListener() {},
      }),
    });
    monitoring.resolved = true;
    monitoring.recoveryRequired = false;
    monitoring.status = null;
    vi.spyOn(monitoring, 'attach').mockResolvedValue(() => {});
    settings.state = loading();
    investigation.setDraft('');
    eventDetails.close();
    timeline.state = ready({
      items: [{ kind: 'event', event }],
      loadedRows: 1,
      totalCount: 1,
      hasMore: false,
    });
    checkpoints.state = ready([]);
  });

  afterEach(() => {
    Reflect.deleteProperty(window, '__TAURI_INTERNALS__');
    eventDetails.close();
    window.location.hash = '';
    vi.restoreAllMocks();
  });

  it('detects a transient System Map hop even after the hash has changed again', async () => {
    window.location.hash = '#/timeline';
    const routes = observeRoutes();
    try {
      window.location.hash = '#/system-map';
      window.location.hash = '#/timeline';

      expect(window.location.hash).toBe('#/timeline');
      await waitFor(() => expect(routes.visited).toContain('#/system-map'));
      expect(() => routes.expectNoSystemMap()).toThrow();
    } finally {
      routes.stop();
    }
  });

  it('starts an investigation directly from Home', async () => {
    window.location.hash = '#/';
    const routes = observeRoutes();
    try {
      render(App);
      expect(await screen.findByRole('heading', { level: 1, name: 'Home' })).toBeTruthy();

      await fireEvent.input(screen.getByLabelText('Describe the symptom'), {
        target: { value: 'the API stopped connecting' },
      });
      await fireEvent.click(screen.getByRole('button', { name: 'Investigate' }));

      await waitFor(() => expect(window.location.hash).toBe('#/investigations'));
      expect(await screen.findByRole('heading', { level: 1, name: 'Investigations' })).toBeTruthy();
      expect(investigation.draft).toBe('the API stopped connecting');
      routes.expectNoSystemMap();
    } finally {
      routes.stop();
    }
  });

  it('opens event details directly from Timeline', async () => {
    window.location.hash = '#/timeline';
    const routes = observeRoutes();
    try {
      render(App);
      expect(await screen.findByRole('heading', { level: 1, name: 'Timeline' })).toBeTruthy();

      await fireEvent.click(await screen.findByRole('button', { name: 'View details: Port 25565 opened' }));

      expect(await screen.findByTestId('drawer')).toBeTruthy();
      expect(window.location.hash).toBe('#/timeline');
      routes.expectNoSystemMap();
    } finally {
      routes.stop();
    }
  });

  it('reaches checkpoint creation from Investigations through the primary navigation', async () => {
    window.location.hash = '#/investigations';
    const routes = observeRoutes();
    try {
      render(App);
      expect(await screen.findByRole('heading', { level: 1, name: 'Investigations' })).toBeTruthy();

      // Only expose a bridge after App mounts. The Checkpoints screen can then
      // enable its action without any native IPC running in this DOM test.
      vi.spyOn(checkpoints, 'attachProgress').mockResolvedValue(() => {});
      vi.spyOn(checkpoints, 'load').mockResolvedValue(true);
      Object.defineProperty(window, '__TAURI_INTERNALS__', {
        configurable: true,
        value: {},
      });

      const navigation = within(screen.getByTestId('sidebar'));
      await fireEvent.click(navigation.getByRole('link', { name: /Checkpoints/ }));
      await waitFor(() => expect(window.location.hash).toBe('#/checkpoints'));
      const main = within(screen.getByTestId('main'));
      expect(await main.findByRole('heading', { level: 1, name: 'Checkpoints' })).toBeTruthy();
      const create = main.getByRole('button', { name: 'Create checkpoint' }) as HTMLButtonElement;
      expect(create.disabled).toBe(false);
      await fireEvent.click(create);

      expect(await screen.findByRole('dialog', { name: 'Create checkpoint' })).toBeTruthy();
      routes.expectNoSystemMap();
    } finally {
      routes.stop();
    }
  });
});
