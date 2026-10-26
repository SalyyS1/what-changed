import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync, renameSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { noticeSection, reviewedLicense } from './third-party-review/reviewed-license-notices.mjs';
import { reviewedNativeSections } from './third-party-review/reviewed-native-notices.mjs';

const DEFAULT_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const TARGET = 'x86_64-pc-windows-msvc';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function readJson(path, label) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    throw new Error(`${label} is missing or invalid: ${path} (${error.message})`);
  }
}

function packageKey(ecosystem, name, version) {
  return `${ecosystem}:${name}@${version}`;
}

function runtimeCargoPackages(tree) {
  const packages = new Set();
  for (const line of tree.split(/\r?\n/)) {
    const match = /^([^\s]+) v([^\s]+)/.exec(line.trim());
    if (match && match[1] !== 'what-changed') {
      packages.add(packageKey('cargo', match[1], match[2]));
    }
  }
  if (!packages.size) throw new Error('the release-target Cargo tree is empty');
  return packages;
}

function frontendPackages(input, root) {
  if (input.schemaVersion !== 2 || !Array.isArray(input.packages) || !input.packages.length
    || !Array.isArray(input.cssPackages)) {
    throw new Error('the final Vite module-input inventory is missing or empty');
  }
  if (input.unresolvedNodeModules !== 0) {
    throw new Error(`the Vite module-input inventory has ${input.unresolvedNodeModules ?? 'unknown'} unresolved node_modules inputs`);
  }
  if (input.unresolvedCssImports !== 0) {
    throw new Error(`the Vite module-input inventory has ${input.unresolvedCssImports ?? 'unknown'} unresolved CSS package imports`);
  }
  const packages = new Set();
  for (const item of input.packages) {
    if (!item || typeof item.name !== 'string' || typeof item.version !== 'string') {
      throw new Error('the final Vite module-input inventory has an invalid package');
    }
    packages.add(packageKey('frontend', item.name, item.version));
  }
  for (const item of input.cssPackages) {
    if (!item || typeof item.name !== 'string' || typeof item.version !== 'string'
      || !packages.has(packageKey('frontend', item.name, item.version))) {
      throw new Error('the Vite CSS package-input inventory is invalid or not covered');
    }
  }
  const cssEntry = resolve(root, 'src/styles/app.css');
  if (existsSync(cssEntry) && /@import\s+['"]tailwindcss['"]/.test(readFileSync(cssEntry, 'utf8'))
    && !input.cssPackages.some((item) => item.name === 'tailwindcss')) {
    throw new Error('Tailwind CSS is emitted but missing from the Vite CSS package-input inventory');
  }
  return packages;
}

function cargoTreeText(root, override) {
  if (override) return readFileSync(override, 'utf8');
  try {
    return execFileSync('cargo', [
      'tree', '--locked', '--offline', '--manifest-path', 'src-tauri/Cargo.toml',
      '--target', TARGET, '--edges', 'normal,no-proc-macro',
      '--prefix', 'none', '--format', '{p}',
    ], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
  } catch (error) {
    throw new Error(`cannot resolve the Windows runtime dependency candidates: ${error.message}`);
  }
}

function noticePayload(root, manifest, candidates, cargoMetadata) {
  if (!Array.isArray(manifest.entries)) throw new Error('notice entries must be an array');
  const approved = new Map();
  for (const entry of manifest.entries) {
    if (!entry || !['cargo', 'frontend'].includes(entry.ecosystem)
      || typeof entry.name !== 'string' || typeof entry.version !== 'string') {
      throw new Error('notice entry has invalid package identity');
    }
    const key = packageKey(entry.ecosystem, entry.name, entry.version);
    if (approved.has(key)) throw new Error(`duplicate notice entry: ${key}`);
    approved.set(key, reviewedLicense(root, entry, key));
  }
  const missing = [...candidates].filter((key) => !approved.has(key));
  const extra = [...approved.keys()].filter((key) => !candidates.has(key));
  if (missing.length || extra.length) {
    throw new Error(`notice coverage differs from release inputs: missing=${missing.slice(0, 12).join(', ') || 'none'}${missing.length > 12 ? ` (+${missing.length - 12} more)` : ''}; extra=${extra.slice(0, 12).join(', ') || 'none'}${extra.length > 12 ? ` (+${extra.length - 12} more)` : ''}`);
  }
  const nativeSections = reviewedNativeSections({ root, manifest, cargoMetadata });
  const sections = [...approved.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([key, value]) => noticeSection(key, value));
  const nativeScope = nativeSections ? 'Native sections cover reviewed installed source candidates; frozen executable/installer binding is still required.\n' : '';
  return `What Changed — Third-Party Notices\nTarget: ${TARGET}\nCargo.lock SHA-256: ${manifest.cargoLockSha256}\nbun.lock SHA-256: ${manifest.bunLockSha256}\n\nThis file covers the reviewed Windows release-target Rust dependency closure and frontend bundler module inputs. Some conservatively covered components may be removed by linking or tree shaking.\n${nativeScope}${sections.join('')}${nativeSections}`;
}

export function generateThirdPartyNotices({
  root = DEFAULT_ROOT,
  mode = 'check',
  cargoTree,
  cargoMetadata,
} = {}) {
  const manifest = readJson(resolve(root, 'scripts/third-party-notices-manifest.json'), 'reviewed notice manifest');
  if (manifest.schemaVersion !== 1 || manifest.target !== TARGET) {
    throw new Error('notice manifest has an unsupported schema or target');
  }
  if (manifest.status !== 'reviewed' || !manifest.reviewedBy || !manifest.reviewedAt) {
    throw new Error('third-party notices are pending exact-version legal/text review; release is blocked');
  }
  for (const [file, expected] of [
    ['src-tauri/Cargo.lock', manifest.cargoLockSha256],
    ['bun.lock', manifest.bunLockSha256],
  ]) {
    if (!/^[a-f0-9]{64}$/.test(expected ?? '') || sha256(readFileSync(resolve(root, file))) !== expected) {
      throw new Error(`notice review is stale for ${file}`);
    }
  }
  const candidates = runtimeCargoPackages(cargoTreeText(root, cargoTree));
  const frontend = readJson(resolve(root, 'dist-installer/frontend-inputs.json'), 'final Vite module inputs');
  for (const key of frontendPackages(frontend, root)) candidates.add(key);
  const payload = noticePayload(root, manifest, candidates, cargoMetadata);
  const output = resolve(root, 'dist-installer/THIRD_PARTY_NOTICES.txt');
  if (mode === 'prepare') {
    mkdirSync(dirname(output), { recursive: true });
    const temporary = `${output}.${process.pid}.tmp`;
    writeFileSync(temporary, payload, { encoding: 'utf8', flag: 'wx' });
    renameSync(temporary, output);
  } else if (mode === 'check') {
    if (!existsSync(output) || readFileSync(output, 'utf8') !== payload) {
      throw new Error('bundled third-party notice payload is missing or stale');
    }
  } else {
    throw new Error(`unknown notice mode: ${mode}`);
  }
  return { output, packages: candidates.size, sha256: sha256(Buffer.from(payload)) };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const mode = process.argv[2] === '--prepare' ? 'prepare' : process.argv[2] === '--check' ? 'check' : null;
    if (!mode || process.argv.length !== 3) throw new Error('usage: node scripts/generate-third-party-notices.mjs --prepare|--check');
    const result = generateThirdPartyNotices({ mode });
    process.stdout.write(`third-party notices: ${result.packages} reviewed inputs; sha256=${result.sha256}\n`);
  } catch (error) {
    process.stderr.write(`third-party notices blocked: ${error.message}\n`);
    process.exitCode = 1;
  }
}
