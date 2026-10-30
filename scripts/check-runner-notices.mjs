import { createHash } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkRunnerBundle, runnerVersions } from './check-runner-bundle.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
async function ownedText(path) {
  if (typeof path !== 'string' || isAbsolute(path) || path.includes('\\') || path.split('/').some((part) => !part || part === '.' || part === '..')) throw new Error('Invalid notice source path');
  const actual = await realpath(resolve(root, path)); const local = relative(await realpath(root), actual);
  if (local.startsWith(`..${sep}`) || local === '..' || isAbsolute(local)) throw new Error('Notice source escaped the repository');
  return readFile(actual);
}
try {
  const review = JSON.parse(await readFile(resolve(root, 'release/runner-notice-review.json'), 'utf8'));
  if (review.schemaVersion !== 1 || review.status !== 'reviewed' || !review.obligationsReviewed || !review.reviewedBy || !review.approvalEvidence) throw new Error('The new Node/Playwright/Chromium notice delta has not been reviewed by the owner');
  if (Object.entries(runnerVersions).some(([key, value]) => review[key] !== value)) throw new Error('Reviewed runtime notice versions differ');
  const bundle = await checkRunnerBundle(resolve(root, 'runner/bundle'));
  if (review.bundleInventorySha256 !== bundle.inventorySha256) throw new Error('Runtime bytes changed after notice review');
  const components = ['node', 'playwright', 'chromium'];
  if (!Array.isArray(review.components) || components.some((name) => !review.components.some((item) => item.name === name))) throw new Error('A runtime notice component is missing');
  for (const item of review.components) {
    if (!item.sourceFiles?.length || !item.obligations?.trim()) throw new Error('Incomplete reviewed notice obligations');
    for (const source of item.sourceFiles) if (hash(await ownedText(source.path)) !== source.sha256) throw new Error('Reviewed notice text changed');
  }
  const raw = await ownedText(review.approvalEvidence.path);
  if (hash(raw) !== review.approvalEvidence.sha256) throw new Error('Runtime notice approval evidence changed');
  const evidence = JSON.parse(raw);
  if (evidence.actor !== 'user' || evidence.source !== 'user_message' || !evidence.answer?.trim() || evidence.bundleInventorySha256 !== bundle.inventorySha256) throw new Error('Runtime notice approval is not bound to this bundle');
  if (review.noticePayload?.path !== 'release/RUNNER_THIRD_PARTY_NOTICES.txt') throw new Error('Runtime notice payload must match the packaged resource path');
  const notices = await ownedText(review.noticePayload.path);
  if (hash(notices) !== review.noticePayload.sha256) throw new Error('Reviewed runtime notice payload changed');
  console.log('Runner exact byte inventory and reviewed notice delta match. Publication remains separately gated.');
} catch (error) { console.error(`Runner notices unavailable: ${error.message}`); process.exitCode = 1; }
