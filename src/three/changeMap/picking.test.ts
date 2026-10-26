import { describe, expect, it } from 'vitest';
import { chainMembership, pointerToNdc, PointerClickGuard } from '../shared/picking';
import type { ChangeMapNodeLike } from './types';

describe('pointerToNdc', () => {
  it.each([1, 1.5, 2])('uses the CSS bounding rect at DPR %s', (dpr) => {
    Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: dpr });
    const rect = new DOMRect(100, 50, 400, 200);
    expect(pointerToNdc(300, 150, rect)).toEqual({ x: 0, y: 0 });
    expect(pointerToNdc(100, 50, rect)).toEqual({ x: -1, y: 1 });
    expect(pointerToNdc(500, 250, rect)).toEqual({ x: 1, y: -1 });
  });
});

describe('chainMembership', () => {
  it('returns every chain attached to an instance and null for unchained nodes', () => {
    const base: ChangeMapNodeLike = {
      id: 'node', event_id: 1, timestamp_ms: 0, category: 'system', lane: 'system',
      importance: 'important', label: 'Node', chain_ids: [], chain_depth: 0,
    };
    expect(chainMembership(base)).toBeNull();
    expect([...(chainMembership({ ...base, chain_ids: ['a', 'b'] }) ?? [])]).toEqual(['a', 'b']);
  });
});

describe('PointerClickGuard', () => {
  it('accepts a stationary press and suppresses orbit drags', () => {
    const guard = new PointerClickGuard();
    guard.press(10, 10);
    guard.move(12, 12);
    expect(guard.release()).toBe(true);
    guard.press(10, 10);
    guard.move(20, 10);
    expect(guard.release()).toBe(false);
  });
});
