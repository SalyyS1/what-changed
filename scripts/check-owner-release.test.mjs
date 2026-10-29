import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assessOwnerRelease, requiredOwnerEvidence } from './check-owner-release.mjs';

const commit = 'a'.repeat(40);
async function fixture(fn) {
  const root = await mkdtemp(join(tmpdir(), 'ownvero-candidate-'));
  try {
    await mkdir(join(root, 'wiki-site')); await mkdir(join(root, 'evidence'));
    await writeFile(join(root, 'wiki-site/release-manifest.json'), JSON.stringify({ schemaVersion: 1, status: 'unavailable', reason: 'No approved artifact.' }));
    await fn(root);
  } finally { await rm(root, { recursive: true, force: true }); }
}
test('a valid incomplete local packet remains NO-GO with exact missing checks', () => fixture(async (root) => {
  const result = await assessOwnerRelease({ schemaVersion: 1, sourceCommit: commit, artifacts: [], verification: [] }, { root });
  assert.equal(result.status, 'NO-GO'); assert.equal(result.publicationApproved, false);
  assert.deepEqual(result.missing, ['frozen_clean_source', ...requiredOwnerEvidence, 'approved_download_manifest', 'frozen_installer']);
}));
test('changed evidence bytes and mismatched source cannot satisfy a candidate gate', () => fixture(async (root) => {
  const source = 'source proof'; const sha256 = createHash('sha256').update(source).digest('hex');
  await writeFile(join(root, 'evidence/result.txt'), `${source} changed`);
  const packet = { schemaVersion: 1, sourceCommit: commit, artifacts: [], verification: [{ name: requiredOwnerEvidence[0], status: 'pass', sourceCommit: commit, reference: { path: 'evidence/result.txt', sha256 } }] };
  await assert.rejects(assessOwnerRelease(packet, { root }), /changed/);
  packet.verification[0].sourceCommit = 'b'.repeat(40);
  await assert.rejects(assessOwnerRelease(packet, { root }), /source/);
}));
test('duplicate gates and traversal evidence are rejected before release assessment', () => fixture(async (root) => {
  const duplicate = { name: requiredOwnerEvidence[0], status: 'pending' };
  await assert.rejects(assessOwnerRelease({ schemaVersion: 1, sourceCommit: commit, artifacts: [], verification: [duplicate, duplicate] }, { root }), /Duplicate/);
  await assert.rejects(assessOwnerRelease({ schemaVersion: 1, sourceCommit: commit, artifacts: [{ path: '../outside', sha256: 'a'.repeat(64) }], verification: [] }, { root }), /Invalid/);
}));
test('an incomplete packet cannot publish without human bound approvals', () => fixture(async (root) => {
  await assert.rejects(assessOwnerRelease({ schemaVersion: 1, sourceCommit: commit, artifacts: [], verification: [] }, { root, publish: true }));
}));
