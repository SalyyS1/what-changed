import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkApprovals, projectRoot, planDirectory } from './check-ownvero-approvals.mjs';

const manifestPath = 'wiki-site/release-manifest.json';
const required = ['schemaVersion', 'status', 'product', 'version', 'channel', 'sourceCommit',
  'builtAt', 'approvedAt', 'platform', 'architecture', 'assetUrl', 'bytes', 'sha256',
  'signingStatus', 'notesUrl', 'noticesUrl', 'docsVersion', 'minOS'];
const allowed = [...required, 'runnerVersion', 'browserRevision'];
const fail = (message) => { throw new Error(`Download unavailable: ${message}`); };

/** Structural checks are separate from the human G3 approval binding below. */
export function validateRelease(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value) || value.schemaVersion !== 1) fail('invalid schema');
  if (value.status === 'unavailable') {
    if (Object.keys(value).some((key) => !['schemaVersion', 'status', 'reason'].includes(key))
      || typeof value.reason !== 'string' || !value.reason.trim() || value.reason.length > 600) fail('invalid unavailable state');
    return value;
  }
  if (value.status !== 'approved' || Object.keys(value).some((key) => !allowed.includes(key))
    || required.some((key) => !(key in value))) fail('incomplete or unknown metadata');
  if (value.product !== 'Ownvero' || value.platform !== 'windows' || value.architecture !== 'x86_64'
    || !['preview', 'stable'].includes(value.channel) || !['unsigned', 'signed'].includes(value.signingStatus)) fail('unsupported release');
  if (typeof value.version !== 'string' || !/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(value.version)
    || !/^[a-f0-9]{40}$/.test(value.sourceCommit) || !/^[a-f0-9]{64}$/.test(value.sha256)
    || !Number.isSafeInteger(value.bytes) || value.bytes < 1 || value.bytes > 2147483648) fail('invalid artifact identity');
  for (const key of ['docsVersion', 'minOS', 'runnerVersion', 'browserRevision']) {
    if (key in value && (typeof value[key] !== 'string' || !value[key].trim() || value[key].length > (key === 'minOS' ? 120 : 80))) fail(`invalid ${key}`);
  }
  if (('runnerVersion' in value) !== ('browserRevision' in value)) fail('runner/browser versions must be paired');
  for (const key of ['builtAt', 'approvedAt']) {
    if (typeof value[key] !== 'string' || !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/.test(value[key])
      || !Number.isFinite(Date.parse(value[key]))) fail(`invalid ${key}`);
  }
  if (Date.parse(value.approvedAt) < Date.parse(value.builtAt) || Date.parse(value.approvedAt) > Date.now()) fail('invalid approval chronology');
  const prefix = `https://github.com/SalyyS1/what-changed/releases/download/v${value.version}/`;
  if (typeof value.assetUrl !== 'string' || !value.assetUrl.startsWith(prefix)
    || !/^[A-Za-z0-9_.-]+\.exe$/.test(value.assetUrl.slice(prefix.length))
    || value.assetUrl.slice(prefix.length).includes('..')) fail('asset must be an exact allowlisted GitHub release');
  const notes = `https://github.com/SalyyS1/what-changed/releases/tag/v${value.version}`;
  if (value.notesUrl !== notes) fail('notes must match the exact release tag');
  const noticesPrefix = `https://github.com/SalyyS1/what-changed/blob/${value.sourceCommit}/`;
  if (value.noticesUrl !== `${noticesPrefix}THIRD_PARTY_NOTICES.md`) fail('notices must match the source commit');
  return value;
}

export async function readApprovedRelease(root = projectRoot) {
  const value = validateRelease(JSON.parse(await readFile(resolve(root, manifestPath), 'utf8')));
  if (value.status === 'approved') {
    await checkApprovals({ root, required: ['G2', 'G3'] });
    const ledger = JSON.parse(await readFile(resolve(root, planDirectory, 'approval-ledger.json'), 'utf8'));
    if (!ledger.gates.find((gate) => gate.id === 'G3')?.scopeFiles?.includes(manifestPath)) fail('G3 must bind this exact manifest');
  }
  return value;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const release = await readApprovedRelease();
    process.stdout.write(`Download manifest: ${release.status}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
