import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const tests = readdirSync(resolve(root, 'runner/tests')).filter((name) => name.endsWith('.test.mjs')).sort().map((name) => `runner/tests/${name}`);
if (!tests.length) throw new Error('No runner policy tests were discovered');
const result = spawnSync(process.execPath, ['--test', ...tests], { cwd: root, stdio: 'inherit' });
if (result.error || result.status !== 0) { console.error('Runner source tests failed'); process.exitCode = result.status || 1; }
else console.log('Runner source policy/recipe/protocol tests passed. Installed network confinement and disabled-mode observations remain separate required evidence.');
