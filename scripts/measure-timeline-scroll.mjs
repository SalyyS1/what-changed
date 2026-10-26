import { chromium } from 'playwright';
import { createServer } from 'vite';

// Development-source smoke measurement only. Installed WebView2 remains a
// separate manual release gate; this script neither launches nor certifies it.
const server = await createServer({
  logLevel: 'error',
  server: { host: '127.0.0.1', port: 0, strictPort: true },
});
let browser;

try {
  await server.listen();
  const address = server.httpServer?.address();
  if (!address || typeof address === 'string') throw new Error('Vite did not bind a local TCP port');
  const origin = `http://127.0.0.1:${address.port}`;
  const response = await fetch(`${origin}/`, { signal: AbortSignal.timeout(15_000) });
  if (!response.ok) throw new Error(`Vite readiness returned HTTP ${response.status}`);

  // Use the machine's installed Edge; Playwright must not fetch a browser.
  browser = await chromium.launch({
    channel: 'msedge',
    headless: true,
    args: ['--disable-background-networking', '--no-proxy-server'],
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.route('**/*', (route) => new URL(route.request().url()).origin === origin
    ? route.continue()
    : route.abort());
  await page.goto(`${origin}/#/`, { waitUntil: 'commit', timeout: 30_000 });
  await page.getByRole('heading', { name: 'Home', level: 1 }).waitFor({ state: 'visible', timeout: 60_000 });
  await page.evaluate(() => { window.location.hash = '#/dev/perf'; });
  await page.getByTestId('perf-corpus-size').getByText('100,000 events').waitFor({ timeout: 120_000 });

  const viewport = page.getByTestId('timeline-viewport');
  for (const [fraction, minimumIndex] of [[0.5, 45_000], [0.9, 85_000]]) {
    await viewport.evaluate((element, position) => {
      element.scrollTop = (element.scrollHeight - element.clientHeight) * position;
    }, fraction);
    await page.waitForFunction((threshold) => {
      const element = document.querySelector('[data-testid="timeline-viewport"]');
      if (!element) return false;
      const bounds = element.getBoundingClientRect();
      return [...element.querySelectorAll('[data-index]')].some((row) => {
        const rect = row.getBoundingClientRect();
        const label = row.querySelector('[data-testid="timeline-row"]')?.getAttribute('aria-label') ?? '';
        const eventId = Number(label.match(/Fixture #(\d+):/)?.[1]);
        return Number(row.getAttribute('data-index')) > threshold
          && eventId > threshold - 2_000
          && rect.bottom > bounds.top && rect.top < bounds.bottom;
      });
    }, minimumIndex);
  }

  await page.getByRole('button', { name: 'Measure 600 frames' }).click();
  await page.waitForFunction(() => /Pass|Review/.test(document.querySelector('[data-testid="perf-frame-status"]')?.textContent ?? ''), null, { timeout: 90_000 });
  const result = {
    evidence: 'dev-source headless Microsoft Edge (Chromium); not installed WebView2',
    method: '100k loaded events; three 200-frame segments at 0%, 50%, 95%; 37 px/frame; repositioning excluded',
    browser: browser.version(),
    corpus: (await page.getByTestId('perf-corpus-size').innerText()).trim(),
    p95: (await page.getByTestId('perf-frame-p95').innerText()).trim(),
    status: (await page.getByTestId('perf-frame-status').innerText()).trim(),
    coverage: (await page.getByTestId('perf-scroll-coverage').innerText()).trim(),
  };
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  if (!result.status.endsWith('Pass')) process.exitCode = 1;
} finally {
  await browser?.close();
  await server.close();
}
