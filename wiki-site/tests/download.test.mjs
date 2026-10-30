import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { validateRelease, readApprovedRelease } from '../../scripts/check-download-manifest.mjs';
import { downloadPage } from '../src/pages.mjs';

function fixture() {
  return {
    schemaVersion: 1, status: 'approved', product: 'Ownvero', version: '1.2.3', channel: 'preview',
    sourceCommit: 'a'.repeat(40), builtAt: '2026-10-20T00:00:00Z', approvedAt: '2026-10-21T00:00:00Z',
    platform: 'windows', architecture: 'x86_64',
    assetUrl: 'https://github.com/SalyyS1/what-changed/releases/download/v1.2.3/Ownvero-Setup-x64.exe',
    bytes: 123456, sha256: 'b'.repeat(64),
    sourceAssetUrl: 'https://github.com/SalyyS1/what-changed/releases/download/v1.2.3/Ownvero-Source.zip',
    sourceBytes: 789012, sourceSha256: 'c'.repeat(64), signingStatus: 'unsigned',
    notesUrl: 'https://github.com/SalyyS1/what-changed/releases/tag/v1.2.3',
    noticesUrl: `https://github.com/SalyyS1/what-changed/blob/${'a'.repeat(40)}/THIRD_PARTY_NOTICES.md`,
    docsVersion: '1.2.3', minOS: 'Windows 10 x64',
  };
}
test('immutable structural fixture is rendered with exact hash, byte count, notes and notices', () => {
  const release = validateRelease(fixture()), html = downloadPage(release);
  for (const value of [release.sha256, release.assetUrl, release.sourceSha256, release.sourceAssetUrl, release.notesUrl, release.noticesUrl, '123,456 bytes', '789,012 bytes', 'Unknown publisher']) assert.ok(html.includes(value));
});
test('redirects, arbitrary origins, tag mismatch, paths, metadata omissions and extras fail closed', () => {
  const variants = [
    { assetUrl: 'https://github.com/SalyyS1/what-changed/releases/latest/download/setup.exe' },
    { assetUrl: 'https://example.com/setup.exe' }, { assetUrl: `${fixture().assetUrl}?redirect=1` },
    { assetUrl: fixture().assetUrl.replace('v1.2.3/', 'v1.2.4/') },
    { assetUrl: fixture().assetUrl.replace('Ownvero-Setup-x64.exe', '../setup.exe') },
    { sha256: 'bad' }, { bytes: 0 }, { sourceSha256: 'bad' }, { sourceBytes: 0 }, { sourceCommit: 'main' }, { platform: 'macos' },
    { approvedAt: null }, { approvedAt: '2020-01-01T00:00:00Z' },
    { docsVersion: '' }, { runnerVersion: '1.0.0' }, { extra: true },
    { noticesUrl: 'https://github.com/SalyyS1/what-changed/blob/main/THIRD_PARTY_NOTICES.md' },
    { sourceAssetUrl: 'https://example.com/source.zip' },
    { sourceAssetUrl: 'https://github.com/SalyyS1/what-changed/releases/download/v1.2.4/source.zip' },
    { sourceAssetUrl: 'https://github.com/SalyyS1/what-changed/releases/download/v1.2.3/../source.zip' },
  ];
  for (const variant of variants) assert.throws(() => validateRelease({ ...fixture(), ...variant }), /Download unavailable/);
  const missing = fixture(); delete missing.sha256; assert.throws(() => validateRelease(missing));
});
test('approved release requires a distinct source archive bound to the same tag', () => {
  const missing = fixture(); delete missing.sourceAssetUrl; assert.throws(() => validateRelease(missing), /Download unavailable/);
  const duplicate = { ...fixture(), sourceAssetUrl: fixture().assetUrl }; assert.throws(() => validateRelease(duplicate), /Download unavailable/);
});
test('unavailable and error pages never include an installer fallback', () => {
  const manifest = validateRelease({ schemaVersion: 1, status: 'unavailable', reason: 'Approval missing' });
  for (const html of [downloadPage(manifest), downloadPage(undefined, 'Invalid manifest')]) {
    assert.ok(!html.includes('data-download')); assert.ok(!html.includes('.exe')); assert.ok(html.includes('No installer link is enabled'));
  }
  assert.throws(() => validateRelease({ ...manifest, assetUrl: fixture().assetUrl }));
});
test('structural approval cannot enable a release without the human approval ledger', async () => {
  const root = await mkdtemp(join(tmpdir(), 'ownvero-release-'));
  try {
    await mkdir(join(root, 'wiki-site')); await writeFile(join(root, 'wiki-site/release-manifest.json'), JSON.stringify(fixture()));
    await assert.rejects(readApprovedRelease(root));
    await writeFile(join(root, 'wiki-site/release-manifest.json'), '{malformed');
    await assert.rejects(readApprovedRelease(root));
  } finally { await rm(root, { recursive: true, force: true }); }
});
