/**
 * The two timings that cannot live in CSS: bits-ui takes the tooltip delay as
 * a number, and the toast lifetime is a `setTimeout`. Everything else in the
 * app reads `--t-*` from motion.css, so these are the only durations written
 * in TypeScript and they are written once.
 *
 * They are deliberately not multiplied by `--motion-scale`: reduced motion
 * asks for less movement, not for hints that vanish or messages that never
 * finish being read.
 */

/** Hover dwell before a tooltip appears. */
export const TOOLTIP_DELAY_MS = 450;

/** How long a toast stays before it dismisses itself. */
export const TOAST_TTL_MS = 4000;
