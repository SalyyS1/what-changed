import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { checkReviewedNoticeInputs } from './check-reviewed-notice-inputs.mjs';

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

function fixture(fn) {
  const root = resolve(mkdtempSync(join(tmpdir(), 'ownvero-notice-input-')));
  const licensePath = 'legal/third-party/cargo/fixture/1.2.3/LICENSE';
  const noticePath = 'legal/third-party/cargo/fixture/1.2.3/NOTICE';
  const license = Buffer.from('License fixture\r\nExact reviewed bytes.\r\n');
  const notice = Buffer.from('Notice fixture\r\nCopyright fixture author.\r\n');
  const manifestPath = join(root, 'scripts/third-party-notices-manifest.json');
  const manifest = {
    schemaVersion: 1, target: 'x86_64-pc-windows-msvc', status: 'reviewed',
    reviewedBy: 'project-owner', reviewedAt: '2026-10-30', obligationsReviewed: true,
    entries: [{ ecosystem: 'cargo', name: 'fixture', version: '1.2.3',
      licenseExpression: 'MIT', selectedLicense: 'MIT', obligationsReviewed: true,
      sourceUrl: 'https://example.invalid/fixture-1.2.3.crate',
      textFile: licensePath, textSha256: sha256(license),
      noticeFiles: [{ file: noticePath, sha256: sha256(notice) }],
    }],
  };
  try {
    for (const [path, bytes] of [[licensePath, license], [noticePath, notice]]) {
      mkdirSync(dirname(join(root, path)), { recursive: true }); writeFileSync(join(root, path), bytes);
    }
    mkdirSync(dirname(manifestPath), { recursive: true });
    const save = () => writeFileSync(manifestPath, JSON.stringify(manifest)); save();
    fn({ root, licensePath: join(root, licensePath), noticePath: join(root, noticePath), license, notice, manifest, manifestPath, save });
  } finally {
    assert.equal(dirname(root), resolve(tmpdir()));
    assert.ok(root.startsWith(join(resolve(tmpdir()), 'ownvero-notice-input-')));
    rmSync(root, { recursive: true, force: true });
  }
}

test('exact reviewed CRLF license and NOTICE inputs pass without modifying bytes', () => fixture(({ root, licensePath, noticePath, license, notice, manifestPath }) => {
  const manifestBefore = readFileSync(manifestPath); const result = checkReviewedNoticeInputs(root);
  assert.equal(result.reviewedPackages, 1); assert.equal(result.licenseTextChecks, 1); assert.equal(result.noticeTextChecks, 1);
  assert.equal(result.manifestSha256, sha256(manifestBefore));
  assert.deepEqual(readFileSync(licensePath), license); assert.deepEqual(readFileSync(noticePath), notice);
  assert.deepEqual(readFileSync(manifestPath), manifestBefore);
}));

test('LF normalization of reviewed CRLF license bytes fails', () => fixture(({ root, licensePath, license }) => {
  writeFileSync(licensePath, license.toString('utf8').replaceAll('\r\n', '\n'));
  assert.throws(() => checkReviewedNoticeInputs(root), /license text hash does not match reviewed bytes: cargo:fixture@1.2.3/);
}));

test('changed NOTICE bytes fail', () => fixture(({ root, noticePath, notice }) => {
  const changed = Buffer.from(notice); changed[0] ^= 1; writeFileSync(noticePath, changed);
  assert.throws(() => checkReviewedNoticeInputs(root), /NOTICE text hash does not match reviewed bytes/);
}));

test('missing NOTICE bytes fail', () => fixture(({ root, noticePath }) => {
  unlinkSync(noticePath);
  assert.throws(() => checkReviewedNoticeInputs(root), /NOTICE text is missing/);
}));

test('pending or incomplete human review cannot pass', () => fixture(({ root, manifest, save }) => {
  manifest.status = 'pending'; save();
  assert.throws(() => checkReviewedNoticeInputs(root), /existing completed review/);
  manifest.status = 'reviewed'; manifest.obligationsReviewed = false; save();
  assert.throws(() => checkReviewedNoticeInputs(root), /existing completed review/);
  manifest.obligationsReviewed = true; manifest.reviewedBy = ''; save();
  assert.throws(() => checkReviewedNoticeInputs(root), /existing completed review/);
}));

test('unsupported schema, empty coverage and duplicate packages cannot pass', () => fixture(({ root, manifest, save }) => {
  manifest.schemaVersion = 2; save();
  assert.throws(() => checkReviewedNoticeInputs(root), /unsupported schema or target/);
  manifest.schemaVersion = 1; const entry = manifest.entries[0]; manifest.entries = []; save();
  assert.throws(() => checkReviewedNoticeInputs(root), /bounded nonempty array/);
  manifest.entries = [entry, { ...entry }]; save();
  assert.throws(() => checkReviewedNoticeInputs(root), /Duplicate reviewed notice entry/);
}));
