import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, readFile, readdir, realpath } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

export const runnerVersions = Object.freeze({ nodeVersion: 'v24.12.0', playwrightVersion: '1.62.1', chromiumRevision: '156.0.8078.47-1.1', browserExecutable: 'browsers/chromium-156.0.8078.47-1.1/chrome.exe', checksumSource: 'https://nodejs.org/dist/v24.12.0/SHASUMS256.txt' });
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
async function sha256(path) { const hash = createHash('sha256'); for await (const bytes of createReadStream(path)) hash.update(bytes); return hash.digest('hex'); }

async function regularFiles(directory, prefix = '') {
  const files = [];
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, item.name); const name = `${prefix}${item.name}`;
    if ((await lstat(path)).isSymbolicLink()) throw new Error('Runner bundle contains a redirected entry');
    if (item.isDirectory()) files.push(...await regularFiles(path, `${name}/`));
    else if (item.isFile()) files.push(name);
    else throw new Error('Runner bundle contains an unsupported entry');
    if (files.length > 10000) throw new Error('Runner bundle file count exceeds its limit');
  }
  return files.sort();
}

/** Checks byte inventory and source parity. It does not execute or certify a browser. */
export async function checkRunnerBundle(directory, { sourceRoot = root } = {}) {
  const base = resolve(directory);
  for (let current = base; ; current = dirname(current)) {
    if ((await lstat(current)).isSymbolicLink()) throw new Error('Runner bundle root is redirected');
    if (dirname(current) === current) break;
  }
  // Windows temp paths can contain 8.3 aliases; compare filesystem-resolved
  // paths on both sides after auditing the original ancestors for redirects.
  const canonicalBase = await realpath(base);
  const manifestFile = join(base, 'bundle-inventory.json');
  if ((await lstat(manifestFile)).size > 2 * 1024 * 1024) throw new Error('Runner inventory too large');
  const inventory = JSON.parse(await readFile(manifestFile, 'utf8'));
  const fields = ['schemaVersion', 'nodeVersion', 'playwrightVersion', 'chromiumRevision', 'browserExecutable', 'checksumSource', 'installedBytes', 'files'];
  if (!inventory || typeof inventory !== 'object' || Object.keys(inventory).some((key) => !fields.includes(key)) || fields.some((key) => !(key in inventory))) throw new Error('Invalid runner inventory schema');
  if (inventory.schemaVersion !== 1 || Object.entries(runnerVersions).some(([key, value]) => inventory[key] !== value)) throw new Error('Runner version inventory mismatch');
  if (!Array.isArray(inventory.files) || !inventory.files.length || inventory.files.length > 10000) throw new Error('Invalid runner file inventory');
  const listed = new Set(); let bytes = 0;
  for (const entry of inventory.files) {
    if (!entry || typeof entry !== 'object' || Object.keys(entry).some((key) => !['path', 'bytes', 'sha256'].includes(key)) || typeof entry.path !== 'string' || entry.path.includes('\\') || isAbsolute(entry.path) || entry.path.split('/').some((part) => !part || part === '.' || part === '..') || listed.has(entry.path)
      || !Number.isSafeInteger(entry.bytes) || entry.bytes < 0 || !/^[a-f0-9]{64}$/.test(entry.sha256)) throw new Error('Unsafe or duplicate runner inventory entry');
    listed.add(entry.path); bytes += entry.bytes;
    if (bytes > 1536 * 1024 * 1024) throw new Error('Runner exceeds reviewed distribution budget');
    const path = join(base, entry.path);
    const metadata = await lstat(path);
    const ownedRelative = relative(canonicalBase, await realpath(path));
    if (!metadata.isFile() || metadata.isSymbolicLink() || ownedRelative === '..' || ownedRelative.startsWith(`..${sep}`) || isAbsolute(ownedRelative)) throw new Error('Runner entry is not a regular owned file');
    if (metadata.size !== entry.bytes || await sha256(path) !== entry.sha256) throw new Error(`Runner byte mismatch: ${entry.path}`);
    if (entry.path.startsWith('src/')) {
      const source = join(sourceRoot, 'runner', entry.path);
      if (await sha256(source) !== entry.sha256) throw new Error(`Runner source is stale: ${entry.path}`);
    }
  }
  for (const required of ['node.exe', 'src/main.mjs', 'playwright-core/index.mjs', runnerVersions.browserExecutable]) if (!listed.has(required)) throw new Error(`Runner missing ${required}`);
  const expectedSources = await regularFiles(join(sourceRoot, 'runner', 'src'));
  if (expectedSources.some((name) => !listed.has(`src/${name}`))) throw new Error('Runner source inventory is incomplete');
  const actual = (await regularFiles(base)).filter((name) => name !== 'bundle-inventory.json');
  if (actual.length !== listed.size || actual.some((name) => !listed.has(name))) throw new Error('Runner bundle contains unlisted files');
  if (inventory.installedBytes !== bytes) throw new Error('Runner size sum does not match');
  const packageInfo = JSON.parse(await readFile(join(base, 'playwright-core/package.json'), 'utf8'));
  if (packageInfo.version !== runnerVersions.playwrightVersion) throw new Error('Packaged Playwright version differs');
  return { bytes, fileCount: listed.size, inventorySha256: await sha256(manifestFile), ...runnerVersions };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { console.log(JSON.stringify({ check: 'runner-byte-inventory', ...(await checkRunnerBundle(process.argv[2] ?? join(root, 'runner/bundle'))), installedRuntimeProven: false, confinementProven: false })); }
  catch (error) { console.error(`Runner bundle unavailable: ${error.message}`); process.exitCode = 1; }
}
