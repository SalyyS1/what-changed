import { mkdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';

const root = fileURLToPath(new URL('..', import.meta.url));
const work = resolve(root, '.qa/runtime-notice-20261028');
const destination = resolve(root, 'legal/third-party/runner/chrome-151.0.7922.34');
await mkdir(work, { recursive: true });
await mkdir(destination, { recursive: true });
const { chromium } = await import(pathToFileURL(resolve(root, 'runner/bundle/playwright-core/index.mjs')).href);
let browser;
try {
  browser = await chromium.launch({
    executablePath: resolve(root, 'runner/bundle/browsers/chromium-1234/chrome-win64/chrome.exe'),
    headless: true, chromiumSandbox: true,
    env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.SystemRoot, SystemDrive: process.env.SystemRoot.slice(0, 2), TEMP: work, TMP: work, USERPROFILE: work, LOCALAPPDATA: work, PATH: '' },
    args: ['--disable-background-networking', '--disable-component-update', '--disable-sync', '--no-first-run', '--disable-quic', '--host-resolver-rules=MAP * ~NOTFOUND'],
  });
  const context = await browser.newContext({ serviceWorkers: 'block', acceptDownloads: false });
  await context.route(/^https?:/, (route) => route.abort('blockedbyclient'));
  const page = await context.newPage();
  const records = [];
  for (const [pageName, file] of [['credits', 'CHROME-CREDITS.txt'], ['terms', 'CHROME-TERMS.txt']]) {
    await page.goto(`chrome://${pageName}`, { waitUntil: 'domcontentloaded' });
    // The credits page collapses every license by default. textContent retains
    // the complete embedded terms; innerText would copy only component titles.
    const bytes = Buffer.from(await page.locator('body').textContent());
    if (bytes.length < 500) throw new Error(`Internal ${pageName} page is incomplete`);
    await writeFile(join(destination, file), bytes);
    records.push({ file, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
    if (pageName === 'credits') {
      const html = Buffer.from(await page.content());
      await writeFile(join(work, 'CHROME-CREDITS.html'), html);
      records.push({ file: 'CHROME-CREDITS.html', bytes: html.length, sha256: createHash('sha256').update(html).digest('hex') });
    }
  }
  console.log(JSON.stringify({ browserVersion: browser.version(), source: 'matched packaged internal browser pages', networkRequestsPermitted: false, records }));
} finally { await browser?.close(); }
