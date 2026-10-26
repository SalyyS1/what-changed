import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { collectPackage } from './third-party-review/collect-package.mjs';
import { TARGET } from './third-party-review/filesystem.mjs';
import { captureReviewInputs } from './third-party-review/inputs.mjs';
import { persistReviewDraft } from './third-party-review/persist-draft.mjs';

const DEFAULT_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

// Preparation never changes the reviewed release manifest or emits a notice
// payload. A human still chooses licenses, checks native components and NOTICE
// obligations, and reviews one frozen release before the existing gate can pass.
export function prepareThirdPartyReview({ root = DEFAULT_ROOT, cargoTree, cargoMetadata, inputSnapshot, cargoSourceRoots } = {}) {
  root = resolve(root);
  const { inputs, candidates } = captureReviewInputs({ root, cargoTree, cargoMetadata, inputSnapshot });
  const collected = candidates.map((item) => collectPackage(root, item, cargoSourceRoots));
  const draft = {
    schemaVersion: 1, status: 'pending', target: TARGET, reviewedBy: null, reviewedAt: null,
    obligationsReviewed: false, publishable: false, inputs,
    evidenceScope: 'Windows normal,no-proc-macro dependency candidates and schema-2 Vite inputs; not final linked artifact proof.',
    entries: collected.map((item) => item.entry),
  };
  return persistReviewDraft({ root, draft, copies: collected.flatMap((item) => item.copies),
    inputFiles: { cargoTree, cargoMetadata, inputSnapshot } });
}

function argumentsFrom(argv) {
  const options = {};
  const flags = { '--root': 'root', '--cargo-tree': 'cargoTree', '--cargo-metadata': 'cargoMetadata', '--input-snapshot': 'inputSnapshot' };
  for (let index = 0; index < argv.length; index += 2) {
    const key = flags[argv[index]];
    const value = argv[index + 1];
    if (!key || !value || value.startsWith('--') || options[key]) {
      throw new Error('usage: node scripts/prepare-third-party-review.mjs [--root directory] [--cargo-tree file --cargo-metadata file --input-snapshot file]');
    }
    options[key] = value;
  }
  return options;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const result = prepareThirdPartyReview(argumentsFrom(process.argv.slice(2)));
    process.stdout.write(`third-party review draft: ${result.packages} packages, ${result.sourceFiles} original source files; pending, publishable=false; sha256=${result.sha256}\n`);
    for (const item of result.issues) process.stdout.write(`${item.package}: ${item.issues.join(', ')}\n`);
  } catch (error) {
    process.stderr.write(`third-party review preparation blocked: ${error.message}\n`);
    process.exitCode = 1;
  }
}
