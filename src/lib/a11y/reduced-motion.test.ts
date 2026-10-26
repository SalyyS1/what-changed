import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..', '..', '..');

describe('Phase 20 reduced-motion structural safeguards', () => {
  it('neutralises CSS transitions and animations under the media feature', () => {
    const css = readFileSync(join(root, 'src', 'styles', 'motion.css'), 'utf8');
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).toMatch(/animation-duration:\s*0\.001ms\s*!important/);
    expect(css).toMatch(/transition-duration:\s*0\.001ms\s*!important/);
    expect(css).toContain('scroll-behavior: auto');
  });

  it('pauses both map renderers when the preference is active', () => {
    const changeMap = readFileSync(join(root, 'src', 'components', 'visualization', 'ChangeMapFrame.svelte'), 'utf8');
    const systemMap = readFileSync(join(root, 'src', 'components', 'system', 'SystemMapFrame.svelte'), 'utf8');
    expect(changeMap).toContain("prefers-reduced-motion: reduce");
    expect(systemMap).toContain("prefers-reduced-motion: reduce");
    expect(changeMap).toContain('reducedMotion');
    expect(systemMap).toContain('reducedMotion');
  });
});

