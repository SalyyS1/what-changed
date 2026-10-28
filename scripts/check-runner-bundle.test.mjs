import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { checkRunnerBundle, runnerVersions } from './check-runner-bundle.mjs';

async function fixture(fn) {
  const root = await mkdtemp(join(tmpdir(), 'ownvero-inventory-')); const bundle = join(root, 'bundle');
  try {
    const contents = new Map([['node.exe', 'inventory-test executable bytes'], ['src/main.mjs', 'throw new Error("inventory fixture must never execute");'], ['playwright-core/index.mjs', 'export const fixture = true;'], ['playwright-core/package.json', JSON.stringify({ version: runnerVersions.playwrightVersion })], [runnerVersions.browserExecutable, 'inventory-test browser bytes']]);
    const files = [];
    for (const [path, content] of contents) { await mkdir(dirname(join(bundle, path)), { recursive: true }); await writeFile(join(bundle, path), content); files.push({ path, bytes: Buffer.byteLength(content), sha256: createHash('sha256').update(content).digest('hex') }); }
    await mkdir(join(root, 'runner/src'), { recursive: true }); await writeFile(join(root, 'runner/src/main.mjs'), contents.get('src/main.mjs'));
    await writeFile(join(bundle, 'bundle-inventory.json'), JSON.stringify({ schemaVersion: 1, ...runnerVersions, installedBytes: files.reduce((sum, item) => sum + item.bytes, 0), files }));
    await fn({ root, bundle });
  } finally { await rm(root, { recursive: true, force: true }); }
}
test('inventory check hashes resources without executing Node or browser', () => fixture(async ({ root, bundle }) => {
  const result = await checkRunnerBundle(bundle, { sourceRoot: root }); assert.equal(result.fileCount, 5); assert.equal(result.nodeVersion, runnerVersions.nodeVersion);
}));
test('mutated packaged runtime and unlisted files cannot pass', () => fixture(async ({ root, bundle }) => {
  await writeFile(join(bundle, 'unlisted.env'), 'must not distribute');
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /unlisted/);
  await rm(join(bundle, 'unlisted.env')); await writeFile(join(bundle, 'node.exe'), 'changed');
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /mismatch/);
}));
test('stale runner source is rejected even when its packaged hash matches inventory', () => fixture(async ({ root, bundle }) => {
  await writeFile(join(root, 'runner/src/main.mjs'), 'new runner source');
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /stale/);
}));
test('duplicate and traversal inventory entries are rejected', () => fixture(async ({ root, bundle }) => {
  const path = join(bundle, 'bundle-inventory.json'); const manifest = JSON.parse(await readFile(path, 'utf8'));
  manifest.files.push({ ...manifest.files[0] }); await writeFile(path, JSON.stringify(manifest));
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /duplicate/);
  manifest.files[manifest.files.length - 1].path = '../outside'; await writeFile(path, JSON.stringify(manifest));
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /Unsafe/);
}));
test('missing or replaced Node checksum provenance cannot pass packaging validation', () => fixture(async ({ root, bundle }) => {
  const path = join(bundle, 'bundle-inventory.json'); const manifest = JSON.parse(await readFile(path, 'utf8'));
  delete manifest.checksumSource; await writeFile(path, JSON.stringify(manifest));
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /inventory schema/);
  manifest.checksumSource = 'https://example.com/SHASUMS256.txt'; await writeFile(path, JSON.stringify(manifest));
  await assert.rejects(checkRunnerBundle(bundle, { sourceRoot: root }), /version inventory/);
}));
