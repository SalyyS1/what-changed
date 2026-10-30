import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, cp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';
import { renderMarkdown } from '../src/markdown.mjs';
import { checkSite } from '../../scripts/check-site.mjs';
import { booking } from '../src/demo-data.mjs';
import { demoTemplate } from '../src/demo-template.mjs';
import { landing } from '../src/landing.mjs';

test('untrusted Markdown cannot inject HTML, handlers, or dangerous URLs', () => {
  const { html } = renderMarkdown('## Trust\n\n<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n[x](javascript:evil) [y](//evil.test) [safe](https://example.com)');
  assert.ok(!html.includes('<script>')); assert.ok(!html.includes('<img'));
  assert.ok(!html.includes('href="javascript:')); assert.ok(!html.includes('href="//'));
  assert.ok(html.includes('href="https://example.com"'));
});
test('duplicate headings produce stable unique targets', () => {
  const result = renderMarkdown('## Read\n\nA\n\n## Read\n\nB');
  assert.deepEqual(result.toc.map((item) => item.id), ['read', 'read-1']);
});
test('the list exposes the same exact nodes and explicit edges as the optional renderer', () => {
  const html = demoTemplate();
  for (const node of booking.nodes) assert.ok(html.includes(`data-node="${node.id}"`));
  for (const edge of booking.edges) {
    assert.ok(booking.nodes.some((node) => node.id === edge.from)); assert.ok(booking.nodes.some((node) => node.id === edge.to)); assert.ok(html.includes(edge.type));
  }
  assert.ok(html.includes('not yet made')); assert.equal(booking.states.blocked.after, 'blocked'); assert.equal(booking.states.stale.after, 'stale');
});
test('both landing languages expose the complete change workflow and static evidence graph', () => {
  const html = landing();
  for (const stage of ['Intent', 'Preserve', 'Brief', 'Verify', 'Decide']) assert.ok(html.includes(`</span>${stage}</button>`));
  for (const vi of [false, true]) {
    const page = landing(vi);
    assert.ok(page.includes('data-workflow'));
    assert.ok(page.includes('data-model-canvas'));
    for (const node of booking.nodes) assert.ok(page.includes(`data-poster-node="${node.id}"`));
    assert.ok(page.includes('<noscript>'));
    assert.ok(page.includes('href="#walkthrough"'));
    if (vi) {
      assert.ok(page.includes('Chỉ nhân viên được đổi lịch'));
      assert.ok(page.includes('Ý định nối với hai kịch bản kiểm tra'));
      assert.ok(!page.includes('Owner intent connects'));
    }
  }
});
test('the full demo keeps 3D opt-in and lazy', () => {
  const html = demoTemplate();
  assert.ok(html.includes('data-load-3d'));
  assert.ok(!html.includes('data-auto-3d="true"'));
});
test('generated routes, allowlist, private data boundary, and initial JS budget', async () => {
  const result = await checkSite(); assert.equal(result.pages, 19);
  const client = await readFile('wiki-site/dist/assets/client.js');
  assert.ok(gzipSync(client).length < 150 * 1024);
  assert.ok(!client.toString().includes('class WebGLRenderer'));
});
test('output allowlist rejects extra QA content and private text in an allowed page', async () => {
  const root = await mkdtemp(join(tmpdir(), 'ownvero-site-output-'));
  try {
    await cp('wiki-site/dist', root, { recursive: true });
    await writeFile(join(root, 'personal-qa.json'), '{}');
    await assert.rejects(checkSite(root), /Unexpected public output/);
    await rm(join(root, 'personal-qa.json'));
    await writeFile(join(root, 'index.html'), '<p>C:\\Users\\Example\\private</p>');
    await assert.rejects(checkSite(root), /Private data/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
