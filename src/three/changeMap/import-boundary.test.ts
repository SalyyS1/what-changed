import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? sourceFiles(path) : /\.(?:ts|svelte)$/.test(entry.name) ? [path] : [];
  });
}

describe('Three.js import boundary', () => {
  it('keeps direct Three.js imports inside src/three', () => {
    const root = join(process.cwd(), 'src');
    const offenders = sourceFiles(root)
      .filter((path) => !relative(root, path).replaceAll('\\', '/').startsWith('three/'))
      .filter((path) => /from\s+['"]three['"]/.test(readFileSync(path, 'utf8')))
      .map((path) => relative(root, path));
    expect(offenders).toEqual([]);
  });
});
