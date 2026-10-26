import { describe, expect, it } from 'vitest';
import { FrameTiming, HEALTHY_FRAME_CADENCE_MS, QUALITY_SETTINGS, qualityAfterTiming } from './quality';

describe('Change Map quality tiers', () => {
  it('low quality reduces sphere segments and thins temporal edges', () => {
    expect(QUALITY_SETTINGS.low.sphereWidthSegments).toBeLessThan(QUALITY_SETTINGS.high.sphereWidthSegments);
    expect(QUALITY_SETTINGS.low.sphereHeightSegments).toBeLessThan(QUALITY_SETTINGS.high.sphereHeightSegments);
    expect(QUALITY_SETTINGS.low.temporalEdgeStride).toBeGreaterThan(QUALITY_SETTINGS.high.temporalEdgeStride);
  });

  it('records the median of a 300-frame timing window', () => {
    const timing = new FrameTiming(300);
    let timestamp = 1;
    timing.frame(timestamp);
    for (let frame = 1; frame <= 300; frame += 1) {
      timestamp += 19 + (frame % 3);
      timing.frame(timestamp);
    }
    expect(timing.sampleCount).toBe(300);
    expect(timing.median()).toBe(20);
  });

  it('does not count a paused background gap as a frame interval', () => {
    const timing = new FrameTiming(3);
    timing.frame(10);
    timing.frame(26);
    timing.pause();
    timing.frame(10_000);
    timing.frame(10_016);
    expect(timing.sampleCount).toBe(2);
    expect(timing.median()).toBe(16);
  });

  it('drops to the low tier only after the frame budget is exceeded', () => {
    expect(qualityAfterTiming('high', 16.8)).toBe('high');
    expect(qualityAfterTiming('high', HEALTHY_FRAME_CADENCE_MS)).toBe('high');
    expect(qualityAfterTiming('high', HEALTHY_FRAME_CADENCE_MS + 0.01)).toBe('low');
    expect(qualityAfterTiming('low', 40)).toBe('low');
  });
});
