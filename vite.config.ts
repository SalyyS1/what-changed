import { defineConfig } from 'vite';
import type { Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, parse, resolve } from 'node:path';

// This private build record is outside frontendDist. It names modules the
// bundler actually read, not every package installed by Bun. The release
// notice gate still treats it as a conservative input set for human review.
function frontendNoticeInputs(): Plugin {
  const moduleIds = new Set<string>();
  const cssPackageIds = new Set<string>();
  const unresolvedCssImports = new Set<string>();
  const packageForModule = (id: string) => {
    const file = id.split('?')[0];
    if (!file || !existsSync(file)) return null;
    let directory = dirname(realpathSync(file));
    while (directory !== parse(directory).root) {
      if (directory.replaceAll('\\', '/').includes('/node_modules/')) {
        const manifest = join(directory, 'package.json');
        if (existsSync(manifest)) {
          const parsed = JSON.parse(readFileSync(manifest, 'utf8')) as { name?: string; version?: string };
          if (parsed.name && parsed.version) return { name: parsed.name, version: parsed.version };
        }
      }
      directory = dirname(directory);
    }
    return null;
  };
  return {
    name: 'what-changed-notice-inputs',
    apply: 'build' as const,
    enforce: 'pre' as const,
    transform(source, id) {
      const importer = id.split('?')[0];
      if (!importer.endsWith('.css') || !existsSync(importer)) return null;
      // CSS processors consume package imports before Rollup records JS module
      // inputs. Capture the package source while the original @import remains.
      for (const match of source.matchAll(/@import\s+(?:url\(\s*)?['"]([^'"]+)['"]/g)) {
        const specifier = match[1];
        if (specifier.startsWith('.') || specifier.startsWith('/')
          || /^[a-z][a-z\d+.-]*:/i.test(specifier)) continue;
        try {
          cssPackageIds.add(createRequire(importer).resolve(specifier));
        } catch {
          unresolvedCssImports.add(`${importer} -> ${specifier}`);
        }
      }
      return null;
    },
    generateBundle(_options, bundle) {
      for (const output of Object.values(bundle)) {
        if (output.type === 'chunk') {
          for (const id of Object.keys(output.modules ?? {})) moduleIds.add(id);
        }
      }
      for (const id of this.getModuleIds()) moduleIds.add(id);
    },
    writeBundle() {
      const packages = new Map<string, { name: string; version: string }>();
      const cssPackages = new Map<string, { name: string; version: string }>();
      let unresolvedNodeModules = 0;
      for (const id of cssPackageIds) {
        const item = packageForModule(id);
        if (item) cssPackages.set(`${item.name}@${item.version}`, item);
      }
      for (const id of new Set([...moduleIds, ...cssPackageIds])) {
        const item = packageForModule(id);
        if (item) packages.set(`${item.name}@${item.version}`, item);
        else if (id.replaceAll('\\', '/').includes('/node_modules/')) unresolvedNodeModules += 1;
      }
      const output = resolve(process.cwd(), 'dist-installer/frontend-inputs.json');
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, `${JSON.stringify({
        schemaVersion: 2,
        packages: [...packages.values()].sort((a, b) =>
          `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`)),
        cssPackages: [...cssPackages.values()].sort((a, b) =>
          `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`)),
        unresolvedNodeModules,
        unresolvedCssImports: unresolvedCssImports.size,
      }, null, 2)}\n`);
    },
  };
}

// Tauri drives this dev server, so the port is fixed and failure is loud:
// a silent port bump would leave `build.devUrl` in tauri.conf.json pointing at nothing.
const DEV_PORT = 5183;

// `TAURI_ENV_*` is injected by the Tauri CLI; `1` marks a debug build.
const isTauriDebug = !!process.env.TAURI_ENV_DEBUG;

export default defineConfig({
  plugins: [frontendNoticeInputs(), svelte(), tailwindcss()],

  // Tauri reads the Rust-side console; Vite must not wipe it.
  clearScreen: false,

  server: {
    port: DEV_PORT,
    strictPort: true,
    // `localhost` (not a bare IP) so it binds both 127.0.0.1 and ::1 — WebView2
    // may resolve the devUrl host to either, and a half-bound server fails silently.
    host: 'localhost',
    // The webview watches the filesystem itself; ignore Rust build output.
    watch: { ignored: ['**/src-tauri/**'] },
  },

  // Only these prefixes reach the frontend bundle. Nothing else from the
  // developer's environment is allowed to leak into a shipped artifact.
  envPrefix: ['VITE_', 'TAURI_ENV_'],

  build: {
    // WebView2 on Windows 10/11 is evergreen Chromium; no legacy transpile needed.
    target: 'esnext',
    // Vite 8 minifies with Oxc; `'esbuild'` would require installing esbuild
    // separately (measured: the build fails with ERR_MODULE_NOT_FOUND without it).
    minify: isTauriDebug ? false : 'oxc',
    sourcemap: isTauriDebug,
    emptyOutDir: true,
  },

  resolve: {
    alias: {
      $lib: new URL('./src/lib', import.meta.url).pathname,
      $components: new URL('./src/components', import.meta.url).pathname,
      $features: new URL('./src/features', import.meta.url).pathname,
      $app: new URL('./src/app', import.meta.url).pathname,
      $three: new URL('./src/three', import.meta.url).pathname,
    },
    // Under Vitest, Svelte's package exports would otherwise resolve to the SSR
    // build, whose `mount` throws `lifecycle_function_unavailable`. The app is a
    // client-only SPA, so the browser condition is the correct one everywhere.
    ...(process.env.VITEST ? { conditions: ['browser'] } : {}),
  },

  test: {
    environment: 'jsdom',
    include: ['src/**/*.{test,spec}.{ts,svelte.ts}'],
    globals: false,
    setupFiles: ['./src/lib/test/vitest-setup.ts'],
  },
});
