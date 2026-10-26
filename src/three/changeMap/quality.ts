export type QualityTier = 'high' | 'low';

export interface QualitySettings {
  readonly sphereWidthSegments: number;
  readonly sphereHeightSegments: number;
  readonly temporalEdgeStride: number;
}

export const QUALITY_SETTINGS: Readonly<Record<QualityTier, QualitySettings>> = {
  high: { sphereWidthSegments: 12, sphereHeightSegments: 8, temporalEdgeStride: 1 },
  low: { sphereWidthSegments: 8, sphereHeightSegments: 6, temporalEdgeStride: 2 },
};

/** One 60 Hz refresh interval plus timer quantization and scheduler tolerance. */
export const HEALTHY_FRAME_CADENCE_MS = 17.5;

export function qualityAfterTiming(current: QualityTier, medianMs: number): QualityTier {
  return current === 'high' && medianMs > HEALTHY_FRAME_CADENCE_MS ? 'low' : current;
}

export class FrameTiming {
  readonly #samples: Float32Array;
  #count = 0;
  #cursor = 0;
  #previousFrame: number | null = null;

  constructor(capacity = 300) {
    this.#samples = new Float32Array(Math.max(1, capacity));
  }

  /**
   * Records the interval between consecutive animation-frame callbacks.
   * Renderer CPU duration is deliberately excluded: quality follows the
   * cadence the user sees, including GPU presentation and browser scheduling.
   */
  frame(timestamp: number): boolean {
    if (!Number.isFinite(timestamp)) return false;
    if (this.#previousFrame === null) {
      this.#previousFrame = timestamp;
      return false;
    }
    const interval = timestamp - this.#previousFrame;
    this.#previousFrame = timestamp;
    if (interval <= 0) return false;
    this.#samples[this.#cursor] = interval;
    this.#cursor = (this.#cursor + 1) % this.#samples.length;
    this.#count = Math.min(this.#count + 1, this.#samples.length);
    return true;
  }

  /** Prevent a hidden/inactive gap from being counted as a slow frame. */
  pause(): void { this.#previousFrame = null; }

  median(): number {
    if (this.#count === 0) return 0;
    const copy = Array.from(this.#samples.subarray(0, this.#count));
    copy.sort((left, right) => left - right);
    const middle = Math.floor(copy.length / 2);
    const current = copy[middle] ?? 0;
    return copy.length % 2 === 0 ? ((copy[middle - 1] ?? current) + current) / 2 : current;
  }

  get sampleCount(): number { return this.#count; }
}
