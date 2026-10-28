import { createHash } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

export const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const planDirectory = 'plans/261027-0817-ownvero-owner-workspace-web-wiki';
export const gateIds = ['G0', 'G1', 'G2', 'G3'];
const hash = (value) => createHash('sha256').update(value).digest('hex');

async function ownedFile(root, name) {
  if (typeof name !== 'string' || !name || name.includes('\\') || isAbsolute(name)
    || name.split('/').some((part) => !part || part === '.' || part === '..')) {
    throw new Error('Approval file must be a canonical repository-relative path');
  }
  const base = await realpath(root);
  const path = await realpath(resolve(base, name));
  const local = relative(base, path);
  if (!local || local === '..' || local.startsWith(`..${sep}`) || isAbsolute(local)) {
    throw new Error('Approval file escapes the repository');
  }
  return readFile(path);
}

/** Bind approval to ordered paths and exact content, independent of the caller's cwd. */
export async function scopeDigest(root, scopeFiles) {
  if (!Array.isArray(scopeFiles) || !scopeFiles.length || scopeFiles.length > 32
    || new Set(scopeFiles).size !== scopeFiles.length) {
    throw new Error('Approval scope must list distinct files');
  }
  const entries = [];
  for (const name of [...scopeFiles].sort()) {
    entries.push([name, hash(await ownedFile(root, name))]);
  }
  return hash(JSON.stringify(entries));
}

/** An audit guard, not a signature or proof of the identity of a human approver. */
export async function checkApprovals({ root = projectRoot, ledgerPath = `${planDirectory}/approval-ledger.json`, required }) {
  if (!Array.isArray(required) || !required.length || required.some((id) => !gateIds.includes(id))) {
    throw new Error('Unknown or empty approval gate selection');
  }
  const ledger = JSON.parse(await ownedFile(root, ledgerPath));
  if (ledger.schemaVersion !== 1 || ledger.plan !== planDirectory.split('/').at(-1)
    || !Array.isArray(ledger.gates) || ledger.gates.length !== gateIds.length
    || new Set(ledger.gates.map((gate) => gate.id)).size !== gateIds.length
    || ledger.gates.some((gate) => !gateIds.includes(gate.id))) {
    throw new Error('Invalid approval ledger');
  }
  for (const id of required) {
    const gate = ledger.gates.find((entry) => entry.id === id);
    if (gate.status !== 'approved' || !gate.evidence || !gate.scopeDigest || !gate.evidenceSha256) {
      throw new Error(`${id}: approval is missing`);
    }
    if (!gate.evidence.startsWith(`${planDirectory}/reports/`)) {
      throw new Error(`${id}: approval evidence has an unexpected owner`);
    }
    const raw = await ownedFile(root, gate.evidence);
    if (hash(raw) !== gate.evidenceSha256) throw new Error(`${id}: approval evidence changed`);
    const evidence = JSON.parse(raw);
    if (evidence.schemaVersion !== 1 || evidence.source !== 'user_message'
      || evidence.actor !== 'user' || !evidence.questionItemId
      || !evidence.question?.trim() || !evidence.answer?.trim()
      || !Number.isFinite(Date.parse(evidence.recordedAt))
      || evidence.approvals?.[id] !== gate.scopeDigest) {
      throw new Error(`${id}: invalid human decision record`);
    }
    if (await scopeDigest(root, gate.scopeFiles) !== gate.scopeDigest) {
      throw new Error(`${id}: approved scope changed; review the changed scope`);
    }
  }
  return [...new Set(required)];
}

export function requiredGates(args) {
  if (args.length === 1 && args[0] === '--publish') return gateIds;
  if (args.length === 2 && args[0] === '--gates') return args[1].split(',');
  if (args.length === 2 && args[0] === '--phase' && /^(?:[1-9]|1[0-2])$/.test(args[1])) {
    const phase = Number(args[1]);
    return phase >= 10 ? ['G0', 'G1', 'G2'] : ['G0', 'G1'];
  }
  throw new Error('Usage: node scripts/check-ownvero-approvals.mjs --phase 1..12 | --gates G0,G1 | --publish');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const required = requiredGates(process.argv.slice(2));
    const checked = await checkApprovals({ required });
    process.stdout.write(`Ownvero approval check: ${checked.join(', ')}\n`);
  } catch (error) {
    process.stderr.write(`Ownvero approval check denied: ${error.message}\n`);
    process.exitCode = 1;
  }
}
