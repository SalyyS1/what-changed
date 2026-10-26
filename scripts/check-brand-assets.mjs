import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const root = path.resolve('assets/brand');
const required = ['icon-source.svg', 'logo-mark.svg', 'logo-mark-monochrome.svg', 'logo-wordmark.svg', 'logo-lockup-horizontal.svg', 'logo-lockup-stacked.svg', 'tray-light.svg', 'tray-dark.svg', 'tray-high-contrast.svg', 'social-preview-template.svg', 'installer-header.svg', 'installer-sidebar.svg', 'article-cover-template.svg', 'brand-guidelines.md'];
for (const name of required) {
  const file = path.join(root, name);
  if (!fs.existsSync(file)) throw new Error(`missing brand source: ${name}`);
  if (name.endsWith('.svg') && !fs.readFileSync(file, 'utf8').includes('<svg')) throw new Error(`not an SVG: ${name}`);
}
const source = fs.readFileSync(path.join(root, 'icon-source.svg'), 'utf8');
if (!source.includes('#0B0E14') || !source.includes('#4FA3FF')) throw new Error('canonical mark lost approved flat tokens');

const manifestPath = path.join(root, 'generated-assets-manifest.json');
if (!fs.existsSync(manifestPath)) throw new Error('missing generated-assets-manifest.json; run bun run export:brand -UpdateTauriIcons');
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8').replace(/^\uFEFF/, ''));
const sha256 = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
if (manifest.source !== 'assets/brand/icon-source.svg') throw new Error('brand manifest source is not canonical');
const repoRoot = path.resolve(root, '..', '..');
const sourceHash = sha256(path.join(root, 'icon-source.svg'));
if (manifest.source_sha256 !== sourceHash) throw new Error('brand manifest source hash is stale');
for (const relative of [
  'assets/brand/installer-header.bmp',
  'assets/brand/installer-sidebar.bmp',
  'assets/designs/delta-bridge-approval-sheet.svg',
  'assets/launch/social/social-preview.svg',
  'src-tauri/icons/tray-light.png',
  'src-tauri/icons/tray-dark.png',
  'src-tauri/icons/tray-high-contrast.png',
]) {
  if (!manifest.files?.[relative]) throw new Error(`brand manifest lacks ${relative}`);
}
for (const [relative, expected] of Object.entries(manifest.files ?? {})) {
  const file = path.resolve(repoRoot, relative);
  if (!fs.existsSync(file) || sha256(file) !== expected) throw new Error(`stale generated brand asset: ${relative}`);
}
if (manifest.files['assets/launch/social/social-preview.svg'] !== manifest.files['assets/brand/social-preview-template.svg']) {
  throw new Error('launch social preview drifted from the canonical brand template');
}

function pngInfo(file) {
  const bytes = fs.readFileSync(file);
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (bytes.length < 33 || !bytes.subarray(0, 8).equals(signature) || bytes.toString('ascii', 12, 16) !== 'IHDR') {
    throw new Error(`invalid PNG: ${file}`);
  }
  const width = bytes.readUInt32BE(16);
  const height = bytes.readUInt32BE(20);
  const colorType = bytes[25];
  let hasAlpha = colorType === 4 || colorType === 6;
  for (let offset = 8; offset + 12 <= bytes.length;) {
    const length = bytes.readUInt32BE(offset);
    const next = offset + 12 + length;
    if (next > bytes.length) throw new Error(`truncated PNG chunk: ${file}`);
    const chunk = bytes.toString('ascii', offset + 4, offset + 8);
    if (chunk === 'tRNS') hasAlpha = true;
    if (chunk === 'IEND') break;
    offset = next;
  }
  return { width, height, hasAlpha };
}

function expectedSize(relative) {
  const name = path.basename(relative);
  if (name === 'icon.png') return 512;
  if (name === 'StoreLogo.png') return 50;
  if (/^tray-(light|dark|high-contrast)\.png$/.test(name)) return 32;
  const tile = /^Square(\d+)x\1Logo\.png$/.exec(name);
  if (tile) return Number(tile[1]);
  const icon = /^(\d+)x(\d+)(@2x)?\.png$/.exec(name);
  if (icon && icon[1] === icon[2]) return Number(icon[1]) * (icon[3] ? 2 : 1);
  return undefined;
}

for (const relative of Object.keys(manifest.files ?? {}).filter((file) => file.endsWith('.png'))) {
  const size = expectedSize(relative);
  if (!size) throw new Error(`brand PNG has no dimension rule: ${relative}`);
  const { width, height, hasAlpha } = pngInfo(path.resolve(repoRoot, relative));
  if (width !== size || height !== size) throw new Error(`wrong brand PNG dimensions: ${relative} (${width}x${height}, expected ${size}x${size})`);
  if (!hasAlpha && !relative.endsWith('/tray-high-contrast.png')) throw new Error(`brand PNG lost alpha support: ${relative}`);
}

const icoPath = path.resolve(repoRoot, 'src-tauri/icons/icon.ico');
const ico = fs.readFileSync(icoPath);
if (ico.length < 6 || ico.readUInt16LE(0) !== 0 || ico.readUInt16LE(2) !== 1) throw new Error('invalid brand ICO header');
const frames = ico.readUInt16LE(4);
const icoSizes = new Set();
for (let index = 0; index < frames; index += 1) {
  const offset = 6 + index * 16;
  if (offset + 16 > ico.length) throw new Error('truncated brand ICO directory');
  const width = ico[offset] || 256;
  const height = ico[offset + 1] || 256;
  const bytes = ico.readUInt32LE(offset + 8);
  const imageOffset = ico.readUInt32LE(offset + 12);
  if (width !== height || imageOffset + bytes > ico.length) throw new Error(`invalid brand ICO frame ${index}`);
  icoSizes.add(width);
}
for (const size of [16, 20, 24, 32, 48, 64, 128, 256]) {
  if (!icoSizes.has(size)) throw new Error(`brand ICO lacks ${size}px frame`);
}

for (const [name, width, height] of [
  ['installer-header.bmp', 150, 57],
  ['installer-sidebar.bmp', 164, 314],
]) {
  const bytes = fs.readFileSync(path.join(root, name));
  if (bytes.length < 54 || bytes.toString('ascii', 0, 2) !== 'BM' || bytes.readUInt32LE(14) < 40) {
    throw new Error(`invalid installer BMP: ${name}`);
  }
  if (bytes.readInt32LE(18) !== width || Math.abs(bytes.readInt32LE(22)) !== height || bytes.readUInt16LE(28) !== 24) {
    throw new Error(`wrong installer BMP format or dimensions: ${name}`);
  }
}

console.log(`brand assets pass (${required.length} canonical sources; ${Object.keys(manifest.files ?? {}).length} generated hashes)`);
