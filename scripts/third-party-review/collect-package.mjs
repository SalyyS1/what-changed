import { existsSync, readFileSync, readdirSync, realpathSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { compare, isInside, packageKey, readJson, sha256, sourceFile } from './filesystem.mjs';
import { collectNativeComponents } from './native-components.mjs';

const LICENSE_FILE = /^(license|licence|copying|copyright|notice)([._-]|$)/i;

function packageDirectory(root, candidate, cargoSourceRoots) {
  if (candidate.ecosystem === 'frontend') {
    const nodeRoot = realpathSync(resolve(root, 'node_modules'));
    if (!isInside(realpathSync(root), nodeRoot)) throw new Error('node_modules symlink escapes the workspace');
    const directory = realpathSync(resolve(nodeRoot, candidate.name));
    if (!isInside(nodeRoot, directory)) throw new Error(`frontend package symlink escapes node_modules: ${candidate.name}`);
    return directory;
  }
  const manifest = candidate.metadata.manifest_path;
  if (typeof manifest !== 'string') throw new Error(`Cargo manifest path is missing: ${packageKey(candidate)}`);
  const directory = realpathSync(dirname(manifest));
  if (!cargoSourceRoots.some((allowed) => existsSync(allowed) && isInside(realpathSync(allowed), directory))) {
    throw new Error(`Cargo package source is outside allowed source roots: ${packageKey(candidate)}`);
  }
  sourceFile(directory, 'Cargo.toml');
  return directory;
}

export function collectPackage(root, candidate, cargoSourceRoots = [resolve(homedir(), '.cargo/registry/src'), resolve(root, 'src-tauri/vendor')]) {
  const directory = packageDirectory(root, candidate, cargoSourceRoots);
  const metadata = candidate.ecosystem === 'cargo' ? candidate.metadata
    : readJson(sourceFile(directory, 'package.json'), `installed package ${candidate.name}`);
  if (metadata.name !== candidate.name || metadata.version !== candidate.version) {
    throw new Error(`installed source differs from exact input version: ${packageKey(candidate)}`);
  }
  const prefix = `legal/third-party/${candidate.ecosystem}/${candidate.name}/${candidate.version}`;
  const copies = [];
  const licenseFiles = [];
  const noticeFiles = [];
  const declarationFiles = [];
  const attributionFiles = [];
  const issues = [];
  for (const file of readdirSync(directory).filter((name) => LICENSE_FILE.test(name)).sort(compare)) {
    // SPDX documents describe declarations; they are not original full license text.
    const bytes = readFileSync(sourceFile(directory, file));
    const record = { file: `${prefix}/${file}`, sha256: sha256(bytes), bytes: bytes.length };
    copies.push({ ...record, content: bytes });
    if (/\.spdx(?:\.json)?$/i.test(file)) declarationFiles.push(record);
    else if (/^notice([._-]|$)/i.test(file)) noticeFiles.push(record);
    else if (/^copyright([._-]|$)/i.test(file)) attributionFiles.push(record);
    else licenseFiles.push(record);
    if (!bytes.length) issues.push(`empty-source-file:${file}`);
  }
  const expression = typeof metadata.license === 'string' && metadata.license.trim() ? metadata.license : null;
  if (!expression) issues.push('missing-license-expression');
  if (!licenseFiles.some((file) => file.bytes > 0)) issues.push('missing-full-license-text');
  const sourceMetadata = [];
  for (const file of candidate.ecosystem === 'cargo' ? ['Cargo.toml', 'Cargo.toml.orig', '.cargo_vcs_info.json'] : ['package.json']) {
    if (existsSync(resolve(directory, file))) sourceMetadata.push({ file, sha256: sha256(readFileSync(sourceFile(directory, file))) });
  }
  const nativeComponents = collectNativeComponents(directory, candidate);
  if (nativeComponents.length) issues.push('native-component-license-review-required');
  return { copies, entry: {
    ecosystem: candidate.ecosystem, name: candidate.name, version: candidate.version,
    licenseExpression: expression, declaredLicenseMetadata: metadata.license ?? metadata.licenses ?? null,
    selectedLicense: null, obligationsReviewed: false, sourceUrl: null, sourceAvailabilityUrl: null,
    sourceMetadata: { repository: metadata.repository ?? null, homepage: metadata.homepage ?? null,
      cargoSource: candidate.ecosystem === 'cargo' ? metadata.source ?? null : null, manifests: sourceMetadata },
    licenseFiles, noticeFiles, declarationFiles, attributionFiles, nativeComponents,
    issues: issues.sort(compare),
  } };
}
