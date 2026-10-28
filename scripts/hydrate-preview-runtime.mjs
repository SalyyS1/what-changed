import { mkdir, copyFile, readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { checkRunnerBundle } from './check-runner-bundle.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
if (process.platform !== 'win32') throw new Error('Runtime packaging requires Windows x64');
const inventoryBytes = await readFile(resolve(root, 'release/runner-runtime-inventory.json'));
const inventory = JSON.parse(inventoryBytes);
const bundle = resolve(root, 'runner/bundle');
const nodeRecord = inventory.files.find((item) => item.path === 'node.exe');
const official = await fetch(inventory.checksumSource);
if (!official.ok) throw new Error('Official Node checksum inventory unavailable');
const checksums = await official.text();
const expected = checksums.split('\n').find((line) => /\swin-x64\/node\.exe\r?$/.test(line))?.trim().split(/\s+/)[0];
if (expected !== nodeRecord?.sha256) throw new Error('Reviewed Node checksum differs from official source');
await mkdir(bundle, { recursive: true });
const nodeDownload = await fetch('https://nodejs.org/dist/v24.12.0/win-x64/node.exe');
if (!nodeDownload.ok) throw new Error('Pinned Node download unavailable');
const nodeBytes = Buffer.from(await nodeDownload.arrayBuffer());
if (nodeBytes.length !== nodeRecord.bytes || createHash('sha256').update(nodeBytes).digest('hex') !== expected) throw new Error('Downloaded Node bytes differ');
await writeFile(join(bundle, 'node.exe'), nodeBytes, { flag: 'wx' });
const browserCache = resolve(root, '.qa/hosted-browser-cache');
execFileSync(process.execPath, [resolve(root, 'node_modules/playwright-core/cli.js'), 'install', 'chromium'], {
  cwd: root, env: { ...process.env, PLAYWRIGHT_BROWSERS_PATH: browserCache, PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD: '0' }, stdio: 'inherit',
});
for (const entry of inventory.files) {
  if (entry.path === 'node.exe') continue;
  let source;
  if (entry.path.startsWith('src/')) source = resolve(root, 'runner', entry.path);
  else if (entry.path.startsWith('playwright-core/')) source = resolve(root, 'node_modules', entry.path);
  else if (entry.path.startsWith('browsers/')) source = resolve(browserCache, entry.path.slice('browsers/'.length));
  else throw new Error('Unsupported reviewed runtime entry');
  const output = join(bundle, entry.path);
  await mkdir(dirname(output), { recursive: true });
  await copyFile(source, output);
}
await writeFile(join(bundle, 'bundle-inventory.json'), inventoryBytes, { flag: 'wx' });
console.log(JSON.stringify(await checkRunnerBundle(bundle)));
