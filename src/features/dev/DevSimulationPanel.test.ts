import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import DevSimulationPanel from './DevSimulationPanel.svelte';

const client = vi.hoisted(() => ({
  seedSynthetic: vi.fn(),
  simulateEvent: vi.fn(),
}));

vi.mock('../../lib/tauri/client', () => client);

describe('DevSimulationPanel', () => {
  beforeEach(() => {
    client.seedSynthetic.mockReset();
    client.simulateEvent.mockReset();
  });

  it('keeps synthetic actions unavailable until developer mode is enabled', () => {
    render(DevSimulationPanel, { developerMode: false });

    expect(screen.getByText('Developer mode is off')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Seed profile' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Driver update' })).toBeNull();
  });

  it('offers every profile, the relations switch and all six simulations', async () => {
    render(DevSimulationPanel, { developerMode: true });

    expect(screen.getByRole('button', { name: 'Synthetic data profile' })).toBeTruthy();
    expect(screen.getByRole('switch', { name: 'Include synthetic entity relations' })).toBeTruthy();

    for (const name of [
      'Driver update',
      'Port change',
      'Process crash',
      'DNS change',
      'Docker restart',
      'Config diff',
    ]) {
      expect(screen.getByRole('button', { name })).toBeTruthy();
    }

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Synthetic data profile' }), {
      key: 'Enter',
    });
    const listbox = await screen.findByRole('listbox');
    for (const option of [
      'One day',
      'One week',
      'Burst',
      'Linked demo story (8 events)',
      '10,000 events',
      '100,000 events',
      '1 million events',
    ]) {
      expect(within(listbox).getByRole('option', { name: option })).toBeTruthy();
    }
  });

  it('makes the linked demo relations mandatory and discloses the fictional story', async () => {
    client.seedSynthetic.mockResolvedValue(8);
    render(DevSimulationPanel, { developerMode: true });

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Synthetic data profile' }), {
      key: 'Enter',
    });
    await screen.findByRole('listbox');
    await fireEvent.pointerUp(screen.getByRole('option', { name: 'Linked demo story (8 events)' }), {
      pointerType: 'mouse',
    });

    const relationsSwitch = screen.getByRole('switch', { name: 'Include synthetic entity relations' });
    expect(relationsSwitch.getAttribute('aria-checked')).toBe('true');
    expect(relationsSwitch.hasAttribute('disabled')).toBe(true);
    expect(screen.getByText('Fictional observations and historical edges for a walkthrough; no causal claim.')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Seed profile' }));
    expect(client.seedSynthetic).toHaveBeenCalledWith('demo', true);
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toContain('8 synthetic events added with relations.'),
    );
  });

  it('seeds the selected profile with relations and announces success', async () => {
    client.seedSynthetic.mockResolvedValue(10_000);
    render(DevSimulationPanel, { developerMode: true });

    await fireEvent.keyDown(screen.getByRole('button', { name: 'Synthetic data profile' }), {
      key: 'Enter',
    });
    await screen.findByRole('listbox');
    await fireEvent.pointerUp(screen.getByRole('option', { name: '10,000 events' }), {
      pointerType: 'mouse',
    });
    await fireEvent.click(screen.getByRole('switch', { name: 'Include synthetic entity relations' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Seed profile' }));

    expect(client.seedSynthetic).toHaveBeenCalledWith('10k', true);
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('10,000 synthetic events added with relations.'));
  });

  it('disables actions and announces an operation while it is pending', async () => {
    let finish: ((count: number) => void) | undefined;
    client.seedSynthetic.mockReturnValue(
      new Promise<number>((resolve) => {
        finish = resolve;
      }),
    );
    render(DevSimulationPanel, { developerMode: true });

    await fireEvent.click(screen.getByRole('button', { name: 'Seed profile' }));

    expect(screen.getByRole('progressbar', { name: 'Synthetic data operation in progress' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Seed profile' }).hasAttribute('disabled')).toBe(true);
    expect(screen.getByRole('button', { name: 'Driver update' }).hasAttribute('disabled')).toBe(true);
    finish?.(24);
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('24 synthetic events added.'));
  });

  it('sends every simulation button as its exact backend kind', async () => {
    client.simulateEvent.mockResolvedValue(1);
    render(DevSimulationPanel, { developerMode: true });

    const cases = [
      ['Driver update', 'driver_update'],
      ['Port change', 'port_change'],
      ['Process crash', 'process_crash'],
      ['DNS change', 'dns_change'],
      ['Docker restart', 'docker_restart'],
      ['Config diff', 'config_diff'],
    ] as const;

    for (const [label, kind] of cases) {
      await fireEvent.click(screen.getByRole('button', { name: label }));
      await waitFor(() => expect(client.simulateEvent).toHaveBeenCalledWith(kind));
    }
  });

  it('uses the backend kind and never renders a rejected internal value', async () => {
    client.simulateEvent.mockRejectedValue(new Error('C:\\Users\\private\\events.db'));
    render(DevSimulationPanel, { developerMode: true });

    await fireEvent.click(screen.getByRole('button', { name: 'DNS change' }));

    expect(client.simulateEvent).toHaveBeenCalledWith('dns_change');
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('The event could not be simulated. Check the logs and try again.');
    expect(alert.textContent).not.toContain('private');
  });
});
