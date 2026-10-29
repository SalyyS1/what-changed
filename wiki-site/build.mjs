import { build as bundle } from 'vite';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, copyFile, lstat, rm, realpath, readdir } from 'node:fs/promises';
import { dirname, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { site, docs, aliases } from './site.config.mjs';
import { layout, source, url } from './src/layout.mjs';
import { renderMarkdown } from './src/markdown.mjs';
import { landing } from './src/landing.mjs';
import { docsHub, howItWorks, demoPage, downloadPage, legacyPage, changelogPage, supportPage } from './src/pages.mjs';
import { readApprovedRelease } from '../scripts/check-download-manifest.mjs';
import { checkSite } from '../scripts/check-site.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = resolve(root, site.output), expected = resolve(root, 'wiki-site/dist');
if (output !== expected || await realpath(resolve(root, 'wiki-site')) !== resolve(await realpath(root), 'wiki-site')) throw new Error('Unsafe site output owner');
try { if ((await lstat(output)).isSymbolicLink()) throw new Error('Site output cannot be a link'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
await bundle({
  configFile: false, root: resolve(root, 'wiki-site'), publicDir: false, logLevel: 'warn',
  build: { outDir: resolve(output, 'assets'), emptyOutDir: false, sourcemap: false, minify: true,
    lib: { entry: resolve(root, 'wiki-site/src/client.mjs'), formats: ['es'], fileName: () => 'client.js' },
    rollupOptions: { output: { chunkFileNames: '[name]-[hash].js' } }, reportCompressedSize: false },
});
const hash = (value) => createHash('sha256').update(value).digest('hex');
const inputs = [], searchIndex = [], generated = [];
const sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
async function page(route, title, description, body, options = {}) {
  const path = route === '404.html' ? route : `${route}index.html`;
  await mkdir(dirname(resolve(output, path)), { recursive: true });
  await writeFile(resolve(output, path), layout({ route, title, description, body, ...options }));
  generated.push(path);
}
await page('', 'Build with AI. Stay in charge.', 'Preserve working behavior, review evidence, and own the next AI-made change.', landing());
await page('how-it-works/', 'How it works', 'Follow one booking change from intent through owner decision.', howItWorks());
await page('demo/', 'Interactive booking example', 'Explore synthetic passed, failed, blocked, and stale evidence in 2D and optional 3D.', demoPage());
await page('docs/', 'Documentation', 'Read and search the development documentation.', docsHub());
await page('docs/legacy/', 'Legacy diagnostics', 'Preserved What Changed guides and source links.', legacyPage());
await page('changelog/', 'Changelog', 'Development work and approved releases are labeled separately.', changelogPage());
await page('support/', 'Support and security', 'Report a synthetic reproduction and keep sensitive evidence private.', supportPage());
await page('vi/', 'Làm app với AI. Giữ quyền quyết định.', 'Ghi điều cần giữ, xem bằng chứng rồi tự quyết định nhận thay đổi.', landing(true), { lang: 'vi' });
let release, releaseError;
try { release = await readApprovedRelease(root); } catch (error) { releaseError = error.message; }
await page('download/', 'Download status', 'Only an exact approved release can enable a download.', downloadPage(release, releaseError));
for (const [id, title, description] of docs) {
  const path = `docs/ownvero/${id}.md`, text = await readFile(resolve(root, path), 'utf8');
  const revision = hash(text).slice(0, 12); inputs.push({ path, sha256: hash(text) });
  const rendered = renderMarkdown(text, (href) => {
    if (/^[a-z-]+\.md(?:#[a-z0-9-]+)?$/.test(href)) {
      const [file, anchor] = href.split('#');
      if (docs.some(([name]) => `${name}.md` === file)) return `${url(`docs/${file.slice(0, -3)}/`)}${anchor ? `#${anchor}` : ''}`;
    }
    if (href.startsWith('../../') && !href.slice(6).includes('..')) return source(href.slice(6));
    return href;
  });
  await page(`docs/${id}/`, title, description, rendered.html, { doc: rendered, revision });
  searchIndex.push({ title, route: `docs/${id}/`, text, excerpt: description });
}
for (const [name, title] of [['quickstart', 'Bắt đầu với một thay đổi'], ['glossary', 'Thuật ngữ Việt–Anh']]) {
  const path = `wiki-site/content/${name}-vi.md`, text = await readFile(resolve(root, path), 'utf8');
  const rendered = renderMarkdown(text); inputs.push({ path, sha256: hash(text) });
  await page(`vi/${name}/`, title, title, rendered.html, { lang: 'vi', doc: rendered, revision: hash(text).slice(0, 12) });
  searchIndex.push({ title, route: `vi/${name}/`, text, excerpt: 'Hướng dẫn tiếng Việt · technical reference remains English' });
}
await page('404.html', 'Page not found', 'Return to the documentation or home page.', `<section class="page-heading wrap"><p class="eyebrow">404</p><h1>This page is not here.</h1><p>Use the <a href="${url('docs/')}">documentation</a> or return to <a href="${url()}">the homepage</a>.</p></section>`);
await copyFile(resolve(root, 'wiki-site/styles.css'), resolve(output, 'styles.css'));
await copyFile(resolve(root, 'assets/brand/ownvero/mark.svg'), resolve(output, 'assets/mark.svg'));
await writeFile(resolve(output, 'search-index.json'), JSON.stringify(searchIndex));
await writeFile(resolve(output, 'legacy-aliases.json'), JSON.stringify({ schemaVersion: 1, basePath: site.basePath, aliases }, null, 2));
await writeFile(resolve(output, '.nojekyll'), '');
const sourcePaths = ['wiki-site/build.mjs', 'wiki-site/site.config.mjs', 'wiki-site/styles.css',
  'wiki-site/release-manifest.json', 'assets/brand/ownvero/mark.svg', 'release/release-manifest.schema.json',
  'scripts/check-site.mjs', 'scripts/check-download-manifest.mjs',
  ...(await readdir(resolve(root, 'wiki-site/src'))).filter((name) => name.endsWith('.mjs')).map((name) => `wiki-site/src/${name}`)];
for (const path of sourcePaths) inputs.push({ path, sha256: hash(await readFile(resolve(root, path))) });
inputs.sort((left, right) => left.path.localeCompare(right.path));
const assets = (await readdir(resolve(output, 'assets'))).map((name) => `assets/${name}`);
const allowedFiles = [...generated, ...assets, 'styles.css', 'search-index.json', 'legacy-aliases.json', '.nojekyll', 'build-manifest.json'].sort();
await writeFile(resolve(output, 'build-manifest.json'), JSON.stringify({
  schemaVersion: 1, mode: 'local-source-preview', publicationApproved: false, basePath: site.basePath,
  sourceCommit, docsVersion: site.docsVersion, reviewedAt: site.reviewedAt, inputs,
  sourceHash: hash(JSON.stringify(inputs)), allowedFiles,
}, null, 2));
await checkSite(output);
process.stdout.write(`Site built: ${relative(root, output)} (${generated.length} pages; download ${releaseError ? 'invalid/unavailable' : release.status})\n`);
