import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createConnection } from 'node:net';
import { mkdir, writeFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const base = 'http://127.0.0.1:5184/what-changed/';
const artifacts = '.qa/ownvero-site';
const delay = (ms) => new Promise((done) => setTimeout(done, ms));
const occupied = () => new Promise((done) => {
  const socket = createConnection({ host: '127.0.0.1', port: 5184 });
  socket.once('connect', () => { socket.destroy(); done(true); }); socket.once('error', () => done(false));
});
test('rendered static navigation, search, demo, base path, no-JS and responsive behavior', async () => {
  assert.equal(await occupied(), false, '5184 already has an owner; stop/reuse explicitly before running this suite');
  const server = spawn(process.execPath, ['wiki-site/serve.mjs'], { stdio: 'pipe', windowsHide: true });
  let browser;
  try {
    for (let count = 0; count < 50 && !await occupied(); count++) await delay(100);
    browser = await chromium.launch({ headless: true, channel: process.env.SITE_BROWSER_CHANNEL || 'msedge' });
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const page = await context.newPage(), errors = [], outbound = [], requests = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => { requests.push(request.url()); if (!request.url().startsWith('http://127.0.0.1:5184')) outbound.push(request.url()); });
    await mkdir(artifacts, { recursive: true });
    for (const [width, height] of [[360, 800], [768, 1024], [1440, 900]]) {
      await page.setViewportSize({ width, height }); await page.goto(base); await page.locator('h1').waitFor();
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `landing overflow at ${width}`);
      await page.screenshot({ path: `${artifacts}/landing-${width}.png`, fullPage: true });
    }
    for (const route of ['how-it-works/', 'docs/privacy/', 'vi/', 'vi/quickstart/', 'vi/glossary/', 'changelog/', 'support/', 'download/']) {
      const response = await page.goto(`${base}${route}`); assert.equal(response.status(), 200); assert.ok(await page.locator('h1').textContent());
      await page.reload(); assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `${route} overflow`);
    }
    await page.goto(`${base}missing-route/`); assert.equal(await page.locator('h1').textContent(), 'This page is not here.');
    await page.goto(`${base}#timeline`); await page.waitForURL('**/docs/legacy/#timeline');
    await page.goto(`${base}docs/`); await page.getByRole('searchbox').fill('stale'); await page.getByText('matching pages', { exact: false }).waitFor();
    assert.ok(await page.locator('[data-search-results] a').count() > 0);
    await page.getByRole('searchbox').fill('zzznomatchzzz'); await page.getByText('No matching page.', { exact: false }).waitFor();
    await page.goto(`${base}download/`); assert.equal(await page.locator('[data-download]').count(), 0);
    await page.locator('[data-platform] select').selectOption('macos'); assert.ok((await page.locator('[data-platform-state]').textContent()).includes('no installer'));
    const requestsBeforeDemo = requests.length;
    await page.goto(`${base}demo/`);
    assert.equal(requests.slice(requestsBeforeDemo).some((path) => /workbench-3d-/.test(path)), false, '3D loaded before the visitor asked');
    for (const value of ['failed', 'blocked', 'stale', 'current']) {
      await page.locator('[data-state]').selectOption(value);
      assert.equal(await page.locator('[data-verdict]').getAttribute('data-verdict'), value === 'current' ? 'passed' : value);
    }
    await page.locator('[data-build]').selectOption('before'); assert.ok((await page.locator('[data-verdict]').textContent()).includes('booking-demo-a'));
    await page.locator('[data-build]').selectOption('after');
    await page.locator('[data-node="rule"] summary').click(); await page.locator('[data-select="rule"]').click();
    assert.ok((await page.locator('[data-inspector]').textContent()).includes('requires'));
    await page.getByRole('button', { name: 'Load 3D workbench', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('[data-load-3d]').textContent !== 'Loading 3D…');
    const webglRendered = await page.locator('[data-three-canvas] canvas').count() === 1;
    if (webglRendered) {
      await page.screenshot({ path: `${artifacts}/demo-3d.png`, fullPage: true });
      await page.locator('[data-explode]').uncheck(); await page.locator('[data-layer]').selectOption('1');
      await page.getByRole('button', { name: 'Close 3D and release resources' }).click();
      assert.equal(await page.locator('[data-three-canvas] canvas').count(), 0);
    } else assert.ok((await page.locator('[data-inspector]').textContent()).includes('unavailable'));
    await page.emulateMedia({ reducedMotion: 'reduce' }); await page.goto(`${base}demo/`);
    await page.setViewportSize({ width: 360, height: 800 }); assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.screenshot({ path: `${artifacts}/demo-360.png`, fullPage: true });
    assert.deepEqual(errors, []); assert.deepEqual(outbound, []);
    const nojs = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 360, height: 800 } });
    const staticPage = await nojs.newPage(); await staticPage.goto(`${base}docs/verification/`);
    assert.ok((await staticPage.locator('main').textContent()).includes('Stale')); await staticPage.locator('.mobile-toc summary').click(); await staticPage.locator('.mobile-toc a[href="#read-the-state"]').click();
    await staticPage.goto(`${base}demo/`); await staticPage.locator('[data-node="rule"] summary').click();
    assert.ok(await staticPage.locator('[data-node="rule"]').getAttribute('open') !== null);
    await staticPage.screenshot({ path: `${artifacts}/demo-no-js.png`, fullPage: true });
    await writeFile(`${artifacts}/browser-results.json`, JSON.stringify({ webglRendered, runtimeErrors: errors, outboundRequests: outbound, testedWidths: [360, 768, 1440], noJavaScript: true, reducedMotion: true, initial3dLazy: true }, null, 2));
    await nojs.close(); await context.close();
  } finally {
    await browser?.close(); server.kill();
    await new Promise((done) => { if (server.exitCode !== null) done(); else server.once('exit', done); });
  }
});
