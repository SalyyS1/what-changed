/**
 * The accessibility manifest consumed by the Phase 20 gate.
 *
 * Keeping the manifest as token names (rather than copied hex values) means a
 * palette change cannot accidentally bypass the contrast test.  Components
 * may use only pairs present here for normal-size text.
 */
import { textPairs, type TextPair } from '../design/tokens';

export const contrastPairs: readonly TextPair[] = textPairs;

export const requiredContrastRatio = 4.5;

