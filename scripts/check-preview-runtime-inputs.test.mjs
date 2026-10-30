import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { checkPreviewRuntimeInputs } from './check-preview-runtime-inputs.mjs';

const record = (path, text) => ({ path, bytes: Buffer.byteLength(text), sha256: createHash('sha256').update(text).digest('hex') });

async function fixture(fn) {
  const root = await mkdtemp(join(tmpdir(), 'ownvero-input-preflight-'));
  const inventoryPath = join(root, 'release/runner-runtime-inventory.json');
  const contents = new Map([['src/main.mjs', 'throw new Error("source must never execute");\n'], ['src/nested/policy.mjs', 'export const allowed = false;\n']]);
  try {
    const files = [record('node.exe', 'vendor bytes need not exist for a source preflight')];
    for (const [path, text] of contents) {
      const source = join(root, 'runner', path);
      await mkdir(dirname(source), { recursive: true }); await writeFile(source, text); files.push(record(path, text));
    }
    await mkdir(dirname(inventoryPath), { recursive: true });
    await writeFile(inventoryPath, JSON.stringify({ schemaVersion: 1, files }));
    const changeInventory = async (change) => {
      const inventory = JSON.parse(await readFile(inventoryPath, 'utf8'));
      change(inventory); await writeFile(inventoryPath, JSON.stringify(inventory));
    };
    await fn({ root, inventoryPath, changeInventory });
  } finally { await rm(root, { recursive: true, force: true }); }
}

test('matching frozen sources pass without executing source or requiring runtime files', () => fixture(async ({ root, inventoryPath }) => {
  const before = await readFile(inventoryPath); const result = await checkPreviewRuntimeInputs({ root });
  assert.equal(result.sourceFiles, 2);
  assert.equal(result.inventorySha256, createHash('sha256').update(before).digest('hex'));
  assert.deepEqual(await readFile(inventoryPath), before);
}));

test('changed source fails even when its byte length is unchanged', () => fixture(async ({ root }) => {
  const path = join(root, 'runner/src/nested/policy.mjs'); const before = await readFile(path);
  await writeFile(path, 'export const allowed = true; \n');
  assert.equal((await readFile(path)).length, before.length);
  await assert.rejects(checkPreviewRuntimeInputs({ root }), /Frozen runner source differs: src\/nested\/policy\.mjs/);
}));

test('a source omitted from the frozen inventory fails', () => fixture(async ({ root, changeInventory }) => {
  await changeInventory((inventory) => { inventory.files = inventory.files.filter((entry) => entry.path !== 'src/nested/policy.mjs'); });
  await assert.rejects(checkPreviewRuntimeInputs({ root }), /source set differs.*unlisted: src\/nested\/policy\.mjs/);
}));

test('an extra frozen source that no longer exists fails', () => fixture(async ({ root, changeInventory }) => {
  await changeInventory((inventory) => { inventory.files.push(record('src/removed.mjs', 'old source')); });
  await assert.rejects(checkPreviewRuntimeInputs({ root }), /source set differs.*missing: src\/removed\.mjs/);
}));

test('unsafe relative paths and Windows aliases fail before source comparison', () => fixture(async ({ root, changeInventory }) => {
  for (const path of ['../outside.mjs', 'src/../../outside.mjs', '/absolute.mjs', 'C:/outside.mjs', 'src\\outside.mjs', 'src//outside.mjs', 'src/NUL.mjs', 'src/file.mjs:stream', 'src/file.mjs.']) {
    await changeInventory((inventory) => { inventory.files[0].path = path; });
    await assert.rejects(checkPreviewRuntimeInputs({ root }), /Unsafe or duplicate frozen inventory entry/, path);
  }
}));

test('duplicate paths cannot hide source inventory changes on Windows', () => fixture(async ({ root, changeInventory }) => {
  await changeInventory((inventory) => { inventory.files.push({ ...inventory.files[1], path: 'SRC/MAIN.MJS' }); });
  await assert.rejects(checkPreviewRuntimeInputs({ root }), /Unsafe or duplicate frozen inventory entry/);
}));

test('a redirected source directory cannot borrow matching bytes', () => fixture(async ({ root }) => {
  const target = join(root, 'external'); await mkdir(target);
  await writeFile(join(target, 'policy.mjs'), 'export const allowed = false;\n');
  await rm(join(root, 'runner/src/nested'), { recursive: true });
  await symlink(target, join(root, 'runner/src/nested'), 'junction');
  await assert.rejects(checkPreviewRuntimeInputs({ root }), /Unsafe runner source path/);
}));
