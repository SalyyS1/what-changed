import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const settledFrame = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
const canvasHash = async (page, path) => createHash('sha256').update(await page.locator('[data-three-canvas] canvas').screenshot({ path })).digest('hex');
async function assertRecoveredCanvas(page) {
  const actual = await page.locator('[data-three-canvas] canvas').screenshot({ path: '.qa/website-overhaul-scene-recovered.png' });
  const expected = await readFile('.qa/website-overhaul-scene-before.png');
  // Decode test PNGs outside the site's CSP, which correctly blocks data URLs.
  const comparison = await page.context().newPage();
  let difference;
  try { difference = await comparison.evaluate(async (sources) => {
    const images = await Promise.all(sources.map(async (source) => {
      const image = new Image(); image.src = `data:image/png;base64,${source}`; await image.decode(); return image;
    }));
    if (images[0].width !== images[1].width || images[0].height !== images[1].height) return { max: Infinity, mean: Infinity };
    const pixels = images.map((image) => {
      const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
      const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
      return context.getImageData(0, 0, canvas.width, canvas.height).data;
    });
    let max = 0, sum = 0;
    for (let i = 0; i < pixels[0].length; i++) { const delta = Math.abs(pixels[0][i] - pixels[1][i]); max = Math.max(max, delta); sum += delta; }
    return { max, mean: sum / pixels[0].length };
  }, [expected.toString('base64'), actual.toString('base64')]);
  } finally { await comparison.close(); }
  // Recreated graphics contexts can round antialiasing differently at tile edges.
  assert.ok(difference.max <= 32 && difference.mean < .1, `context recovery must retain the selected view: ${JSON.stringify(difference)}`);
}
const restorePage = (page) => page.evaluate(() => {
  window.dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true }));
  window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true }));
});
const loseGraphics = (page, selector) => page.locator(selector).evaluate((canvas) => {
  const extension = canvas.getContext('webgl2')?.getExtension('WEBGL_lose_context');
  if (!extension) return false;
  extension.loseContext(); return true;
});

/** Exercise pending imports, retained controls and actual graphics context loss. */
export async function verifyWorkflowLifecycle(browser, base) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  let releaseChunk;
  const chunkWait = new Promise((done) => { releaseChunk = done; });
  await context.route(/\/assets\/workbench-3d-.*\.js$/, async (route) => { await chunkWait; await route.continue(); });
  const page = await context.newPage();
  try {
    await page.goto(base, { waitUntil: 'domcontentloaded' });
    await page.waitForFunction(() => document.querySelector('[data-model-toggle]').textContent === '…');
    await restorePage(page);
    releaseChunk();
    await page.locator('[data-model-canvas] canvas').waitFor();
    await page.locator('[data-model-toggle]').click();
    await restorePage(page);
    assert.equal(await page.locator('[data-model-poster]').isVisible(), true, 'explicit 2D choice survives page restoration');
    assert.equal(await page.locator('[data-model-canvas] canvas').count(), 0);
    await page.locator('[data-model-toggle]').click();
    await page.locator('[data-model-canvas] canvas').waitFor();
    assert.equal(await loseGraphics(page, '[data-model-canvas] canvas'), true);
    await page.getByText('Graphics context lost.', { exact: false }).waitFor();
    assert.equal(await page.locator('[data-model-poster]').isVisible(), true);
    await page.locator('[data-model-toggle]').click();
    await page.locator('[data-model-canvas] canvas').waitFor();

    await page.goto(`${base}vi/`);
    await page.locator('[data-model-canvas] canvas').waitFor();
    assert.equal(await page.getByRole('button', { name: 'Tính năng: Đổi lịch hẹn', exact: true }).count(), 1);
    assert.equal(await page.locator('.three-label strong').allTextContents().then((labels) => labels.includes('Only staff can reschedule')), false);

    await page.goto(`${base}demo/`);
    await page.locator('[data-load-3d]').click();
    await page.locator('[data-three-canvas] canvas').waitFor();
    await page.locator('[data-layer]').selectOption('1');
    await page.locator('[data-explode]').uncheck();
    await page.locator('[data-zoom]').evaluate((input) => { input.value = '1.4'; });
    await page.locator('[data-zoom]').dispatchEvent('input');
    await settledFrame(page);
    const before = await canvasHash(page, '.qa/website-overhaul-scene-before.png');
    await restorePage(page);
    await page.locator('[data-three-canvas] canvas').waitFor(); await settledFrame(page);
    assert.equal(await page.locator('[data-three-canvas] .three-label:visible').count(), 2);
    assert.equal(await canvasHash(page), before, 'restored view matches retained zoom, layer and separation controls');
    assert.equal(await loseGraphics(page, '[data-three-canvas] canvas'), true);
    await page.getByRole('button', { name: 'Retry 3D workbench', exact: true }).click();
    await page.locator('[data-three-canvas] canvas').waitFor(); await settledFrame(page);
    assert.equal(await page.locator('[data-three-canvas] .three-label:visible').count(), 2);
    await assertRecoveredCanvas(page);
  } finally { releaseChunk(); await context.close(); }
}
