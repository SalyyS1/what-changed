import { readFile, readdir, lstat } from 'node:fs/promises';
import { resolve, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { site } from '../wiki-site/site.config.mjs';

async function walk(directory) {
  const files = [];
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, item.name);
    if ((await lstat(path)).isSymbolicLink()) throw new Error('Site output contains a symbolic link');
    if (item.isDirectory()) files.push(...await walk(path)); else files.push(path);
  }
  return files;
}
export async function checkSite(directory = resolve('wiki-site/dist')) {
  const manifest = JSON.parse(await readFile(join(directory, 'build-manifest.json'), 'utf8'));
  const expectedRoutes = ['', 'how-it-works/', 'demo/', 'download/', 'docs/', 'docs/legacy/', 'changelog/', 'support/',
    'vi/', 'vi/quickstart/', 'vi/glossary/', ...['getting-started', 'owner-workspace', 'verification', 'maps', 'privacy', 'faq', 'migration-guide'].map((name) => `docs/${name}/`)];
  const required = [...expectedRoutes.map((route) => `${route}index.html`), '404.html', 'styles.css', 'product.css', 'assets/client.js', 'assets/mark.svg', 'search-index.json', 'legacy-aliases.json', '.nojekyll', 'build-manifest.json'];
  const paths = await walk(directory), files = paths.map((path) => relative(directory, path).replaceAll('\\', '/'));
  if (manifest.basePath !== site.basePath || manifest.publicationApproved !== false || manifest.mode !== 'local-source-preview') throw new Error('Invalid site publication boundary');
  // Derive the allowable shape independently; the generated manifest cannot authorize arbitrary files.
  const validName = (name) => required.includes(name) || /^assets\/[a-zA-Z0-9_-]+\.js$/.test(name);
  for (const name of files) if (!validName(name) || !manifest.allowedFiles.includes(name)) throw new Error(`Unexpected public output: ${name}`);
  for (const name of required) if (!files.includes(name)) throw new Error(`Missing public route: ${name}`);
  if (manifest.allowedFiles.some((name) => !files.includes(name))) throw new Error('Output manifest has missing files');
  const textFiles = new Map();
  for (let index = 0; index < paths.length; index++) {
    const text = await readFile(paths[index], 'utf8'); textFiles.set(files[index], text);
    if (/(?:C:[\\/]+Users[\\/]|D:[\\/]+Project[\\/]|\.qa[\\/]|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY|gh[pousr]_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9]{20,}|@tauri-apps|__TAURI_INTERNALS__|invoke\s*\()/i.test(text)) throw new Error(`Private data or desktop bridge in ${files[index]}`);
  }
  for (const [name, html] of textFiles) {
    if (!name.endsWith('.html')) continue;
    if (!html.includes('<main id="main"') || !html.includes('class="skip-link"')) throw new Error(`Missing accessible reading route: ${name}`);
    if (/<(?:script|style)\b[^>]*>(?!\s*<\/)([\s\S]+?)<\//i.test(html) || /\son[a-z]+\s*=|javascript\s*:/i.test(html)) throw new Error(`Unsafe inline content: ${name}`);
    for (const match of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
      const href = match[1].replaceAll('&amp;', '&');
      if (href.startsWith('https://')) continue;
      if (!href.startsWith(site.basePath) && !href.startsWith('#')) throw new Error(`Invalid base path in ${name}: ${href}`);
      const [pathname, anchor] = href.split('#');
      const target = pathname ? pathname.slice(site.basePath.length) : name;
      const destination = !target || target.endsWith('/') ? `${target}index.html` : target;
      if (!textFiles.has(destination)) throw new Error(`Broken link in ${name}: ${href}`);
      if (anchor && !textFiles.get(destination).includes(`id="${anchor}"`)) throw new Error(`Missing anchor in ${name}: ${href}`);
    }
  }
  const index = JSON.parse(textFiles.get('search-index.json'));
  if (index.length !== 9 || index.some((item) => !expectedRoutes.includes(item.route))) throw new Error('Search contains unexpected documents');
  return { files: files.length, pages: files.filter((name) => name.endsWith('.html')).length };
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { const result = await checkSite(); process.stdout.write(`Site check: ${result.pages} pages, ${result.files} allowlisted files\n`); }
  catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
}
