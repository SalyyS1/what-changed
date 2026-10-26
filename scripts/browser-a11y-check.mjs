/**
 * Dev-only browser smoke for keyboard navigation and computed reduced motion.
 * It launches a local Vite server and a fresh headless Edge (Chromium) profile.
 * No Tauri bridge, user history, external URL, or production update feed is used.
 */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { get } from 'node:http';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { chromium } from 'playwright';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const host = '127.0.0.1';
const port = 5183;
const origin = `http://${host}:${port}`;
const labels = ['Home', 'Timeline', 'Investigations', 'Checkpoints', 'System Map', 'Settings', 'Diagnostics'];

function seconds(value) {
  return value.split(',').map((part) => {
    const duration = part.trim();
    const number = Number.parseFloat(duration);
    assert.ok(Number.isFinite(number), `Invalid computed CSS duration: ${duration}`);
    if (duration.endsWith('ms')) return number / 1000;
    assert.ok(duration.endsWith('s'), `Invalid computed CSS duration unit: ${duration}`);
    return number;
  });
}

async function requireFocus(page, locator, message) {
  const element = await locator.elementHandle();
  assert.ok(element, `${message}: element is missing`);
  try {
    await page.waitForFunction((target) => document.activeElement === target, element, { timeout: 10_000, polling: 100 });
  } catch {
    const active = await page.evaluate(() => {
      const node = document.activeElement;
      return {
        tag: node?.tagName ?? null,
        role: node?.getAttribute('role') ?? null,
        label: node?.getAttribute('aria-label') ?? null,
        dialogs: document.querySelectorAll('[role="dialog"]').length,
        dialogInputs: document.querySelectorAll('[role="dialog"] input').length,
      };
    });
    throw new Error(`${message}; active element: ${JSON.stringify(active)}`);
  }
}

async function requireFreeDevPort() {
  const probe = createServer();
  try {
    await new Promise((resolve, reject) => {
      probe.once('error', reject);
      probe.listen(port, host, resolve);
    });
  } catch (error) {
    throw new Error(`Port ${port} is occupied; stop the other server before this isolated browser check.`, { cause: error });
  } finally {
    if (probe.listening) await new Promise((resolve) => probe.close(resolve));
  }
}

async function waitForVite(server, output) {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) throw new Error(`Vite exited early (${server.exitCode}): ${output()}`);
    const ready = await new Promise((resolve) => {
      const request = get({ hostname: host, port, path: '/', timeout: 2_000 }, (response) => {
        response.resume();
        resolve(response.statusCode === 200);
      });
      request.once('timeout', () => { request.destroy(); resolve(false); });
      request.once('error', () => resolve(false));
    });
    if (ready) return;
    await delay(250);
  }
  throw new Error(`Vite did not serve the app in 45 seconds: ${output()}`);
}

