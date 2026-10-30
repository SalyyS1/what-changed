import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readFile, realpath, stat } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkApprovals } from './check-ownvero-approvals.mjs';
import { readApprovedRelease } from './check-download-manifest.mjs';
import { checkRunnerBundle, runnerVersions } from './check-runner-bundle.mjs';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const requiredOwnerEvidence = Object.freeze([
  'automated_source', 'native_install_upgrade', 'privacy_purge_restore', 'runner_packaged_clean_windows',
  'runner_disabled_no_outbound', 'runner_authorized_egress', 'runner_cancel_parent_crash',
  'maps_accessibility_fallback', 'four_hour_soak', 'real_owner_repeat_use', 'license_delta_review', 'site_rendered',
]);

async function readOwned(root, entry) {
  if (!entry || typeof entry.path !== 'string' || isAbsolute(entry.path) || entry.path.includes('\\') || entry.path.split('/').some((part) => !part || part === '.' || part === '..') || !/^[a-f0-9]{64}$/.test(entry.sha256)) throw new Error('Invalid frozen evidence reference');
  const path = await realpath(resolve(root, entry.path));
  const local = relative(await realpath(root), path);
  if (!local || local === '..' || local.startsWith(`..${sep}`) || isAbsolute(local)) throw new Error('Frozen evidence escapes the project');
  const meta = await stat(path);
  if (!meta.isFile() || meta.size > 1024 * 1024 * 1024 || (entry.bytes !== undefined && meta.size !== entry.bytes)) throw new Error('Frozen artifact size mismatch');
  const digest = createHash('sha256');
  for await (const bytes of createReadStream(path)) digest.update(bytes);
  if (digest.digest('hex') !== entry.sha256) throw new Error(`Frozen evidence changed: ${entry.path}`);
}

/** Validates exact evidence references; a well-formed pending packet remains NO-GO. */
export async function assessOwnerRelease(packet, { root = projectRoot, publish = false } = {}) {
  if (packet?.schemaVersion !== 1 || !/^[a-f0-9]{40}$/.test(packet.sourceCommit ?? '') || !Array.isArray(packet.artifacts) || !Array.isArray(packet.verification)) throw new Error('Invalid owner candidate packet');
  if (packet.artifacts.length > 30 || packet.verification.length > 60 || new Set(packet.artifacts.map((item) => item.path)).size !== packet.artifacts.length || new Set(packet.verification.map((item) => item.name)).size !== packet.verification.length) throw new Error('Duplicate or excessive candidate references');
  for (const artifact of packet.artifacts) await readOwned(root, artifact);
  const missing = [];
  if (packet.state !== 'frozen-clean-source') missing.push('frozen_clean_source');
  for (const name of requiredOwnerEvidence) {
    const evidence = packet.verification.find((item) => item.name === name);
    if (evidence?.status !== 'pass') { missing.push(name); continue; }
    if (evidence.sourceCommit !== packet.sourceCommit || !evidence.reference) throw new Error(`Evidence ${name} does not identify this source`);
    await readOwned(root, evidence.reference);
  }
  const release = await readApprovedRelease(root);
  if (release.status !== 'approved') missing.push('approved_download_manifest');
  if (release.status === 'approved') {
    if (release.sourceCommit !== packet.sourceCommit) throw new Error('Site and candidate refer to different source revisions');
    const installer = packet.artifacts.find((artifact) => artifact.kind === 'installer');
    if (!installer || installer.sha256 !== release.sha256 || installer.bytes !== release.bytes) throw new Error('Site and candidate installer bytes differ');
    const sourceArchive = packet.artifacts.find((artifact) => artifact.kind === 'source-zip');
    if (!sourceArchive || sourceArchive.sha256 !== release.sourceSha256 || sourceArchive.bytes !== release.sourceBytes) throw new Error('Site and candidate source archive bytes differ');
    if (release.runnerVersion || packet.runnerBundle) {
      const runnerPackage = JSON.parse(await readFile(resolve(root, 'runner/package.json'), 'utf8'));
      if (!packet.runnerBundle || release.runnerVersion !== runnerPackage.version || release.browserRevision !== runnerVersions.chromiumRevision) throw new Error('Site runner versions differ from the candidate');
      await readOwned(root, { path: `${packet.runnerBundle.path}/bundle-inventory.json`, sha256: packet.runnerBundle.inventorySha256 });
      const bundle = await checkRunnerBundle(resolve(root, packet.runnerBundle.path), { sourceRoot: root });
      if (bundle.inventorySha256 !== packet.runnerBundle.inventorySha256) throw new Error('Runner inventory changed after freeze');
    }
  }
  if (!packet.artifacts.some((artifact) => artifact.kind === 'installer')) missing.push('frozen_installer');
  if (!packet.artifacts.some((artifact) => artifact.kind === 'source-zip')) missing.push('frozen_source_archive');
  if (publish) {
    await checkApprovals({ root, required: ['G0', 'G1', 'G2', 'G3'] });
    const ledger = JSON.parse(await readFile(resolve(root, 'plans/261027-0817-ownvero-owner-workspace-web-wiki/approval-ledger.json'), 'utf8'));
    const packetArtifact = packet.packetPath;
    if (typeof packetArtifact !== 'string' || !ledger.gates.find((gate) => gate.id === 'G3')?.scopeFiles.includes(packetArtifact)) throw new Error('G3 does not bind this candidate evidence packet');
    if (missing.length) throw new Error(`Release NO-GO: ${missing.join(', ')}`);
  }
  return { status: missing.length ? 'NO-GO' : 'EVIDENCE-READY', sourceCommit: packet.sourceCommit, missing, publicationApproved: publish && missing.length === 0 };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2); const packetPath = args[args.indexOf('--candidate') + 1];
    if (!args.includes('--candidate') || !packetPath || packetPath.startsWith('--') || args.some((arg) => arg.startsWith('--') && !['--candidate', '--publish'].includes(arg))) throw new Error('Usage: node scripts/check-owner-release.mjs --candidate release/candidate-evidence.json [--publish]');
    const packet = JSON.parse(await readFile(resolve(projectRoot, packetPath), 'utf8'));
    packet.packetPath = packetPath.replaceAll('\\', '/');
    console.log(JSON.stringify(await assessOwnerRelease(packet, { publish: args.includes('--publish') }), null, 2));
  } catch (error) { console.error(`Owner release denied: ${error.message}`); process.exitCode = 1; }
}
