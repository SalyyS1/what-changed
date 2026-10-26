import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..', '..', '..');
const screens = [
  'home/HomeScreen.svelte',
  'timeline/TimelineScreen.svelte',
  'investigation/InvestigationsScreen.svelte',
  'checkpoints/CheckpointsScreen.svelte',
  'settings/SettingsScreen.svelte',
  'diagnostics/DiagnosticsScreen.svelte',
  'system-map/SystemMapScreen.svelte',
] as const;

describe('Phase 20 keyboard and landmark structural safeguards', () => {
  it('routes every primary screen through the shared ScreenFrame', () => {
    for (const screen of screens) {
      const source = readFileSync(join(root, 'src', 'features', screen), 'utf8');
      expect(source, screen).toContain('ScreenFrame');
    }
  });

  it('declares a focusable shell main landmark and a global focus ring', () => {
    const shell = readFileSync(join(root, 'src', 'app', 'AppShell.svelte'), 'utf8');
    const styles = readFileSync(join(root, 'src', 'styles', 'app.css'), 'utf8');
    expect(shell).toContain('<main');
    expect(shell).toContain('tabindex="-1"');
    expect(styles).toMatch(/:focus-visible\s*\{/);
  });

  it('keeps the shell as the sole main landmark across primary screens', () => {
    const shell = readFileSync(join(root, 'src', 'app', 'AppShell.svelte'), 'utf8');
    expect(shell.match(/<main(?=[\s/>])/g)).toHaveLength(1);
    for (const screen of screens) {
      const source = readFileSync(join(root, 'src', 'features', screen), 'utf8');
      expect(source, screen).not.toMatch(/<main(?=[\s/>])/);
    }
  });
});

