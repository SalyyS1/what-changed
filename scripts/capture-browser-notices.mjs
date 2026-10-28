import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';

const root = fileURLToPath(new URL('..', import.meta.url));
const source = JSON.parse(await readFile(resolve(root, 'release/runner-browser-source.json'), 'utf8'));
await mkdir(resolve(root, '.qa'), { recursive: true });
const work = await mkdtemp(resolve(root, '.qa/runtime-notice-'));
const destination = resolve(root, `legal/third-party/runner/chromium-${source.version}`);
await mkdir(destination, { recursive: true });
const { chromium } = await import(pathToFileURL(resolve(root, 'runner/bundle/playwright-core/index.mjs')).href);
let browser;
try {
  browser = await chromium.launch({
    executablePath: resolve(root, 'runner/bundle', source.browserExecutable),
    headless: true, chromiumSandbox: true,
    env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.SystemRoot, SystemDrive: process.env.SystemRoot.slice(0, 2), TEMP: work, TMP: work, USERPROFILE: work, LOCALAPPDATA: work, PATH: '' },
    args: ['--disable-background-networking', '--disable-component-update', '--disable-sync', '--no-first-run', '--disable-quic', '--host-resolver-rules=MAP * ~NOTFOUND'],
  });
  if (browser.version() !== source.browserVersion) throw new Error('Browser notice capture version differs from the source descriptor');
  const context = await browser.newContext({ serviceWorkers: 'block', acceptDownloads: false });
  await context.route(/^https?:/, (route) => route.abort('blockedbyclient'));
  const page = await context.newPage();
  const records = [];
  for (const [pageName, file] of [['credits', 'CHROMIUM-CREDITS.txt'], ['terms', 'CHROMIUM-TERMS.txt']]) {
    await page.goto(`chrome://${pageName}`, { waitUntil: 'domcontentloaded' });
    // The credits page collapses every license by default. textContent retains
    // the complete embedded terms; innerText would copy only component titles.
    const bytes = Buffer.from(await page.locator('body').textContent());
    if (pageName === 'credits' && (bytes.length < 100000 || bytes.toString('utf8').includes('This is sample credits page'))) throw new Error('Internal browser credits are incomplete');
    // Unbranded Chromium intentionally has an empty terms page. Its grant is
    // in the root and component licenses, all retained by the notice packet.
    await writeFile(join(destination, file), bytes);
    records.push({ file, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
    if (pageName === 'credits') {
      const html = Buffer.from(await page.content());
      await writeFile(join(work, 'CHROMIUM-CREDITS.html'), html);
      records.push({ file: 'CHROMIUM-CREDITS.html', bytes: html.length, sha256: createHash('sha256').update(html).digest('hex') });
    }
  }
  console.log(JSON.stringify({ browserVersion: browser.version(), source: 'matched packaged internal browser pages', networkRequestsPermitted: false, records }));
} finally { await browser?.close(); }
