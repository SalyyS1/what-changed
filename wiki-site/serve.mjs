import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { site } from './site.config.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), 'dist');
const mime = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript', '.json': 'application/json', '.svg': 'image/svg+xml' };
createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (!pathname.startsWith(site.basePath)) { response.writeHead(302, { Location: site.basePath }); response.end(); return; }
    let path = resolve(root, pathname.slice(site.basePath.length));
    if (path !== root && !path.startsWith(`${root}${sep}`)) throw new Error('Path outside output');
    try { if ((await stat(path)).isDirectory()) path = resolve(path, 'index.html'); } catch { path = resolve(root, '404.html'); response.statusCode = 404; }
    response.setHeader('Content-Type', mime[extname(path)] || 'application/octet-stream');
    response.setHeader('Cache-Control', 'no-store'); response.end(await readFile(path));
  } catch { response.writeHead(400); response.end('Invalid request'); }
}).listen(5184, '127.0.0.1', () => process.stdout.write(`Local preview: http://127.0.0.1:5184${site.basePath}\n`));
