/**
 * WCAG 2.x contrast arithmetic. Pure functions over hex strings so the
 * palette can be asserted in a unit test with no DOM.
 */

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** Parses `#rgb` or `#rrggbb`. Throws on anything else — a malformed token is a build defect. */
export function hexToRgb(hex: string): Rgb {
  const m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) {
    throw new Error(`not a hex colour: ${hex}`);
  }
  const digits = m[1] ?? '';
  const h = digits.length === 3 ? [...digits].map((c) => c + c).join('') : digits;
  return {
    r: parseInt(h.slice(0, 2), 16),
    g: parseInt(h.slice(2, 4), 16),
    b: parseInt(h.slice(4, 6), 16),
  };
}

function channel(v: number): number {
  const s = v / 255;
  return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

/** Relative luminance per WCAG 2.1 §1.4.3 definition. */
export function relativeLuminance(hex: string): number {
  const { r, g, b } = hexToRgb(hex);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** Contrast ratio in [1, 21]; order of arguments does not matter. */
export function contrastRatio(a: string, b: string): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

export function meetsRatio(fg: string, bg: string, min: number): boolean {
  return contrastRatio(fg, bg) >= min;
}
