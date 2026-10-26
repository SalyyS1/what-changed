import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { prepareThirdPartyReview } from './prepare-third-party-review.mjs';
import { sha256 } from './third-party-review/filesystem.mjs';
import { captureReviewInputs } from './third-party-review/inputs.mjs';
import { persistReviewDraft } from './third-party-review/persist-draft.mjs';
import { fixture } from './third-party-review/test-fixture.mjs';

test('draft is deterministic, exact-input pending evidence and preserves original byte sequences', (t) => {
  const item = fixture(t);
  const reviewedBefore = readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json'));
  const result = item.run();
  const draftBytes = readFileSync(result.output);
  const draft = JSON.parse(draftBytes);
  assert.equal(result.packages, 5);
  assert.equal(draft.status, 'pending');
  assert.equal(draft.reviewedBy, null);
  assert.equal(draft.publishable, false);
  assert.equal(draft.obligationsReviewed, false);
  assert.ok(draft.entries.every((entry) => !entry.obligationsReviewed && entry.selectedLicense === null));
  assert.ok(!draft.entries.some((entry) => entry.name === 'proc-only'));
  const browser = draft.entries.find((entry) => entry.name === '@example/browser');
  assert.equal(browser.licenseFiles.length, 2);
  for (const file of browser.licenseFiles) {
    const original = readFileSync(join(item.root, 'node_modules/@example/browser', file.file.split('/').at(-1)));
    assert.deepEqual(readFileSync(join(item.root, file.file)), original);
    assert.equal(file.sha256, sha256(original));
  }
  assert.equal(draft.entries.find((entry) => entry.name === 'crate-one').noticeFiles.length, 1);
  const sqlite = draft.entries.find((entry) => entry.name === 'libsqlite3-sys').nativeComponents[0];
  assert.equal(sqlite.version, '3.53.2');
  assert.equal(sqlite.sourceId, 'fixture-checkin');
  assert.equal(sqlite.sourceSha256, sha256(readFileSync(join(item.root, '.cargo/registry/src/index/libsqlite3-sys-1.2.3/sqlite3/sqlite3.c'))));
  assert.ok(!JSON.stringify(draft).toLowerCase().includes('sqlcipher'));
  assert.deepEqual(readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json')), reviewedBefore);
  assert.equal(existsSync(join(item.root, 'dist-installer/THIRD_PARTY_NOTICES.txt')), false);
  assert.equal(item.run().sha256, result.sha256);
  assert.deepEqual(readFileSync(result.output), draftBytes);
});

test('SPDX declarations are retained but never treated as full license text; absent expressions are flagged', (t) => {
  const item = fixture(t);
  const draft = JSON.parse(readFileSync(item.run().output));
  const declaration = draft.entries.find((entry) => entry.name === 'declaration-only');
  assert.equal(declaration.declarationFiles.length, 1);
  assert.equal(declaration.licenseFiles.length, 0);
  assert.deepEqual(declaration.issues, ['missing-full-license-text']);
  const unexpressed = draft.entries.find((entry) => entry.name === 'unexpressed');
  assert.equal(unexpressed.licenseExpression, null);
  assert.deepEqual(unexpressed.issues, ['missing-license-expression']);
});

test('COPYRIGHT attribution alone never satisfies full-license coverage', (t) => {
  const item = fixture(t);
  rmSync(join(item.root, 'node_modules/unexpressed/LICENSE'));
  const text = 'Copyright 2026 fixture authors\r\nAll rights reserved.\r\n';
  item.write('node_modules/unexpressed/COPYRIGHT', text);
  const draft = JSON.parse(readFileSync(item.run().output));
  const entry = draft.entries.find((candidate) => candidate.name === 'unexpressed');
  assert.equal(entry.licenseFiles.length, 0);
  assert.equal(entry.attributionFiles.length, 1);
  assert.equal(entry.attributionFiles[0].sha256, sha256(Buffer.from(text)));
  assert.equal(readFileSync(join(item.root, entry.attributionFiles[0].file), 'utf8'), text);
  assert.ok(entry.issues.includes('missing-full-license-text'));
});

test('a frontend rewrite after capture is rejected at persistence before any output mutation', (t) => {
  const item = fixture(t);
  const captured = captureReviewInputs(item.options);
  assert.equal(captured.inputs.frontendInputsSha256, sha256(readFileSync(join(item.root, 'dist-installer/frontend-inputs.json'))));
  const originalKeys = captured.candidates.map((candidate) => `${candidate.name}@${candidate.version}`);
  item.inventory.packages[0].version = '9.9.9';
  item.write('dist-installer/frontend-inputs.json', JSON.stringify(item.inventory));
  assert.deepEqual(captured.candidates.map((candidate) => `${candidate.name}@${candidate.version}`), originalKeys);
  assert.throws(() => persistReviewDraft({ root: item.root, draft: { status: 'pending', inputs: captured.inputs, entries: [] },
    copies: [{ file: 'legal/third-party/fixture/LICENSE', content: Buffer.from('fixture') }], inputFiles: item.options }),
  /review inputs changed before output write: frontendInputsSha256/);
  assert.equal(existsSync(join(item.root, 'legal')), false);
});

for (const [key, file] of Object.entries({ cargoLockSha256: 'src-tauri/Cargo.lock', bunLockSha256: 'bun.lock',
  cargoManifestSha256: 'src-tauri/Cargo.toml', frontendInputsSha256: 'dist-installer/frontend-inputs.json',
  cargoTreeSha256: 'cargo-tree.txt', cargoMetadataSha256: 'cargo-metadata.json' })) {
  test(`stale precomputed ${key} is rejected before any output write`, (t) => {
    const item = fixture(t);
    item.write(file, `${readFileSync(join(item.root, file), 'utf8')}\n`);
    assert.throws(item.run, new RegExp(`snapshot is stale or invalid for ${key}`));
    assert.equal(existsSync(join(item.root, 'legal')), false);
  });
}

test('precomputed inputs need a target/edge/hash snapshot and cannot trigger Cargo implicitly', (t) => {
  const item = fixture(t);
  assert.throws(() => prepareThirdPartyReview({ ...item.options, inputSnapshot: undefined }), /require --cargo-tree/);
  const path = item.options.inputSnapshot;
  const snapshot = JSON.parse(readFileSync(path));
  snapshot.cargoEdges = 'normal';
  item.write('input-snapshot.json', JSON.stringify(snapshot));
  assert.throws(item.run, /snapshot is stale or invalid for cargoEdges/);
});

test('installed exact versions, unresolved inputs and ambiguous Cargo identities fail closed', (t) => {
  const item = fixture(t);
  item.write('node_modules/unexpressed/package.json', '{"name":"unexpressed","version":"9.9.9"}');
  assert.throws(item.run, /differs from exact input version/);
  item.inventory.unresolvedCssImports = 1;
  item.saveInputs();
  assert.throws(item.run, /resolved JavaScript and CSS/);
  item.inventory.unresolvedCssImports = 0;
  item.metadata.packages.push({ ...item.metadata.packages[1], source: 'git+other' });
  item.saveInputs();
  assert.throws(item.run, /identity is missing or ambiguous/);
});

test('unsafe package names never become source or destination paths', (t) => {
  const item = fixture(t);
  item.inventory.packages[0].name = '../outside';
  item.saveInputs();
  assert.throws(item.run, /unsafe or invalid frontend package identity/);
  assert.equal(existsSync(join(item.root, 'legal')), false);
});

test('different existing text is protected and no partial draft is created', (t) => {
  const item = fixture(t);
  const destination = 'legal/third-party/frontend/@example/browser/4.5.6/LICENSE_MIT';
  item.write(destination, 'previous reviewed original bytes');
  assert.throws(item.run, /refusing to overwrite different existing third-party text/);
  assert.equal(readFileSync(join(item.root, destination), 'utf8'), 'previous reviewed original bytes');
  assert.equal(existsSync(join(item.root, 'legal/third-party/cargo')), false);
  assert.equal(existsSync(join(item.root, 'legal/third-party/review-draft.json')), false);
});

test('reviewed draft cannot be replaced with pending preparation', (t) => {
  const item = fixture(t);
  item.write('legal/third-party/review-draft.json', '{"status":"reviewed","reviewedBy":"fixture"}');
  assert.throws(item.run, /refusing to replace a reviewed or non-pending draft/);
});

test('frontend and legal-directory symlink escapes are rejected', (t) => {
  const item = fixture(t);
  const outside = mkdtempSync(join(tmpdir(), 'what-changed-review-outside-'));
  t.after(() => rmSync(outside, { recursive: true, force: true }));
  const packagePath = join(item.root, 'node_modules/unexpressed');
  rmSync(packagePath, { recursive: true });
  symlinkSync(outside, packagePath, 'junction');
  assert.throws(item.run, /frontend package symlink escapes/);
  rmSync(packagePath, { recursive: true });
  item.write('node_modules/unexpressed/package.json', '{"name":"unexpressed","version":"1.0.0"}');
  item.write('node_modules/unexpressed/LICENSE', 'MIT\n');
  mkdirSync(join(item.root, 'legal'));
  symlinkSync(outside, join(item.root, 'legal/third-party'), 'junction');
  assert.throws(item.run, /output symlink escapes the workspace/);
  assert.deepEqual(readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json'), 'utf8'), '{"status":"pending","entries":[]}\n');
});
