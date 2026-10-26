import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { generateThirdPartyNotices } from '../generate-third-party-notices.mjs';
import { sha256 } from './filesystem.mjs';
import { SENTINEL, reviewedNativeFixture } from './reviewed-native-notices-test-fixture.mjs';

function assertPreserved(item) {
  assert.equal(readFileSync(join(item.root, 'dist-installer/THIRD_PARTY_NOTICES.txt'), 'utf8'), SENTINEL);
  assert.deepEqual(readdirSync(join(item.root, 'dist-installer')).sort(), ['THIRD_PARTY_NOTICES.txt', 'frontend-inputs.json']);
}

test('reviewed native fixtures emit separate deterministic licenses with inherited reviewer and explicit unresolved provenance', (t) => {
  const item = reviewedNativeFixture(t);
  const before = readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json'));
  const result = item.run();
  const payload = readFileSync(result.output, 'utf8');
  assert.equal(result.packages, 3);
  assert.match(payload, /native:cargo:libsqlite3-sys@0\.36\.0\/SQLite@3\.53\.2/);
  assert.match(payload, /native:cargo:webview2-com-sys@0\.38\.2\/WebView2Loader@version-unresolved/);
  assert.match(payload, /SQLite separately reviewed fixture license text/);
  assert.match(payload, /WebView2Loader separately reviewed fixture license text/);
  assert.match(payload, /Reviewed by: unit fixture reviewer/);
  assert.match(payload, /Reviewed provenance: https:\/\/example\.test\/authoritative-sdk/);
  assert.match(payload, /frozen executable\/installer binding is still required/);
  assert.equal(generateThirdPartyNotices({ ...item.options, mode: 'check' }).sha256, result.sha256);
  assert.equal(item.run().sha256, result.sha256);
  assert.deepEqual(readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json')), before);
});

const failures = [
  ['omitted SQLite', (x) => delete x.manifest.entries[0].nativeComponents, /exactly one reviewed SQLite/],
  ['omitted WebView2', (x) => x.manifest.entries[1].nativeComponents = [], /exactly one reviewed WebView2Loader/],
  ['duplicate native component', (x) => x.manifest.entries[1].nativeComponents.push({ ...x.webview }), /exactly one reviewed WebView2Loader/],
  ['pending obligation review despite MIT wrapper', (x) => x.webview.obligationsReviewed = false, /obligation review/],
  ['pending native status', (x) => x.webview.status = 'pending', /native review is pending/],
  ['missing native license expression', (x) => x.webview.licenseExpression = null, /license expression is missing/],
  ['missing native license choice', (x) => x.webview.selectedLicense = null, /license choice/],
  ['missing native NOTICE decision', (x) => delete x.webview.noticeFiles, /NOTICE file decision/],
  ['missing native license text', (x) => rmSync(join(x.root, x.webview.textFile)), /license text is missing/],
  ['stale native license hash', (x) => x.webview.textSha256 = '0'.repeat(64), /license text hash does not match/],
  ['missing native NOTICE text', (x) => x.webview.noticeFiles = [{ file: 'legal/third-party/absent.txt', sha256: '0'.repeat(64) }], /NOTICE text is missing/],
  ['stale native NOTICE hash', (x) => x.webview.noticeFiles = [{ file: x.webview.textFile, sha256: '0'.repeat(64) }], /NOTICE text hash does not match/],
  ['escaping native NOTICE path', (x) => x.webview.noticeFiles = [{ file: 'src-tauri/Cargo.lock', sha256: '0'.repeat(64) }], /outside legal\/third-party/],
  ['empty native license text', (x) => { x.write(x.webview.textFile, ''); x.webview.textSha256 = sha256(Buffer.alloc(0)); }, /license text is empty/],
  ['escaping native license path', (x) => x.webview.textFile = 'src-tauri/Cargo.lock', /outside legal\/third-party/],
  ['copyleft without source availability', (x) => x.webview.selectedLicense = 'LGPL-2.1-only', /source-availability URL is missing/],
  ['missing HTTPS source', (x) => x.webview.sourceUrl = 'http://example.test/sdk', /authoritative HTTPS source URL/],
  ['invalid HTTPS source', (x) => x.webview.sourceUrl = 'https://', /authoritative HTTPS source URL/],
  ['invalid native source availability', (x) => x.webview.sourceAvailabilityUrl = 'https://', /authoritative HTTPS source-availability URL/],
  ['unresolved version without provenance review', (x) => delete x.webview.provenanceReviewed, /native provenance review is required/],
  ['unresolved version without authoritative provenance', (x) => x.webview.provenanceUrl = 'http://example.test/sdk', /native provenance review is required/],
  ['invented SDK version without provenance', (x) => { x.webview.version = '999.0.0'; x.webview.provenanceReviewed = false; }, /native provenance review is required/],
  ['missing native version decision', (x) => delete x.webview.version, /native version decision is missing/],
  ['wrong SQLite source ID', (x) => x.sqlite.sourceId = 'different', /native source identity differs/],
  ['wrong SQLite version', (x) => x.sqlite.version = '0.0.0', /native source identity differs/],
  ['contradictory SQLite source fields', (x) => { x.sqlite.sourceFiles = [{ ...x.sqlite }]; x.sqlite.sourceFile = 'other.c'; }, /native source identity differs/],
  ['wrong WebView target', (x) => x.webview.target = 'aarch64-pc-windows-msvc', /native source identity differs/],
  ['missing native hash evidence', (x) => delete x.webview.nativeFiles[0].sourceSha256, /native source evidence differs/],
  ['missing selection source evidence', (x) => delete x.webview.selectionEvidence, /native source evidence differs/],
  ['duplicate native source evidence', (x) => x.webview.nativeFiles[1] = x.webview.nativeFiles[0], /native source evidence differs/],
  ['changed installed native bytes', (x) => x.write('src-tauri/vendor/webview2-com-sys/x64/WebView2Loader.dll', Buffer.from([0x4d, 0x5a, 0x01, 0xff])), /native source evidence differs/],
  ['changed installed selection bytes', (x) => x.write('src-tauri/vendor/webview2-com-sys/build.rs', '// changed\n'), /native source evidence differs/],
  ['changed installed SQLite bytes', (x) => x.write('src-tauri/vendor/libsqlite3-sys/sqlite3/sqlite3.c', '#define SQLITE_VERSION "3.53.2"\n#define SQLITE_SOURCE_ID "fixture-source-id"\n// changed\n'), /native source identity differs/],
  ['missing Cargo manifest binding', (x) => delete x.manifest.cargoManifestSha256, /stale for src-tauri\/Cargo.toml/],
  ['stale Cargo manifest binding', (x) => x.write('src-tauri/Cargo.toml', '[package]\nname="changed"\n'), /stale for src-tauri\/Cargo.toml/],
  ['ambiguous native Cargo metadata', (x) => x.metadata.packages.push({ ...x.metadata.packages[0] }), /identity is missing or ambiguous/],
  ['missing native Cargo metadata identity', (x) => x.metadata.packages[0].version = '9.9.9', /identity is missing or ambiguous/],
  ['invalid additional component', (x) => x.manifest.entries[1].nativeComponents.push({ name: 'Additional', version: '1.0.0' }), /license expression is missing/],
  ['additional component without source evidence', (x) => x.manifest.entries[1].nativeComponents.push({ name: 'Additional', version: '1.0.0', ...x.license('Additional') }), /native source evidence is missing/],
  ['escaping additional source evidence', (x) => x.manifest.entries[1].nativeComponents.push({ name: 'Additional', version: '1.0.0', ...x.license('Additional'), sourceFile: '../outside.bin', sourceSha256: '0'.repeat(64), sourceBytes: 1 }), /package source file escapes/],
  ['duplicate additional component', (x) => { const extra = { name: 'Extra', version: '1.0.0', ...x.license('Extra'), sourceFile: 'build.rs', sourceSha256: sha256(Buffer.from('fn main() {}\r\n')), sourceBytes: 14 }; x.manifest.entries[1].nativeComponents.push(extra, { ...extra }); }, /identity is invalid or duplicated/],
];

for (const [label, mutate, pattern] of failures) {
  test(`${label} rejects before replacing existing payload`, (t) => {
    const item = reviewedNativeFixture(t);
    mutate(item);
    item.save();
    assert.throws(item.run, pattern);
    assertPreserved(item);
  });
}

test('pending manifest guard runs before Cargo/input reads', (t) => {
  const item = reviewedNativeFixture(t);
  item.manifest.status = 'pending';
  delete item.manifest.cargoManifestSha256;
  item.save();
  assert.throws(() => generateThirdPartyNotices({ root: item.root, mode: 'prepare',
    cargoTree: join(item.root, 'absent-tree'), cargoMetadata: join(item.root, 'absent-metadata') }), /pending exact-version/);
  assertPreserved(item);
});

for (const reviewed of [true, false]) {
  for (const invalid of ['https://', 'http://example.test/sqlite-source']) {
    test(`known SQLite version rejects provenance URL ${invalid} with provenanceReviewed=${reviewed}`, (t) => {
      const item = reviewedNativeFixture(t);
      item.sqlite.provenanceReviewed = reviewed;
      item.sqlite.provenanceUrl = invalid;
      item.save();
      assert.throws(item.run, /authoritative HTTPS native provenance URL is invalid/);
      assertPreserved(item);
    });
  }
}

test('reviewed provenance can use authoritative source URL fallback for known and unresolved native versions', (t) => {
  const item = reviewedNativeFixture(t);
  item.sqlite.provenanceReviewed = true;
  delete item.webview.provenanceUrl;
  item.save();
  const payload = readFileSync(item.run().output, 'utf8');
  assert.ok(payload.includes(`Reviewed provenance: ${item.sqlite.sourceUrl}\n`));
  assert.ok(payload.includes(`Reviewed provenance: ${item.webview.sourceUrl}\n`));
});

test('contained additional native component is independently reviewed and its NOTICE is emitted', (t) => {
  const item = reviewedNativeFixture(t);
  const sourceFile = 'extra.bin';
  const bytes = Buffer.from([1, 0, 255]);
  item.write(`src-tauri/vendor/webview2-com-sys/${sourceFile}`, bytes);
  item.write('legal/third-party/native-notice.txt', 'Native NOTICE attribution\n');
  item.manifest.entries[1].nativeComponents.push({ name: 'Extra', version: '1.0.0', ...item.license('Extra', 'Apache-2.0'),
    sourceFile, sourceSha256: sha256(bytes), sourceBytes: bytes.length, reviewedBy: 'extra reviewer', reviewedAt: '2026-10-01',
    noticeFiles: [{ file: 'legal/third-party/native-notice.txt', sha256: sha256(Buffer.from('Native NOTICE attribution\n')) }] });
  item.save();
  const payload = readFileSync(item.run().output, 'utf8');
  assert.match(payload, /native:cargo:webview2-com-sys@0\.38\.2\/Extra@1\.0\.0/);
  assert.match(payload, /Native NOTICE attribution/);
  assert.match(payload, /Reviewed by: extra reviewer/);
});

test('escaping installed native bytes through a junction are rejected', (t) => {
  const item = reviewedNativeFixture(t);
  const outside = mkdtempSync(join(tmpdir(), 'what-changed-notice-native-outside-'));
  t.after(() => rmSync(outside, { recursive: true, force: true }));
  writeFileSync(join(outside, 'WebView2Loader.dll'), Buffer.from([0x4d, 0x5a, 0, 0xff]));
  const directory = join(item.root, 'src-tauri/vendor/webview2-com-sys/x64');
  rmSync(directory, { recursive: true });
  symlinkSync(outside, directory, 'junction');
  assert.throws(item.run, /package source file escapes its package directory/);
  assertPreserved(item);
  assert.equal(existsSync(join(outside, 'THIRD_PARTY_NOTICES.txt')), false);
});
