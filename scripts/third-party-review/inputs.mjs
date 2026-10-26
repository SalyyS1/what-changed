import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, realpathSync } from 'node:fs';
import { resolve } from 'node:path';
import { CARGO_EDGES, TARGET, compare, packageKey, readJson, sha256, validateIdentity } from './filesystem.mjs';

const INPUT_FILES = {
  cargoLockSha256: 'src-tauri/Cargo.lock', bunLockSha256: 'bun.lock',
  cargoManifestSha256: 'src-tauri/Cargo.toml', frontendInputsSha256: 'dist-installer/frontend-inputs.json',
};

function captureFiles(root) {
  return Object.fromEntries(Object.entries(INPUT_FILES).map(([key, file]) => [key, readFileSync(resolve(root, file))]));
}

function fileHashes(files) {
  return Object.fromEntries(Object.entries(files).map(([key, bytes]) => [key, sha256(bytes)]));
}

export function verifyReviewInputsUnchanged(root, expected, { cargoTree, cargoMetadata, inputSnapshot } = {}) {
  const actual = fileHashes(captureFiles(root));
  for (const [key, path] of [['cargoTreeSha256', cargoTree], ['cargoMetadataSha256', cargoMetadata]]) {
    if (path) actual[key] = sha256(readFileSync(path));
  }
  for (const [key, hash] of Object.entries(actual)) {
    if (expected[key] !== hash) throw new Error(`review inputs changed before output write: ${key}`);
  }
  if (inputSnapshot) {
    const snapshot = readJson(inputSnapshot, 'input snapshot');
    for (const [key, value] of Object.entries(expected)) {
      if (snapshot[key] !== value) throw new Error(`input snapshot is stale or invalid for ${key}`);
    }
  }
}

function runCargo(root, args) {
  return execFileSync('cargo', args, { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
}

export function captureReviewInputs({ root, cargoTree, cargoMetadata, inputSnapshot } = {}) {
  const captured = captureFiles(root);
  const before = fileHashes(captured);
  let treeBytes;
  let metadataBytes;
  if (cargoTree || cargoMetadata || inputSnapshot) {
    if (!cargoTree || !cargoMetadata || !inputSnapshot) {
      throw new Error('precomputed inputs require --cargo-tree, --cargo-metadata and --input-snapshot together');
    }
    treeBytes = readFileSync(cargoTree);
    metadataBytes = readFileSync(cargoMetadata);
  } else {
    treeBytes = Buffer.from(runCargo(root, ['tree', '--locked', '--offline', '--manifest-path',
      'src-tauri/Cargo.toml', '--target', TARGET, '--edges', CARGO_EDGES, '--prefix', 'none', '--format', '{p}']));
    metadataBytes = Buffer.from(runCargo(root, ['metadata', '--locked', '--offline', '--manifest-path',
      'src-tauri/Cargo.toml', '--filter-platform', TARGET, '--format-version', '1']));
  }
  const inputs = { schemaVersion: 1, target: TARGET, cargoEdges: CARGO_EDGES, ...before,
    cargoTreeSha256: sha256(treeBytes), cargoMetadataSha256: sha256(metadataBytes) };
  if (JSON.stringify(before) !== JSON.stringify(fileHashes(captureFiles(root)))) {
    throw new Error('locks, Cargo manifest or frontend inventory changed while resolving review inputs');
  }
  if (inputSnapshot) {
    const snapshot = readJson(inputSnapshot, 'input snapshot');
    for (const [key, value] of Object.entries(inputs)) {
      if (snapshot[key] !== value) throw new Error(`input snapshot is stale or invalid for ${key}`);
    }
  }
  let metadata;
  try { metadata = JSON.parse(metadataBytes.toString('utf8')); }
  catch { throw new Error('Cargo metadata is invalid JSON'); }
  if (metadata.version !== 1 || !Array.isArray(metadata.packages)) throw new Error('unsupported Cargo metadata schema');
  const tree = treeBytes.toString('utf8');
  const candidates = cargoCandidates(root, tree, metadata);
  // Parse precisely the bytes whose digest is in inputs. A concurrent Vite
  // rewrite cannot substitute fresh entries below a previously captured hash.
  let frontend;
  try { frontend = JSON.parse(captured.frontendInputsSha256.toString('utf8')); }
  catch { throw new Error('frontend inventory is invalid JSON'); }
  return { inputs, candidates: [...candidates, ...frontendCandidates(root, frontend)] };
}

function cargoCandidates(root, tree, metadata) {
  const local = metadata.packages.find((item) => item.name === 'what-changed' && item.manifest_path
    && realpathSync(item.manifest_path) === realpathSync(resolve(root, 'src-tauri/Cargo.toml')));
  const lines = tree.split(/\r?\n/).filter((line) => line.trim());
  if (!local || !lines[0]?.startsWith(`what-changed v${local.version} `)
    && lines[0] !== `what-changed v${local?.version}`) throw new Error('Cargo tree does not start with this application version');
  const keys = new Map();
  for (const line of lines.slice(1)) {
    const match = /^([^\s]+) v([^\s]+)(?:\s.*)?$/.exec(line.trim());
    if (!match) throw new Error('Cargo tree must use --prefix none --format {p}');
    const [, name, version] = match;
    validateIdentity('cargo', name, version);
    if (name === 'what-changed') throw new Error('unexpected application package in runtime dependencies');
    const matches = metadata.packages.filter((item) => item.name === name && item.version === version);
    if (matches.length !== 1) throw new Error(`Cargo metadata identity is missing or ambiguous: ${name}@${version}`);
    const item = { ecosystem: 'cargo', name, version, metadata: matches[0] };
    keys.set(packageKey(item), item);
  }
  if (!keys.size) throw new Error('the Windows runtime Cargo tree is empty');
  return [...keys.values()].sort((a, b) => compare(packageKey(a), packageKey(b)));
}

function frontendCandidates(root, inventory) {
  if (inventory.schemaVersion !== 2 || !Array.isArray(inventory.packages) || !inventory.packages.length
    || !Array.isArray(inventory.cssPackages) || inventory.unresolvedNodeModules !== 0 || inventory.unresolvedCssImports !== 0) {
    throw new Error('frontend inventory must be schema 2 with resolved JavaScript and CSS package inputs');
  }
  const keys = new Map();
  for (const item of inventory.packages) {
    validateIdentity('frontend', item?.name, item?.version);
    const candidate = { ecosystem: 'frontend', name: item.name, version: item.version };
    if (keys.has(packageKey(candidate))) throw new Error(`duplicate frontend package: ${packageKey(candidate)}`);
    keys.set(packageKey(candidate), candidate);
  }
  for (const item of inventory.cssPackages) {
    validateIdentity('frontend', item?.name, item?.version);
    if (!keys.has(packageKey({ ecosystem: 'frontend', ...item }))) throw new Error('CSS package is missing from frontend package inputs');
  }
  const appCss = resolve(root, 'src/styles/app.css');
  if (existsSync(appCss) && /@import\s+['"]tailwindcss['"]/.test(readFileSync(appCss, 'utf8'))
    && !inventory.cssPackages.some((item) => item.name === 'tailwindcss')) {
    throw new Error('Tailwind CSS import is missing from frontend CSS inputs');
  }
  return [...keys.values()].sort((a, b) => compare(packageKey(a), packageKey(b)));
}
