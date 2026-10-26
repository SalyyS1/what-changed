import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { generateThirdPartyNotices } from '../generate-third-party-notices.mjs';
import { collectPackage } from './collect-package.mjs';
import { TARGET, sha256 } from './filesystem.mjs';

export const SENTINEL = 'existing reviewed payload must survive failed validation\n';

export function reviewedNativeFixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'what-changed-reviewed-native-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (file, bytes) => {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), bytes);
  };
  write('src-tauri/Cargo.lock', 'locked cargo fixture\n');
  write('src-tauri/Cargo.toml', '[package]\nname="what-changed"\n');
  write('bun.lock', 'locked bun fixture\n');
  write('cargo-tree.txt', 'what-changed v0.1.0\nlibsqlite3-sys v0.36.0\nwebview2-com-sys v0.38.2\n');
  write('dist-installer/frontend-inputs.json', JSON.stringify({ schemaVersion: 2,
    packages: [{ name: 'browser', version: '1.0.0' }], cssPackages: [], unresolvedNodeModules: 0, unresolvedCssImports: 0 }));
  write('dist-installer/THIRD_PARTY_NOTICES.txt', SENTINEL);
  const metadata = { version: 1, packages: [
    { name: 'libsqlite3-sys', version: '0.36.0' }, { name: 'webview2-com-sys', version: '0.38.2' },
  ].map((item) => ({ ...item, license: 'MIT', manifest_path: join(root, `src-tauri/vendor/${item.name}/Cargo.toml`) })) };
  for (const item of metadata.packages) {
    write(`src-tauri/vendor/${item.name}/Cargo.toml`, `[package]\nname="${item.name}"\nversion="${item.version}"\n`);
    write(`src-tauri/vendor/${item.name}/LICENSE`, 'Original fixture crate MIT license\n');
  }
  write('src-tauri/vendor/libsqlite3-sys/sqlite3/sqlite3.c',
    '#define SQLITE_VERSION "3.53.2"\n#define SQLITE_SOURCE_ID "fixture-source-id"\n');
  for (const [file, bytes] of Object.entries({
    'x64/WebView2Loader.dll': Buffer.from([0x4d, 0x5a, 0x00, 0xff]),
    'x64/WebView2Loader.dll.lib': Buffer.from([0x21, 0x3c, 0x61, 0x72]),
    'x64/WebView2LoaderStatic.lib': Buffer.from([0x21, 0x3c, 0x61, 0x72, 0xff]),
    'build.rs': 'fn main() {}\r\n', 'src/lib.rs': '// fixture selection source\n',
  })) write(`src-tauri/vendor/webview2-com-sys/${file}`, bytes);
  const license = (name, selectedLicense = 'MIT') => {
    const text = `${name} separately reviewed fixture license text\n`;
    const textFile = `legal/third-party/${name}.txt`;
    write(textFile, text);
    return { licenseExpression: selectedLicense, selectedLicense, obligationsReviewed: true, noticeFiles: [],
      sourceUrl: `https://example.test/${name}`, textFile, textSha256: sha256(Buffer.from(text)) };
  };
  const entries = metadata.packages.map((item) => ({ ecosystem: 'cargo', name: item.name, version: item.version,
    ...license(item.name), nativeComponents: collectPackage(root, { ecosystem: 'cargo', ...item, metadata: item }).entry.nativeComponents
      .map((component) => ({ ...component, ...license(component.name, `LicenseRef-${component.name}`),
        ...(component.version === null ? { provenanceReviewed: true, provenanceUrl: 'https://example.test/authoritative-sdk' } : {}) })) }));
  entries.push({ ecosystem: 'frontend', name: 'browser', version: '1.0.0', ...license('browser') });
  const manifest = { schemaVersion: 1, target: TARGET, status: 'reviewed', reviewedBy: 'unit fixture reviewer',
    reviewedAt: '2026-10-04', entries,
    cargoLockSha256: sha256(readFileSync(join(root, 'src-tauri/Cargo.lock'))),
    bunLockSha256: sha256(readFileSync(join(root, 'bun.lock'))),
    cargoManifestSha256: sha256(readFileSync(join(root, 'src-tauri/Cargo.toml'))) };
  const save = () => {
    write('scripts/third-party-notices-manifest.json', JSON.stringify(manifest));
    write('cargo-metadata.json', JSON.stringify(metadata));
  };
  save();
  const options = { root, cargoTree: join(root, 'cargo-tree.txt'), cargoMetadata: join(root, 'cargo-metadata.json'), mode: 'prepare' };
  return { root, write, manifest, metadata, save, options, license,
    sqlite: entries[0].nativeComponents[0], webview: entries[1].nativeComponents[0],
    run: () => generateThirdPartyNotices(options) };
}
