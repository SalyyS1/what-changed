import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, within } from '@testing-library/svelte';
import App from './App.svelte';
import { navRoutes } from './routes';
import { theme, THEME_STORAGE_KEY } from '../lib/design/theme.svelte';
import { collectors } from '../lib/stores/collectors.svelte';
import { monitoring } from '../lib/stores/monitoring.svelte';
import { ready } from '../lib/types/async-state';
import type { CollectorStatusDto } from '../lib/tauri/bindings';

/**
 * The shell's behavioural claims: every destination is reachable from the
 * sidebar and from `Ctrl+N`, the active route is announced with
 * `aria-current`, the titlebar controls are named, and the theme is applied
 * to `<html>` from the store. jsdom has no `matchMedia`, so a stub is
 * installed that reports "dark" — the shell must not crash without one.
 */

function installMatchMedia(prefersDark: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      matches: prefersDark && query.includes('dark'),
      media: query,
      addEventListener() {},
      removeEventListener() {},
    }),
  });
}

async function settle() {
  await Promise.resolve();
  await new Promise((r) => setTimeout(r, 0));
}

describe('App shell', () => {
  beforeEach(() => {
    installMatchMedia(true);
    localStorage.removeItem(THEME_STORAGE_KEY);
    theme.set('dark');
    window.location.hash = '';
    collectors.state = ready([]);
    collectors.announcement = '';
    monitoring.paused = false;
    monitoring.recoveryRequired = false;
  });

  afterEach(() => {
    window.location.hash = '';
    vi.restoreAllMocks();
  });

  it('renders the titlebar, sidebar and main region', async () => {
    const { getByTestId } = render(App);
    await settle();
    expect(getByTestId('titlebar')).toBeTruthy();
    expect(getByTestId('sidebar')).toBeTruthy();
    expect(getByTestId('main').textContent?.trim()).not.toBe('');
  });

  it('lists all seven destinations with the home route current', async () => {
    const { getByTestId } = render(App);
    await settle();
    const links = within(getByTestId('sidebar')).getAllByRole('link');
    expect(links).toHaveLength(7);
    const current = links.filter((l) => l.getAttribute('aria-current') === 'page');
    expect(current).toHaveLength(1);
    expect(current[0]?.getAttribute('href')).toBe('#/');
  });

  it('navigates when the hash changes and marks the new route current', async () => {
    const { getByTestId } = render(App);
    await settle();
    window.location.hash = '#/settings';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await settle();
    const current = within(getByTestId('sidebar'))
      .getAllByRole('link')
      .find((l) => l.getAttribute('aria-current') === 'page');
    expect(current?.getAttribute('href')).toBe('#/settings');
    expect(within(getByTestId('main')).getByRole('heading', { level: 1 }).textContent).toBe(
      'Settings',
    );
  });

  it('jumps to a destination on Ctrl+N', async () => {
    render(App);
    await settle();
    const target = navRoutes.find((r) => r.shortcut === 3);
    await fireEvent.keyDown(window, { key: '3', ctrlKey: true });
    await settle();
    expect(window.location.hash).toBe(`#/${target?.path}`);
  });

  it('names every titlebar button', async () => {
    const { getByTestId } = render(App);
    await settle();
    const bar = within(getByTestId('titlebar'));
    for (const name of ['Minimize', 'Maximize', 'Close to tray']) {
      expect(bar.getByRole('button', { name })).toBeTruthy();
    }
  });

  it('renders the monitoring pill as active when the backend is unreachable', async () => {
    // jsdom has no Tauri bridge, so the status call fails. The shell must
    // still draw, and the honest default for a preview is "Active".
    const { getByTestId } = render(App);
    await settle();
    expect(within(getByTestId('titlebar')).getByText('Active')).toBeTruthy();
  });

  it('contains a rejected async subscription and remains safe to unmount', async () => {
    const unhandled = vi.fn();
    window.addEventListener('unhandledrejection', unhandled);
    vi.spyOn(monitoring, 'attach').mockRejectedValueOnce(new Error('listener unavailable'));

    const { getByTestId, unmount } = render(App);
    await settle();
    expect(getByTestId('titlebar')).toBeTruthy();
    unmount();
    await settle();

    expect(unhandled).not.toHaveBeenCalled();
    window.removeEventListener('unhandledrejection', unhandled);
  });

  it('shows and politely announces degraded collector health', async () => {
    const degraded: CollectorStatusDto = {
      id: 'process', enabled: true, state: 'degraded',
      descriptor: { id: 'process', display_name: 'Programs', what_it_watches: 'Processes.', why_it_matters: 'Explains restarts.', cost_hint: 'cheap', cadence: { kind: 'poll', interval_ms: 5_000 }, requires_admin: false },
      last_run_at: null, last_duration_ms: null, events_emitted: 0, consecutive_failures: 0,
      last_error: null, requires_admin: false, queue_drops: 1, failed_commit_rows: 0,
      details: null,
      cycle_admission_drops: 0, failed_cycles: 0,
    };
    collectors.state = ready([degraded]);
    collectors.announcement = 'Programs is now degraded.';
    const { getByTestId } = render(App);
    await settle();

    const bar = within(getByTestId('titlebar'));
    expect(bar.getByText('Degraded')).toBeTruthy();
    const live = bar.getByText('Programs is now degraded.');
    expect(live.getAttribute('aria-live')).toBe('polite');
  });

  it('applies the theme mode to <html> and follows the OS under "system"', async () => {
    theme.set('light');
    const { unmount } = render(App);
    await settle();
    expect(document.documentElement.dataset.theme).toBe('light');
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('light');
    unmount();

    theme.set('system');
    render(App);
    await settle();
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('never renders a raw hex colour in the shell markup', async () => {
    const { container } = render(App);
    await settle();
    expect(container.innerHTML).not.toMatch(/#[0-9a-fA-F]{6}\b/);
  });
});
