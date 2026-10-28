#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
export const legacyLaunchFiles = ['launch-article-en.md', 'launch-article-vi.md', 'community-posts.md',
  'demo-script.md', 'press-kit.md', 'release-notes-template.md'];
export const archiveLabel = '**Publication status: ARCHIVED — legacy What Changed material; not approved Ownvero copy.**';
const claims = [
  [/\b(?:100\s*%|completely|universally)\s+(?:safe|secure)\b/i, 'universal safety'],
  [/\bfully secure\b|an toàn (?:tuyệt đối|100\s*%)/i, 'absolute security'],
  [/\b(?:guarantees?|guaranteed)\s+(?:no regressions|safety|security|every behavior|all behavior)\b/i, 'guaranteed outcome'],
  [/\b(?:tests?|verifies?)\s+(?:everything|every app|all apps)\b/i, 'unbounded verification'],
  [/\b(?:autopilot|autonomous(?:ly)?)\b|tự (?:động )?(?:sửa|chấp nhận|triển khai) (?:app|ứng dụng|code)/i, 'unsupported autonomy'],
  [/\b(?:automatically|autonomously)\s+(?:accepts?|fixes?|deploys?|rewrites?)\b/i, 'automatic owner decision or code mutation'],
  [/\b(?:proves? the root cause|finds? the root cause|fixes your pc|real-time everything|zero memory|replacement for process monitor)\b/i, 'unsupported diagnostic claim'],
];

/** AI-builder wording is factual scope; checks target unsupported promises. */
export function validateLaunchCopy(text, { archived = false } = {}) {
  const errors = [];
  if (!/^\*\*Evidence state:/m.test(text)) errors.push('missing evidence-state label');
  if (archived) {
    if (!text.includes(archiveLabel)) errors.push('legacy material lacks explicit archive disposition');
    if (!text.includes('[Current owner-workspace messaging](messaging.md)')) errors.push('archive lacks current messaging route');
    return errors;
  }
  for (const [pattern, reason] of claims) if (pattern.test(text)) errors.push(`unsupported public claim: ${reason}`);
  return errors;
}

export async function checkLaunchCopy(directory = path.join(root, 'docs/launch')) {
  const errors = [];
  for (const name of legacyLaunchFiles) {
    try {
      const text = await readFile(path.join(directory, name), 'utf8');
      errors.push(...validateLaunchCopy(text, { archived: true }).map((message) => `${name}: ${message}`));
    } catch { errors.push(`missing launch copy: ${name}`); }
  }
  const messaging = await readFile(path.join(directory, 'messaging.md'), 'utf8');
  if (!/^# Launch messaging/m.test(messaging)) errors.push('messaging: missing canonical heading');
  if (!/## Anti-claims/.test(messaging)) errors.push('messaging: missing anti-claims section');
  errors.push(...validateLaunchCopy(messaging.split('## Anti-claims')[0]).map((message) => `messaging: ${message}`));
  if (!messaging.includes('G2') || !messaging.includes('G3')) errors.push('messaging: missing publication boundary');
  if (errors.length) throw new Error(errors.join('\n'));
  return { archived: legacyLaunchFiles.length, active: 1 };
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { const result = await checkLaunchCopy(); process.stdout.write(`check-launch-copy: ${result.active} active fact sheet; ${result.archived} explicitly archived legacy files\n`); }
  catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
}
