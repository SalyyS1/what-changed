import { readFileSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const root = fileURLToPath(new URL('..', import.meta.url));
const hash = (value) => createHash('sha256').update(value).digest('hex');
const records = [];
const sections = [];
const components = [
  { name: 'node', directory: 'node-v24.12.0', source: 'https://raw.githubusercontent.com/nodejs/node/v24.12.0/LICENSE', obligations: 'Retain the complete Node license and all embedded dependency notices without removing copyright, disclaimers or conditions.' },
  { name: 'playwright', directory: 'playwright-1.62.1', source: 'https://github.com/microsoft/playwright/tree/v1.62.1', obligations: 'Retain Apache-2.0 LICENSE, NOTICE, ThirdPartyNotices and every packaged .LICENSE sidecar. Runtime files are unmodified.' },
  { name: 'chromium', directory: 'chrome-151.0.7922.34', source: 'chrome://credits and chrome://terms from the matched packaged Chrome for Testing 151.0.7922.34', obligations: 'Retain the complete embedded credits and executable terms. Google Chrome executable terms are distinct from Chromium source licenses. Redistribution permission still requires owner review; no open-source-only classification is asserted.' },
];
for (const component of components) {
  component.sourceFiles = [];
  const base = resolve(root, 'legal/third-party/runner', component.directory);
  for (const file of readdirSync(base).sort()) {
    if (file.endsWith('.html')) continue;
    const content = readFileSync(join(base, file));
    const record = { path: relative(root, join(base, file)).replaceAll('\\', '/'), bytes: content.length, sha256: hash(content) };
    records.push(record); component.sourceFiles.push({ path: record.path, sha256: record.sha256 });
    sections.push(`\n--- ${component.name}: ${file} ---\nSource: ${component.source}\n\n${content.toString('utf8')}\n`);
  }
}
const inventory = readFileSync(resolve(root, 'release/runner-runtime-inventory.json'));
const versions = JSON.parse(inventory);
const payload = Buffer.from(`OWNVERO DEVELOPMENT RUNNER — THIRD-PARTY TERMS\nExact runtime inventory SHA256: ${hash(inventory)}\nThese texts do not certify redistribution permission or product acceptance.\n${sections.join('')}`);
writeFileSync(resolve(root, 'release/RUNNER_THIRD_PARTY_NOTICES.txt'), payload);
const packet = {
  schemaVersion: 1, status: 'pending', ...Object.fromEntries(['nodeVersion', 'playwrightVersion', 'chromiumRevision', 'browserExecutable', 'checksumSource'].map((key) => [key, versions[key]])),
  bundleInventorySha256: hash(inventory), obligationsReviewed: false, reviewedBy: null,
  components, noticePayload: { path: 'release/RUNNER_THIRD_PARTY_NOTICES.txt', sha256: hash(payload) },
  note: 'Prepared for exact-source review, not owner-approved. Chrome for Testing executable terms and all underlying credits are retained; redistribution is not presumed from Chromium source licensing.',
};
const output = resolve(root, 'plans/261028-1258-build-and-github-update');
mkdirSync(output, { recursive: true });
writeFileSync(join(output, 'proposed-runtime-review.json'), `${JSON.stringify(packet, null, 2)}\n`);
writeFileSync(join(output, 'runtime-sources.json'), `${JSON.stringify(records, null, 2)}\n`);
console.log(JSON.stringify({ status: 'pending', sourceFiles: records.length, noticeBytes: payload.length, noticeSha256: hash(payload), bundleInventorySha256: hash(inventory) }));
