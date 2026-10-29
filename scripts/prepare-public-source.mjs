import { copyFileSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const root = fileURLToPath(new URL('..', import.meta.url));
const roots = ['.github', 'src', 'src-tauri', 'scripts', 'assets', 'public', 'legal', 'docs', 'wiki-site', 'runner', 'release', 'plans'];
const rootFiles = ['.gitattributes', '.gitignore', 'ANALYSIS.md', 'ARCHITECTURE.md', 'bun.lock', 'CODE_OF_CONDUCT.md', 'COLLECTORS.md', 'CONTRIBUTING.md', 'index.html', 'LICENSE', 'package.json', 'PRIVACY.md', 'PRODUCT.md', 'README.md', 'ROADMAP.md', 'rust-toolchain.toml', 'SECURITY.md', 'SUPPORT.md', 'svelte.config.js', 'THIRD_PARTY_NOTICES.md', 'tsconfig.json', 'tsconfig.node.json', 'vite.config.ts'];
const excludedDirectories = /^(?:target|node_modules|dist|dist-installer|\.git|\.qa|captures|screenshots|gen|bundle|bundle-stage-.+|bundle-previous-.+)$/;
const excludedFiles = /(?:^\.env(?:\..*)?$|\.(?:db|sqlite|sqlite3|db-wal|db-shm|log|pem|pfx|p12|key|zip|exe|dll|lib|pdb)$)/i;
const secretPatterns = [/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/, /\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b/, /\bAKIA[0-9A-Z]{16}\b/, /\bsk-(?:proj-)?[A-Za-z0-9_-]{40,}\b/];
const reviewedSyntheticSources = new Map([
  ['src-tauri/tests/fixtures/secrets.rs', '01a9e9243ac13aae5c1ecbf705c29fdf6591168ef3852aa46d54dc88755659d6'],
]);
const publicReviewReports = new Set(['plans/reports/research-261028-1638-browser-redistribution.md', 'plans/reports/research-261028-1703-redistributable-chromium.md']);

// Public source uses a fresh clone's history. Local QA and repository objects are
// never candidates, and files already published but absent locally are preserved.
export function preparePublicSource(destination) {
  const output = resolve(destination);
  if (!output.startsWith(resolve(root, '.qa') + '/') && !output.startsWith(resolve(root, '.qa') + '\\')) throw new Error('Projection must be an isolated QA clone');
  if (!existsSync(join(output, '.git'))) throw new Error('Projection destination must be an existing clone');
  const files = [];
  function copy(name) {
    const source = join(root, name); const stat = lstatSync(source);
    if (stat.isSymbolicLink()) throw new Error(`Redirected public source: ${name}`);
    if (stat.isDirectory()) {
      if (excludedDirectories.test(name.split('/').at(-1))) return;
      for (const child of readdirSync(source).sort()) copy(`${name}/${child}`);
      return;
    }
    if (!stat.isFile() || excludedFiles.test(name.split('/').at(-1))) return;
    if (name.startsWith('plans/') && !/\.(?:md|json|txt)$/.test(name)) return;
    // Internal candidate/advice transcripts are not public build inputs.
    if (name.startsWith('plans/') && /(?:planner|brainstorm|advice|scout|research|candidate-\d)/i.test(name) && !publicReviewReports.has(name)) return;
    if (name === 'scripts/third-party-notices-manifest.json') return; // preserve published review records
    const target = join(output, name); mkdirSync(dirname(target), { recursive: true });
    const textual = /\.(?:md|json|txt|toml|rs|ts|js|mjs|css|html|svelte|yml|yaml|ps1|svg|nsh)$/.test(name) || ['LICENSE', '.gitignore', '.gitattributes'].includes(name);
    if (textual) {
      let content = readFileSync(source, 'utf8');
      if (secretPatterns.some((pattern) => pattern.test(content))) {
        let alreadyPublic = false;
        try { alreadyPublic = execFileSync('git', ['show', `HEAD:${name}`], { cwd: output, stdio: ['ignore', 'pipe', 'ignore'] }).toString('utf8').replaceAll('\r\n', '\n') === content.replaceAll('\r\n', '\n'); } catch { /* New source requires review. */ }
        const sourceHash = createHash('sha256').update(content).digest('hex');
        if (!alreadyPublic && reviewedSyntheticSources.get(name) !== sourceHash) throw new Error(`Possible credential material requires review: ${name}; sha256=${sourceHash}`);
      }
      if (/^(?:plans|docs|legal|release)\//.test(name)) {
        content = content.replaceAll('Salyyy', 'project-maintainer')
          .replaceAll('C:\\Users\\project-maintainer', 'C:\\Users\\<user>')
          .replaceAll('C:/Users/project-maintainer', 'C:/Users/<user>')
          .replaceAll('C:\\\\Users\\\\project-maintainer', 'C:\\\\Users\\\\<user>');
      }
      if (publicReviewReports.has(name)) {
        content = content.replace(/\[([^\]]+)\]\((?:\.\.\/)+\.qa\/[^)]+\)/g, '$1 (local QA evidence; not published)');
      }
      if (name === 'release/runner-runtime-inventory.json' || name === 'release/RUNNER_THIRD_PARTY_NOTICES.txt' || name.startsWith('legal/third-party/')) {
        copyFileSync(source, target);
        files.push({ path: relative(output, target).replaceAll('\\', '/'), bytes: stat.size });
        return;
      }
      writeFileSync(target, content);
    } else copyFileSync(source, target);
    files.push({ path: relative(output, target).replaceAll('\\', '/'), bytes: stat.size });
  }
  for (const name of [...rootFiles, ...roots]) if (existsSync(join(root, name))) copy(name);
  return { copiedFiles: files.length, destination: output, privateInputsIncluded: false };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { if (process.argv.length !== 3) throw new Error('Usage: node scripts/prepare-public-source.mjs <isolated-clone>'); console.log(JSON.stringify(preparePublicSource(process.argv[2]))); }
  catch (error) { console.error(`Public projection blocked: ${error.message}`); process.exitCode = 1; }
}
