import { chromium } from 'playwright';
import { writeFile } from 'node:fs/promises';

const browser = await chromium.launch({ channel: 'msedge', headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
page.setDefaultTimeout(60000);
await page.routeWebSocket('**', (socket) => socket.close());
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
const results = {};
try {
  await page.goto('http://localhost:5183/plans/reports/owner-map-preview.html', { waitUntil: 'domcontentloaded', timeout: 60000 });
  await page.getByRole('listbox').focus();
  await page.getByRole('listbox').press('End');
  await page.getByRole('button', { name: /^Fit selected$/ }).click();
  await page.getByRole('button', { name: /^Zoom in$/ }).click();
  results.zoom2d = await page.locator('[aria-label="2D capability and evidence map"] > div').getAttribute('style');
  await page.getByRole('button', { name: /^3D$/ }).click();
  await page.locator('canvas').waitFor({ state: 'visible', timeout: 20000 });
  await page.getByText(/nonoverlapping labels shown/).waitFor();
  results.threeLoaded = true;
  results.selection3d = await page.getByRole('listbox').getAttribute('aria-activedescendant');
  results.inspector = await page.getByRole('region', { name: 'Selected evidence details' }).innerText();
  await page.getByRole('button', { name: /^Fit selected$/ }).click();
  await page.getByRole('button', { name: /^Zoom in$/ }).click();
  await page.locator('input[type="range"]').fill('10');
  await page.screenshot({ path: 'plans/reports/owner-map-render/3d-reduced-motion.png', fullPage: true });
  await page.locator('canvas').evaluate((canvas) => canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true })));
  await page.getByRole('status').filter({ hasText: 'context was lost' }).waitFor();
  results.contextFallback = await page.getByRole('button', { name: /^2D$/ }).getAttribute('aria-pressed');
  results.selectionFallback = await page.getByRole('listbox').getAttribute('aria-activedescendant');
  results.fallbackPreference = await page.evaluate(() => localStorage.getItem('owner-map-mode'));
  await page.emulateMedia({ forcedColors: 'active' });
  await page.getByText('Forced colors is active.', { exact: false }).waitFor();
  results.forcedColorsList = await page.getByRole('button', { name: /^List$/ }).getAttribute('aria-pressed');
  await page.screenshot({ path: 'plans/reports/owner-map-render/forced-colors.png', fullPage: true });
  await page.emulateMedia({ forcedColors: 'none' });
  await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
  await page.getByRole('listbox').focus(); await page.getByRole('listbox').press('End');
  await page.screenshot({ path: 'plans/reports/owner-map-render/2d-dark-inspector.png', fullPage: true });
  await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'light'));
  await page.getByText('Before / after evidence review', { exact: true }).click();
  await page.screenshot({ path: 'plans/reports/owner-map-render/before-after.png', fullPage: true });
  results.responsive = [];
  for (const width of [375, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await page.screenshot({ path: `plans/reports/owner-map-render/current-${width}.png`, fullPage: true });
    results.responsive.push(await page.evaluate(() => ({ width: innerWidth, scrollWidth: document.documentElement.scrollWidth, overflow: document.documentElement.scrollWidth > innerWidth })));
  }
  results.runtimeErrors = errors;
} finally {
  await writeFile('plans/reports/owner-map-browser-results.json', JSON.stringify(results, null, 2));
  await browser.close();
}
