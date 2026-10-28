import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');

/** A build-feature addition does not approve new license terms or runtime bytes. */
export function assertFeatureOnlyChange(before, after) {
  const feature = '    "Win32_System_JobObjects",';
  const normalize = (value) => value.replaceAll('\r\n', '\n');
  const lines = normalize(after).split('\n');
  if (lines.filter((line) => line === feature).length !== 1
    || lines.filter((line) => line !== feature).join('\n') !== normalize(before)) {
    throw new Error('The Cargo manifest differs beyond the reviewed Windows JobObjects feature');
  }
}

export function verifyPreviewNoticeContinuity(projectRoot = root) {
  const manifest = JSON.parse(readFileSync(resolve(projectRoot, 'scripts/third-party-notices-manifest.json'), 'utf8'));
  const continuity = manifest.inputContinuity;
  if (!continuity || continuity.reason !== 'existing-windows-jobobjects-feature'
    || !/^[a-f0-9]{40}$/.test(continuity.reviewedSourceCommit ?? '')) throw new Error('Missing reviewed input continuity');
  const before = execFileSync('git', ['show', `${continuity.reviewedSourceCommit}:src-tauri/Cargo.toml`], { cwd: projectRoot });
  if (hash(before) !== continuity.reviewedCargoManifestSha256) throw new Error('Reviewed baseline hash differs');
  const after = readFileSync(resolve(projectRoot, 'src-tauri/Cargo.toml'));
  assertFeatureOnlyChange(before.toString('utf8'), after.toString('utf8'));
  if (hash(after) !== manifest.cargoManifestSha256) throw new Error('Current manifest hash differs');
  const baseline = JSON.parse(execFileSync('git', ['show', `${continuity.reviewedSourceCommit}:scripts/third-party-notices-manifest.json`], { cwd: projectRoot }));
  const publicReviewers = (value) => JSON.stringify(value).replaceAll('Salyyy', 'project-maintainer');
  if (baseline.status !== 'reviewed' || publicReviewers(baseline.entries) !== publicReviewers(manifest.entries)
    || JSON.stringify(baseline.approvalEvidence) !== JSON.stringify(manifest.approvalEvidence)
    || baseline.cargoLockSha256 !== manifest.cargoLockSha256 || baseline.bunLockSha256 !== manifest.bunLockSha256) {
    throw new Error('Reviewed package/license choices changed; new review required');
  }
  return { reviewedPackages: manifest.entries.length, delta: continuity.reason, runtimeNoticeApproval: false };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { console.log(JSON.stringify(verifyPreviewNoticeContinuity())); }
  catch (error) { console.error(`Notice continuity blocked: ${error.message}`); process.exitCode = 1; }
}
