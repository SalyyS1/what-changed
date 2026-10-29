import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { generateThirdPartyNotices } from './generate-third-party-notices.mjs';

const digest = (value) => createHash('sha256').update(value).digest('hex');

function fixture(t, status = 'reviewed') {
  const root = mkdtempSync(join(tmpdir(), 'what-changed-notice-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const directory of ['scripts', 'src-tauri', 'dist-installer', 'legal/third-party']) {
    mkdirSync(join(root, directory), { recursive: true });
  }
  const cargoLock = 'locked cargo fixture\n';
  const bunLock = 'locked bun fixture\n';
  writeFileSync(join(root, 'src-tauri/Cargo.lock'), cargoLock);
  writeFileSync(join(root, 'bun.lock'), bunLock);
  const cargoTree = join(root, 'cargo-tree.txt');
  writeFileSync(cargoTree, 'what-changed v0.1.0 (local)\ncrate-one v1.2.3\n');
  writeFileSync(join(root, 'dist-installer/frontend-inputs.json'), JSON.stringify({
    schemaVersion: 2,
    packages: [{ name: '@example/browser', version: '4.5.6' }],
    cssPackages: [],
    unresolvedNodeModules: 0,
    unresolvedCssImports: 0,
  }));
  const rustText = 'Copyright 2026 Rust fixture\nMIT License\n';
  const browserText = 'Copyright 2026 Browser fixture\nApache License\n';
  writeFileSync(join(root, 'legal/third-party/rust.txt'), rustText);
  writeFileSync(join(root, 'legal/third-party/browser.txt'), browserText);
  const manifest = {
    schemaVersion: 1,
    status,
    target: 'x86_64-pc-windows-msvc',
    cargoLockSha256: digest(cargoLock),
    bunLockSha256: digest(bunLock),
    reviewedBy: status === 'reviewed' ? 'fixture reviewer' : null,
    reviewedAt: status === 'reviewed' ? '2026-09-29' : null,
    entries: [
      {
        ecosystem: 'cargo', name: 'crate-one', version: '1.2.3',
        licenseExpression: 'MIT', selectedLicense: 'MIT', obligationsReviewed: true,
        noticeFiles: [], sourceUrl: 'https://example.test/crate-one/1.2.3',
        textFile: 'legal/third-party/rust.txt', textSha256: digest(rustText),
      },
      {
        ecosystem: 'frontend', name: '@example/browser', version: '4.5.6',
        licenseExpression: 'Apache-2.0', selectedLicense: 'Apache-2.0', obligationsReviewed: true,
        noticeFiles: [], sourceUrl: 'https://example.test/browser/4.5.6',
        textFile: 'legal/third-party/browser.txt', textSha256: digest(browserText),
      },
    ],
  };
  const save = () => writeFileSync(join(root, 'scripts/third-party-notices-manifest.json'), JSON.stringify(manifest));
  save();
  return { root, cargoTree, manifest, save };
}

test('pending review refuses to create a notice payload', (t) => {
  const item = fixture(t, 'pending');
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /pending exact-version legal\/text review/,
  );
});

test('reviewed exact-version inputs produce and verify deterministic notices', (t) => {
  const item = fixture(t);
  const prepared = generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' });
  assert.equal(prepared.packages, 2);
  const payload = readFileSync(prepared.output, 'utf8');
  assert.match(payload, /cargo:crate-one@1\.2\.3/);
  assert.match(payload, /frontend:@example\/browser@4\.5\.6/);
  assert.match(payload, /Copyright 2026 Rust fixture/);
  assert.equal(generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'check' }).sha256, prepared.sha256);
  assert.equal(generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }).sha256, prepared.sha256);
});

test('missing package text or coverage fails closed', (t) => {
  const item = fixture(t);
  item.manifest.entries.pop();
  item.save();
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /missing=frontend:@example\/browser@4\.5\.6/,
  );
  item.manifest.entries[0].textSha256 = '0'.repeat(64);
  item.save();
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /license text hash does not match/,
  );
});

test('stale lockfile and unresolved bundler inputs fail closed', (t) => {
  const item = fixture(t);
  writeFileSync(join(item.root, 'bun.lock'), 'changed lock\n');
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /review is stale for bun.lock/,
  );
  writeFileSync(join(item.root, 'bun.lock'), 'locked bun fixture\n');
  writeFileSync(join(item.root, 'dist-installer/frontend-inputs.json'), JSON.stringify({
    schemaVersion: 2, packages: [{ name: '@example/browser', version: '4.5.6' }], cssPackages: [], unresolvedNodeModules: 1, unresolvedCssImports: 0,
  }));
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /unresolved node_modules inputs/,
  );
  writeFileSync(join(item.root, 'dist-installer/frontend-inputs.json'), JSON.stringify({
    schemaVersion: 2, packages: [{ name: '@example/browser', version: '4.5.6' }], cssPackages: [], unresolvedNodeModules: 0, unresolvedCssImports: 1,
  }));
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /unresolved CSS package imports/,
  );
});

test('an emitted Tailwind import cannot disappear from CSS package inputs', (t) => {
  const item = fixture(t);
  mkdirSync(join(item.root, 'src/styles'), { recursive: true });
  writeFileSync(join(item.root, 'src/styles/app.css'), "@import 'tailwindcss';\n");
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /Tailwind CSS is emitted but missing/,
  );
});

test('notice paths cannot escape the reviewed text directory', (t) => {
  const item = fixture(t);
  item.manifest.entries[0].textFile = 'src-tauri/Cargo.lock';
  item.save();
  assert.throws(
    () => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }),
    /outside legal\/third-party/,
  );
});

for (const ecosystem of ['cargo', 'frontend']) {
  for (const field of ['sourceUrl', 'sourceAvailabilityUrl']) {
    for (const invalid of ['https://', 'http://example.test/source', { url: 'https://example.test/source' }]) {
      test(`${ecosystem} ${field} rejects unusable HTTPS URL ${JSON.stringify(invalid)} without replacing payload`, (t) => {
        const item = fixture(t);
        const output = join(item.root, 'dist-installer/THIRD_PARTY_NOTICES.txt');
        const sentinel = 'previous reviewed notice payload\n';
        writeFileSync(output, sentinel);
        item.manifest.entries.find((entry) => entry.ecosystem === ecosystem)[field] = invalid;
        item.save();
        const message = field === 'sourceUrl' ? /authoritative HTTPS source URL/ : /authoritative HTTPS source-availability URL/;
        assert.throws(() => generateThirdPartyNotices({ root: item.root, cargoTree: item.cargoTree, mode: 'prepare' }), message);
        assert.equal(readFileSync(output, 'utf8'), sentinel);
      });
    }
  }
}
