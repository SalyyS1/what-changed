import { Color } from 'three';

export interface PaletteToken {
  readonly token: string;
  readonly fallbackToken: string;
}

export type ThreePalette<TKey extends string> = Readonly<Record<TKey, Color>>;

export function readThreePalette<TKey extends string>(
  tokens: Readonly<Record<TKey, PaletteToken>>,
  root: Element = document.documentElement,
): ThreePalette<TKey> {
  const styles = getComputedStyle(root);
  const palette = {} as Record<TKey, Color>;
  for (const key of Object.keys(tokens) as TKey[]) {
    const mapping = tokens[key];
    const primary = styles.getPropertyValue(mapping.token).trim();
    const fallback = styles.getPropertyValue(mapping.fallbackToken).trim();
    palette[key] = safeColor(primary || fallback || 'gray');
  }
  return palette;
}

function safeColor(value: string): Color {
  try {
    return new Color(value);
  } catch {
    return new Color('gray');
  }
}
