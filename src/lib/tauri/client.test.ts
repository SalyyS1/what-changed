import { afterEach, describe, expect, it, vi } from 'vitest';
import { backendAvailable, toAppError } from './client';
import { COMMANDS } from './bindings';

/**
 * The client's two jobs that are worth testing without a backend: recognising
 * a backend failure, and refusing to explode when there is no backend at all.
 * The command wrappers themselves are covered from the Rust side, where
 * `commands_contract.rs` proves every registered command has one.
 */

describe('the typed client', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('passes a real backend error through unchanged', () => {
    const backendError = {
      code: 'not_implemented' as const,
      message: 'this is not built yet; it lands in phase-06',
      retryable: false,
    };
    expect(toAppError(backendError)).toBe(backendError);
  });

  it('turns anything else into a typed error without leaking it to the user', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const normalized = toAppError('Error: connection to C:\\Users\\dev\\secret.env failed');
    expect(normalized.code).toBe('database');
    expect(normalized.message).not.toContain('secret.env');
    expect(normalized.retryable).toBe(true);
    spy.mockRestore();
  });

  it('handles a rejection that is null or undefined', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    for (const reason of [null, undefined, 42]) {
      expect(toAppError(reason).code).toBe('database');
    }
    spy.mockRestore();
  });

  it('reports the backend as unavailable outside a Tauri window', () => {
    expect(backendAvailable()).toBe(false);
  });

  it('reports the backend as available once the bridge exists', () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(backendAvailable()).toBe(true);
  });

  it('exposes the generated command table with the arguments each one takes', () => {
    const names = Object.keys(COMMANDS);
    expect(names.length).toBeGreaterThan(0);
    expect(names).toContain('get_app_status');
    expect(names).toContain('set_monitoring_paused');
    // The argument names are the part that decides whether a call arrives
    // with its parameters, so the table carries them rather than just names.
    expect(COMMANDS.set_collector_enabled).toEqual(['collectorId', 'enabled']);
    expect(COMMANDS.get_app_version).toEqual([]);
  });
});
