import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startBookingFixture } from './booking-fixture.mjs';

const root = dirname(fileURLToPath(import.meta.url));
const runRoot = join(root, 'run');
await mkdir(runRoot, { recursive: true });
const resultPath = join(runRoot, 'smoke-result.json');
const started = performance.now();
const assertions = [];
let browser;
let fixture;
let watchdog;
let failure;
const result = { schemaVersion: 1, executionId: process.env.OWNVERO_SPIKE_EXECUTION_ID, fixture: 'synthetic-booking', assertions,
  freshMachineProof: false, networkConfinementProof: false };
try {
  assert.equal(resolve(process.execPath).toLowerCase(), join(root, 'node.exe').toLowerCase());
  assert.equal(process.env.PATH || '', '');
  for (const name of ['NODE_OPTIONS', 'NODE_PATH', 'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY']) {
    assert.equal(process.env[name], undefined, `inherited ${name}`);
  }
  const inventory = JSON.parse(await readFile(join(root, 'bundle-inventory.json'), 'utf8'));
  assert.equal(inventory.nodeVersion, process.version);
  assert.ok(inventory.files.some((entry) => entry.path === inventory.browserExecutable));
  const bundledPackage = JSON.parse(await readFile(join(root, 'playwright-core/package.json'), 'utf8'));
  assert.equal(bundledPackage.version, inventory.playwrightVersion);
  assert.equal(bundledPackage.version, '1.62.1');
  for (const entry of inventory.files) {
    assert.ok(!entry.path.includes('..') && !entry.path.includes('\\') && !entry.path.startsWith('/'));
    const bytes = await readFile(join(root, entry.path));
    assert.equal(bytes.length, entry.bytes);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.sha256, `bundle changed: ${entry.path}`);
  }
  const { chromium } = await import('./playwright-core/index.mjs');
  const browsers = JSON.parse(await readFile(join(root, 'playwright-core/browsers.json'), 'utf8'));
  assert.equal(browsers.browsers.find((entry) => entry.name === 'chromium').revision, inventory.chromiumRevision);
  assertions.push({ id: 'packaged-runtime-only', status: 'passed', checkedFiles: inventory.files.length });
  fixture = await startBookingFixture(join(runRoot, 'synthetic-bookings.json'));
  browser = await chromium.launch({ headless: true, chromiumSandbox: true,
    executablePath: join(root, inventory.browserExecutable),
    // No personal profile or TCP debugging server; Playwright uses its private pipe.
    args: ['--disable-background-networking', '--disable-quic', '--force-webrtc-ip-handling-policy=disable_non_proxied_udp'],
  });
  watchdog = setTimeout(() => { browser?.close().catch(() => {}); }, 45000);
  result.browserVersion = browser.version();
  result.nodeVersion = process.version;
  result.playwrightVersion = inventory.playwrightVersion;
  const contexts = await Promise.all([0, 1].map(() => browser.newContext({ serviceWorkers: 'block', acceptDownloads: false })));
  const [customer, staff] = contexts;
  const denied = [];
  for (const context of contexts) {
    await context.route('**/*', async (route) => {
      const url = new URL(route.request().url());
      if (url.origin !== fixture.origin) { denied.push('unexpected-origin'); await route.abort('blockedbyclient'); }
      else await route.continue();
    });
  }
  await customer.request.post(`${fixture.origin}/session?role=customer`);
  await staff.request.post(`${fixture.origin}/session?role=staff`);
  const markerBefore = await (await staff.request.get(`${fixture.origin}/.well-known/ownvero.json`)).json();
  const page = await customer.newPage();
  await page.goto(fixture.origin);
  await page.getByRole('textbox', { name: 'Name', exact: true }).fill('Synthetic booking');
  await page.getByRole('textbox', { name: 'Note', exact: true }).fill('Window seat');
  const responsePromise = page.waitForResponse((response) => response.url() === `${fixture.origin}/bookings` && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Book', exact: true }).click();
  const creation = await responsePromise;
  assert.equal(creation.status(), 201);
  const created = await creation.json();
  assert.match(created.id, /^[0-9a-f-]{36}$/);
  const persisted = await staff.request.get(`${fixture.origin}/bookings/${created.id}`);
  assert.equal(persisted.status(), 200);
  assert.deepEqual(await persisted.json(), { id: created.id, name: 'Synthetic booking', note: 'Window seat' });
  assertions.push({ id: 'separate-staff-persisted-read-back', status: 'passed' });
  await customer.request.post(`${fixture.origin}/session?role=other-customer`);
  const deniedResponse = await customer.request.get(`${fixture.origin}/bookings/${created.id}`);
  assert.equal(deniedResponse.status(), 403);
  assert.deepEqual(await deniedResponse.json(), { error: 'denied' });
  assertions.push({ id: 'other-customer-record-denied', status: 'passed' });
  const markerAfter = await (await staff.request.get(`${fixture.origin}/.well-known/ownvero.json`)).json();
  assert.deepEqual(markerAfter, markerBefore);
  assert.equal(markerAfter.environmentType, 'test');
  assertions.push({ id: 'stable-target-reported-revision', status: 'passed' });
  assert.deepEqual(denied, []);
  result.status = 'passed';
} catch (error) {
  failure = error;
  result.status = 'failed';
  // No page text, request headers, cookies or raw browser output in the evidence file.
  result.failure = error.code || error.name;
} finally {
  clearTimeout(watchdog);
  await browser?.close().catch(() => {});
  await fixture?.close();
  result.elapsedMs = Math.round(performance.now() - started);
  result.remainingProfileEntries = (await readdir(runRoot)).filter((name) => name.startsWith('playwright_'));
  await writeFile(resultPath, `${JSON.stringify(result, null, 2)}\n`);
}
if (failure) { process.stderr.write(`Packaged runtime smoke failed (${failure.code || failure.name})\n`); process.exitCode = 1; }
