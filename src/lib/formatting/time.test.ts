import { describe, expect, it } from 'vitest';
import { formatCorrelation } from './relative';
import { formatDayHeader, formatDuration } from './time';

describe('time formatting', () => {
  it('names today, yesterday and older local days', () => {
    const now = new Date(2025, 7, 23, 12).getTime();
    expect(formatDayHeader(new Date(2025, 7, 23, 9).getTime(), now)).toBe('Today');
    expect(formatDayHeader(new Date(2025, 7, 22, 9).getTime(), now)).toBe('Yesterday');
    expect(formatDayHeader(new Date(2025, 7, 21, 9).getTime(), now)).toContain('Aug 21');
  });

  it('uses calm duration and correlation wording', () => {
    expect(formatDuration(1)).toBe('1 minute');
    expect(formatDuration(34)).toBe('34 minutes');
    expect(formatDuration(120)).toBe('2 hours');
    expect(formatCorrelation(34, 'first game crash')).toBe('34 minutes before first game crash');
  });
});