async function stopServer(server) {
  if (server.exitCode !== null || !server.pid) return;
  const exited = new Promise((resolve) => server.once('exit', resolve));
  server.kill();
  await Promise.race([
    exited,
    delay(3_000),
  ]);
  if (server.exitCode === null && process.platform === 'win32') {
    // The exact PID came from our own child process. Do not touch unrelated processes.
    const stopper = spawn('taskkill', ['/PID', String(server.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' });
    await new Promise((resolve) => stopper.once('exit', resolve));
  }
}

async function checkNavigation(page) {
  const nav = page.getByRole('navigation', { name: 'Main' });
  const links = nav.getByRole('link');
  assert.equal(await links.count(), labels.length, 'Expected the seven primary navigation links');
  for (const [index, label] of labels.entries()) {
    assert.equal(await links.nth(index).getAttribute('title'), `${label} (Ctrl+${index + 1})`);
  }

  await links.first().focus();
  for (let index = 1; index < labels.length; index += 1) {
    await page.keyboard.press('Tab');
    const current = links.nth(index);
    assert.ok(await current.evaluate((element) => document.activeElement === element), `Tab order broke before ${labels[index]}`);
    const outline = await current.evaluate((element) => {
      const style = getComputedStyle(element);
      return { style: style.outlineStyle, width: Number.parseFloat(style.outlineWidth) };
    });
    assert.equal(outline.style, 'solid', `${labels[index]} has no visible keyboard focus ring`);
    assert.ok(outline.width >= 2, `${labels[index]} focus ring is thinner than 2px`);
  }
  await page.keyboard.press('Shift+Tab');
  assert.ok(await links.nth(labels.length - 2).evaluate((element) => document.activeElement === element), 'Shift+Tab did not return to the previous link');
  return links.nth(labels.length - 2);
}

async function checkGlobalSearch(page, returnFocus) {
  await page.keyboard.press('Control+k');
  const dialog = page.getByRole('dialog', { name: 'Search changes' });
  await dialog.waitFor({ state: 'visible' });
  const input = dialog.getByRole('textbox', { name: 'Search history' });
  await input.waitFor({ state: 'visible' });
  await requireFocus(page, input, 'Ctrl+K did not focus the search field');

  for (let index = 0; index < 3; index += 1) {
    await page.keyboard.press('Tab');
    assert.ok(await dialog.evaluate((element) => element.contains(document.activeElement)), 'Search dialog leaked Tab focus');
  }
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'hidden' });
  await requireFocus(page, returnFocus, 'Escape did not restore focus to the original navigation link');
}

async function checkReducedMotion(page) {
  const sidebar = page.getByTestId('sidebar');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  const ordinary = seconds(await sidebar.evaluate((element) => getComputedStyle(element).transitionDuration));
  assert.ok(ordinary.some((value) => value >= 0.1), 'Control transition is absent, so the reduced-motion check would be vacuous');

  await page.emulateMedia({ reducedMotion: 'reduce' });
  assert.ok(await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches));
  const reduced = seconds(await sidebar.evaluate((element) => getComputedStyle(element).transitionDuration));
  assert.ok(reduced.every((value) => value <= 0.000002), `Sidebar still transitions under reduced motion: ${reduced}`);

  // This preview is populated entirely by the checked-in synthetic dev fixture.
  const preview = page.getByRole('button', { name: 'Open ready state' });
  await preview.focus();
  await page.keyboard.press('Enter');
  const dialog = page.getByRole('dialog', { name: 'Search changes' });
  await dialog.waitFor({ state: 'visible' });
  assert.ok(await dialog.getByRole('option').count() > 0, 'Ready fixture did not render synthetic search results');
  const animation = await dialog.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      name: style.animationName,
      duration: style.animationDuration,
      iteration: style.animationIterationCount,
      playState: style.animationPlayState,
    };
  });
  assert.notEqual(animation.name, 'none', 'Dialog has no baseline animation to neutralise');
  assert.ok(seconds(animation.duration).every((value) => value <= 0.000002), `Dialog still animates under reduced motion: ${animation.duration}`);
  assert.equal(animation.iteration, '1');
  assert.equal(animation.playState, 'paused');
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'hidden' });
  await requireFocus(page, preview, 'Fixture dialog did not return focus to its opener');
}

async function main() {
  await requireFreeDevPort();
  const viteScript = join(root, 'node_modules', 'vite', 'bin', 'vite.js');
  assert.ok(existsSync(viteScript), 'Vite is not installed; run bun install before the browser check');
  const server = spawn(process.execPath, [viteScript, '--host', host, '--port', String(port), '--strictPort'], {
    cwd: root,
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let log = '';
  const append = (chunk) => { log = `${log}${chunk.toString()}`.slice(-4_000); };
  server.stdout.on('data', append);
  server.stderr.on('data', append);
  let browser;
  try {
    await waitForVite(server, () => log);
    browser = await chromium.launch({
      channel: 'msedge',
      headless: true,
      args: ['--disable-background-networking', '--no-proxy-server'],
    });
    const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, reducedMotion: 'reduce' });
    const page = await context.newPage();
    page.setDefaultTimeout(60_000);
    const offOrigin = [];
    const pageErrors = [];
    page.on('pageerror', (error) => pageErrors.push(error.message));
    await page.route('**/*', (route) => {
      const request = route.request();
      if (new URL(request.url()).origin !== origin) {
        offOrigin.push(request.url());
        return route.abort();
      }
      return route.continue();
    });
    await page.goto(`${origin}/#/`, { waitUntil: 'commit' });
    await page.getByRole('heading', { name: 'Home', level: 1 }).waitFor({ state: 'visible' });
    assert.equal(await page.evaluate(() => '__TAURI_INTERNALS__' in window), false, 'Browser check must not attach to user history');

    const returnFocus = await checkNavigation(page);
    await checkGlobalSearch(page, returnFocus);
    await page.evaluate(() => { window.location.hash = '#/dev/states'; });
    await page.getByTestId('dev-states').waitFor({ state: 'visible', timeout: 120_000 });
    assert.ok(await page.getByRole('heading', { name: 'States harness', level: 1 }).isVisible());
    await checkReducedMotion(page);
    assert.deepEqual(offOrigin, [], 'The dev preview attempted an external request');
    assert.deepEqual(pageErrors, [], 'The dev preview raised an uncaught browser error');
    console.log('PASS: dev-source headless Edge/Chromium — seven-link Tab/Shift+Tab order and focus ring; Ctrl+K, modal Tab containment, Escape focus restoration; synthetic ready dialog; computed reduced-motion transition and animation.');
  } finally {
    await browser?.close();
    await stopServer(server);
  }
}

main().catch((error) => {
  console.error('FAIL: browser accessibility check:', error);
  process.exitCode = 1;
});
