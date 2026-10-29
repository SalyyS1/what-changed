import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { contrastRatio, hexToRgb, relativeLuminance } from './contrast';
import { palettes, textPairs, type ThemeName } from './tokens';

const ROOT = join(import.meta.dirname, '..', '..');

describe('contrast arithmetic', () => {
  it('matches the WCAG reference values', () => {
    expect(relativeLuminance('#ffffff')).toBeCloseTo(1, 6);
    expect(relativeLuminance('#000000')).toBeCloseTo(0, 6);
    expect(contrastRatio('#ffffff', '#000000')).toBeCloseTo(21, 6);
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 6);
    expect(contrastRatio('#777777', '#ffffff')).toBeCloseTo(4.48, 2);
  });

  it('expands short hex and rejects garbage', () => {
    expect(hexToRgb('#abc')).toEqual({ r: 0xaa, g: 0xbb, b: 0xcc });
    expect(() => hexToRgb('red')).toThrow();
    expect(() => hexToRgb('#12345')).toThrow();
  });
});

describe.each<ThemeName>(['dark', 'light'])('%s palette', (theme) => {
  const p = palettes[theme];

  it.each(textPairs.map((pair) => [pair.fg, pair.bg, pair.minRatio] as const))(
    '%s on %s >= %s:1',
    (fg, bg, min) => {
      const ratio = contrastRatio(p[fg], p[bg]);
      expect(ratio, `${theme}: ${fg} on ${bg} is ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(min);
    },
  );

  it('fails if a text token is darkened toward its surface', () => {
    // Mutation check: push every text token 40% toward black (dark) or white
    // (light) and require the same pair table to report at least one failure.
    // If this ever passes, the table has stopped guarding the palette.
    const towards = theme === 'dark' ? 0 : 255;
    const mutate = (hex: string) =>
      '#' +
      [1, 3, 5]
        .map((i) => Math.round(parseInt(hex.slice(i, i + 2), 16) * 0.6 + towards * 0.4))
        .map((c) => c.toString(16).padStart(2, '0'))
        .join('');
    const failures = textPairs.filter(({ fg, bg, minRatio }) => {
      const mutated = fg === 'accent-contrast' ? p[fg] : mutate(p[fg]);
      return contrastRatio(mutated, p[bg]) < minRatio;
    });
    expect(failures.length).toBeGreaterThan(0);
  });
});

describe('token stylesheet parity', () => {
  // Normalised so a CRLF checkout cannot break the selector match.
  const css = readFileSync(join(ROOT, 'styles', 'tokens.css'), 'utf8').replaceAll('\r\n', '\n');

  function block(selector: string): string {
    const start = css.indexOf(selector);
    expect(start, `selector ${selector} present`).toBeGreaterThan(-1);
    const open = css.indexOf('{', start);
    const close = css.indexOf('}', open);
    return css.slice(open, close);
  }

  it.each<ThemeName>(['dark', 'light'])('%s values in tokens.ts match tokens.css', (theme) => {
    const b = block(theme === 'dark' ? ":root,\n[data-theme='dark']" : "[data-theme='light']");
    for (const [name, value] of Object.entries(palettes[theme])) {
      const m = new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{6});`).exec(b);
      expect(m, `--${name} declared for ${theme}`).not.toBeNull();
      expect(m?.[1]?.toLowerCase()).toBe(value.toLowerCase());
    }
  });
});

describe('no raw hex outside the token files', () => {
  function walk(dir: string, out: string[] = []): string[] {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) {
        walk(p, out);
      } else if (/\.(svelte|ts|css)$/.test(name) && !/\.test\.ts$/.test(name)) {
        out.push(p);
      }
    }
    return out;
  }

  const allowlist = new Set([
    'styles/tokens.css',
    'lib/design/tokens.ts',
  ]);

  const isColourPosition = (src: string, h: string) =>
    new RegExp(`(:|\\s|\\(|,|\\[)${h}(;|\\s|\\)|,|\\]|$)`).test(src);

  it('recognises a hex inside a Tailwind arbitrary value as a colour', () => {
    expect(isColourPosition('<div class="bg-[#123456] text-[#fff]">', '#123456')).toBe(true);
    expect(isColourPosition('<div class="bg-[#123456] text-[#fff]">', '#fff')).toBe(true);
    expect(isColourPosition('<a href="#abc">', '#abc')).toBe(false);
  });

  it('every colour literal lives in tokens.css or tokens.ts', () => {
    const offenders: string[] = [];
    for (const file of walk(ROOT)) {
      const rel = relative(ROOT, file).replaceAll('\\', '/');
      if (allowlist.has(rel)) {
        continue;
      }
      const src = readFileSync(file, 'utf8');
      const hits = src.match(/#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{3}\b(?![0-9a-fA-F])/g) ?? [];
      // `#/` hash routes and `#app` ids are not colours; the regex above already
      // requires hex digits, but `#abc` could be an id in a selector — so only
      // count matches that sit in a colour position, which includes Tailwind's
      // arbitrary-value brackets (`bg-[#123456]`).
      const real = hits.filter((h: string) => isColourPosition(src, h));
      if (real.length > 0) {
        offenders.push(`${rel}: ${real.join(' ')}`);
      }
    }
    expect(offenders, offenders.join('\n')).toEqual([]);
  });
});
