import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { reviewedLicense } from './third-party-review/reviewed-license-notices.mjs';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** Check exact reviewed license/NOTICE inputs without resolving or building dependencies. */
export function checkReviewedNoticeInputs(root = projectRoot) {
  const bytes = readFileSync(resolve(root, 'scripts/third-party-notices-manifest.json'));
  const manifest = JSON.parse(bytes.toString('utf8'));
  if (manifest?.schemaVersion !== 1 || manifest.target !== 'x86_64-pc-windows-msvc') {
    throw new Error('Reviewed notice manifest has an unsupported schema or target');
  }
  if (manifest.status !== 'reviewed' || manifest.obligationsReviewed !== true
    || typeof manifest.reviewedBy !== 'string' || !manifest.reviewedBy.trim()
    || !Number.isFinite(Date.parse(manifest.reviewedAt ?? ''))) {
    throw new Error('License and NOTICE inputs require an existing completed review');
  }
  if (!Array.isArray(manifest.entries) || !manifest.entries.length || manifest.entries.length > 10000) {
    throw new Error('Reviewed notice entries must be a bounded nonempty array');
  }
  const packages = new Set(); let noticeTextChecks = 0;
  for (const entry of manifest.entries) {
    if (!entry || !['cargo', 'frontend'].includes(entry.ecosystem)
      || typeof entry.name !== 'string' || !entry.name.trim()
      || typeof entry.version !== 'string' || !entry.version.trim()) {
      throw new Error('Reviewed notice entry has an invalid package identity');
    }
    const key = `${entry.ecosystem}:${entry.name}@${entry.version}`;
    if (packages.has(key)) throw new Error(`Duplicate reviewed notice entry: ${key}`);
    packages.add(key);
    reviewedLicense(root, entry, key);
    noticeTextChecks += entry.noticeFiles.length;
  }
  return {
    reviewedPackages: packages.size, licenseTextChecks: packages.size, noticeTextChecks,
    manifestSha256: createHash('sha256').update(bytes).digest('hex'),
  };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 2) throw new Error('Usage: node scripts/check-reviewed-notice-inputs.mjs');
    console.log(JSON.stringify({ check: 'reviewed-license-notice-inputs', ...checkReviewedNoticeInputs() }));
  } catch (error) {
    console.error(`Reviewed notice input preflight blocked: ${error.message}`);
    console.error('Restore exact approved license/NOTICE bytes. A genuine source or terms change requires review; preserve the existing approval history.');
    process.exitCode = 1;
  }
}
