import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, readFile, readdir } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

function safeRelativePath(value) {
  return typeof value === 'string' && value.length > 0 && !isAbsolute(value)
    && !/[\\<>:"|?*\u0000-\u001f]/.test(value)
    && value.split('/').every((part) => part && part !== '.' && part !== '..'
      && !/[. ]$/.test(part) && !/^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(part));
}

async function regularDirectory(path) {
  const item = await lstat(path);
  if (!item.isDirectory() || item.isSymbolicLink()) throw new Error('Runner input directory is redirected or unsupported');
}

async function sourceFiles(directory, prefix = 'src/') {
  const files = [];
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, item.name); const name = `${prefix}${item.name}`;
    const metadata = await lstat(path);
    if (!safeRelativePath(name) || metadata.isSymbolicLink()) throw new Error(`Unsafe runner source path: ${name}`);
    if (metadata.isDirectory()) files.push(...await sourceFiles(path, `${name}/`));
    else if (metadata.isFile()) files.push(name);
    else throw new Error(`Unsupported runner source entry: ${name}`);
    if (files.length > 10000) throw new Error('Runner source inventory exceeds its limit');
  }
  return files.sort();
}

/** Read only source inputs; no runtime downloads, packaging or approval changes. */
export async function checkPreviewRuntimeInputs({ root = projectRoot } = {}) {
  const base = resolve(root);
  for (const path of [base, join(base, 'release'), join(base, 'runner'), join(base, 'runner/src')]) await regularDirectory(path);
  const inventoryPath = join(base, 'release/runner-runtime-inventory.json');
  const metadata = await lstat(inventoryPath);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size > 2 * 1024 * 1024) throw new Error('Invalid frozen runtime inventory file');
  const inventoryBytes = await readFile(inventoryPath); const inventory = JSON.parse(inventoryBytes);
  if (inventory?.schemaVersion !== 1 || !Array.isArray(inventory.files) || !inventory.files.length || inventory.files.length > 10000) throw new Error('Invalid frozen runtime inventory');
  const paths = new Set(); const owned = new Map();
  for (const entry of inventory.files) {
    if (!entry || !safeRelativePath(entry.path) || paths.has(entry.path.toLowerCase())
      || !Number.isSafeInteger(entry.bytes) || entry.bytes < 0 || !/^[a-f0-9]{64}$/.test(entry.sha256 ?? '')) throw new Error('Unsafe or duplicate frozen inventory entry');
    paths.add(entry.path.toLowerCase());
    if (entry.path.startsWith('src/')) owned.set(entry.path, entry);
  }
  const actual = await sourceFiles(join(base, 'runner/src'));
  const unlisted = actual.filter((name) => !owned.has(name));
  const missing = [...owned.keys()].filter((name) => !actual.includes(name));
  if (!owned.size || unlisted.length || missing.length) throw new Error(`Frozen runner source set differs (unlisted: ${unlisted.join(', ') || 'none'}; missing: ${missing.join(', ') || 'none'})`);
  let sourceBytes = 0;
  for (const name of actual) {
    const path = join(base, 'runner', name); const item = await lstat(path); const expected = owned.get(name);
    const hash = createHash('sha256');
    for await (const bytes of createReadStream(path)) hash.update(bytes);
    const actualHash = hash.digest('hex');
    if (item.size !== expected.bytes || actualHash !== expected.sha256) throw new Error(`Frozen runner source differs: ${name} (expected ${expected.bytes} bytes/${expected.sha256}; actual ${item.size} bytes/${actualHash})`);
    sourceBytes += item.size;
  }
  return { inventorySha256: digest(inventoryBytes), sourceFiles: actual.length, sourceBytes };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 2) throw new Error('Usage: node scripts/check-preview-runtime-inputs.mjs');
    console.log(JSON.stringify({ check: 'frozen-runner-source-inputs', ...await checkPreviewRuntimeInputs() }));
  } catch (error) {
    console.error(`Preview runtime input preflight blocked: ${error.message}`);
    console.error('Owner review of the exact source delta is required before changing frozen runtime inputs. Existing notice and publication approval gates still apply.');
    process.exitCode = 1;
  }
}
