import { mkdir, mkdtemp, copyFile, readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname, join, posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkRunnerBundle, runnerVersions } from './check-runner-bundle.mjs';
import { downloadVerifiedArtifact, expandReviewedRuntimeArchive } from './fetch-reviewed-runtime-artifact.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
if (process.platform !== 'win32' || process.arch !== 'x64') throw new Error('Runtime packaging requires Windows x64');
const inventoryBytes = await readFile(resolve(root, 'release/runner-runtime-inventory.json'));
const inventory = JSON.parse(inventoryBytes);
const browser = JSON.parse(await readFile(resolve(root, 'release/runner-browser-source.json'), 'utf8'));
if (browser.schemaVersion !== 1 || browser.chromiumRevision !== runnerVersions.chromiumRevision
  || browser.browserExecutable !== runnerVersions.browserExecutable
  || Object.entries(runnerVersions).some(([key, value]) => inventory[key] !== value)) throw new Error('Browser source descriptor and runtime inventory differ');
const browserPrefix = `${posix.dirname(browser.browserExecutable)}/`;
const excludedArchiveFiles = new Set((browser.excludedArchiveFiles ?? []).map((entry) => entry.path));
if (excludedArchiveFiles.has(posix.basename(browser.browserExecutable))) throw new Error('Browser executable cannot be excluded');
const bundle = resolve(root, 'runner/bundle');
const nodeRecord = inventory.files.find((item) => item.path === 'node.exe');
const official = await fetch(runnerVersions.checksumSource);
if (!official.ok) throw new Error('Official Node checksum inventory unavailable');
const checksums = await official.text();
const expected = checksums.split('\n').find((line) => /\swin-x64\/node\.exe\r?$/.test(line))?.trim().split(/\s+/)[0];
if (expected !== nodeRecord?.sha256) throw new Error('Reviewed Node checksum differs from official source');
// A fresh destination prevents a failed hydration from reusing a previous browser.
await mkdir(bundle);
await downloadVerifiedArtifact({ url: `https://nodejs.org/dist/${runnerVersions.nodeVersion}/win-x64/node.exe`, bytes: nodeRecord.bytes, sha256: expected }, join(bundle, 'node.exe'));
await mkdir(resolve(root, '.qa'), { recursive: true });
const cache = await mkdtemp(resolve(root, '.qa/hosted-browser-'));
const archive = join(cache, 'browser.zip');
await downloadVerifiedArtifact(browser.archive, archive);
const extracted = join(cache, 'extracted');
await expandReviewedRuntimeArchive(archive, extracted, { sha256: browser.archive.sha256, expectedRoot: browser.archiveRoot });
for (const entry of inventory.files) {
  if (entry.path === 'node.exe') continue;
  let source;
  if (entry.path.startsWith('src/')) source = resolve(root, 'runner', entry.path);
  else if (entry.path.startsWith('playwright-core/')) source = resolve(root, 'node_modules', entry.path);
  else if (entry.path.startsWith(browserPrefix)) source = resolve(extracted, browser.archiveRoot, entry.path.slice(browserPrefix.length));
  else throw new Error('Unsupported reviewed runtime entry');
  if (entry.path.startsWith(browserPrefix) && excludedArchiveFiles.has(posix.basename(entry.path))) throw new Error(`Excluded browser file is still in runtime inventory: ${entry.path}`);
  const output = join(bundle, entry.path);
  await mkdir(dirname(output), { recursive: true });
  await copyFile(source, output);
}
await writeFile(join(bundle, 'bundle-inventory.json'), inventoryBytes, { flag: 'wx' });
console.log(JSON.stringify(await checkRunnerBundle(bundle)));
