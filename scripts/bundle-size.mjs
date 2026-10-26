#!/usr/bin/env node
/**
 * Report and gate compressed JavaScript output.
 *
 * Vite's manifest is optional in this repository, so the script works on a
 * plain `dist/assets` directory as well.  The `--entry` option identifies the
 * initial shell chunk; without it, all non-Three chunks are reported and the
 * largest one is used for the conservative gate.  Lazy Three.js chunks are
 * listed separately and never counted toward the 400 KB shell budget.
 */

import { gzipSync } from 'node:zlib';
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';

function parseArgs(argv) {
  const options = { dist: 'dist', maxKb: 400, entry: null, json: null };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--dist') options.dist = argv[++i] ?? options.dist;
    else if (arg === '--max-kb') options.maxKb = Number(argv[++i] ?? options.maxKb);
    else if (arg === '--entry') options.entry = argv[++i] ?? null;
    else if (arg === '--json') options.json = argv[++i] ?? null;
    else if (arg === '--help' || arg === '-h') {
      console.log('usage: bundle-size.mjs [--dist dist] [--max-kb 400] [--entry assets/index.js] [--json path]');
      process.exit(0);
    } else {
      throw new Error(`unknown argument ${arg}`);
    }
  }
  if (!Number.isFinite(options.maxKb) || options.maxKb <= 0) throw new Error('--max-kb must be positive');
  return options;
}

function jsFiles(root) {
  const assets = join(root, 'assets');
  if (!statSafe(assets)?.isDirectory()) throw new Error(`build output is missing: ${assets}`);
  return readdirSync(assets)
    .filter((name) => name.endsWith('.js'))
    .map((name) => join(assets, name))
    .sort();
}

function statSafe(path) {
  try { return statSync(path); } catch { return null; }
}

function isThreeChunk(file, source) {
  const name = basename(file).toLowerCase();
  return name.includes('three') || /from\s*["']three["']|three\/src|THREE\./.test(source);
}

function record(file) {
  const bytes = readFileSync(file);
  const source = bytes.toString('utf8');
  return {
    file: file.replaceAll('\\', '/'),
    bytes: bytes.byteLength,
    gzipBytes: gzipSync(bytes, { level: 9 }).byteLength,
    three: isThreeChunk(file, source),
  };
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const root = resolve(options.dist);
  const files = jsFiles(root).map(record);
  const shell = files.filter((file) => !file.three);
  const three = files.filter((file) => file.three);
  const entry = options.entry
    ? shell.find((file) => file.file.endsWith(options.entry.replaceAll('\\', '/')))
    : null;
  const shellBytes = entry ? entry.gzipBytes : shell.reduce((sum, file) => sum + file.gzipBytes, 0);
  const result = {
    maxGzipBytes: Math.round(options.maxKb * 1024),
    measuredGzipBytes: shellBytes,
    countedAs: entry?.file ?? 'all non-Three chunks',
    shell,
    lazyThreeChunks: three,
    threeExcluded: true,
    pass: shellBytes <= options.maxKb * 1024,
  };
  console.log(JSON.stringify(result, null, 2));
  if (options.json) writeFileSync(resolve(options.json), `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  if (!result.pass) {
    throw new Error(`initial JavaScript exceeds ${options.maxKb} KB gzip (${(shellBytes / 1024).toFixed(1)} KB)`);
  }
}

try { main(); } catch (error) {
  console.error(`bundle-size: ${error instanceof Error ? error.message : 'check failed'}`);
  process.exitCode = 1;
}

