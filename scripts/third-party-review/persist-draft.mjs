import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { outputPath, packageKey, readJson, sha256 } from './filesystem.mjs';
import { verifyReviewInputsUnchanged } from './inputs.mjs';

export function persistReviewDraft({ root, draft, copies, inputFiles } = {}) {
  const draftPath = outputPath(root, 'legal/third-party/review-draft.json');
  if (existsSync(draftPath)) {
    const previous = readJson(draftPath, 'existing review draft');
    if (previous.status !== 'pending' || previous.reviewedBy || previous.obligationsReviewed === true) {
      throw new Error('refusing to replace a reviewed or non-pending draft');
    }
  }
  const planned = copies.map((copy) => ({ ...copy, path: outputPath(root, copy.file) }));
  const destinations = new Set();
  for (const copy of planned) {
    if (destinations.has(copy.path)) throw new Error(`duplicate source text destination: ${copy.file}`);
    destinations.add(copy.path);
    if (existsSync(copy.path) && !readFileSync(copy.path).equals(copy.content)) {
      throw new Error(`refusing to overwrite different existing third-party text: ${copy.file}`);
    }
  }
  const bytes = Buffer.from(`${JSON.stringify(draft, null, 2)}\n`);
  // This is the last preflight step. Nothing above it changes the filesystem;
  // lock/build-input changes during collection fail before any output mutation.
  verifyReviewInputsUnchanged(root, draft.inputs, inputFiles);
  for (const copy of planned) {
    if (existsSync(copy.path)) continue;
    mkdirSync(dirname(copy.path), { recursive: true });
    writeFileSync(copy.path, copy.content, { flag: 'wx' });
  }
  mkdirSync(dirname(draftPath), { recursive: true });
  if (!existsSync(draftPath) || !readFileSync(draftPath).equals(bytes)) {
    const temporary = outputPath(root, `legal/third-party/review-draft.${process.pid}.tmp`);
    writeFileSync(temporary, bytes, { flag: 'wx' });
    renameSync(temporary, draftPath);
  }
  return { output: draftPath, packages: draft.entries.length, sourceFiles: planned.length, sha256: sha256(bytes),
    issues: draft.entries.filter((item) => item.issues.length).map((item) => ({ package: packageKey(item), issues: item.issues })) };
}
