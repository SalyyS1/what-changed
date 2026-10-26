#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const docsRoot = resolve(root, "docs");
const readme = resolve(root, "README.md");
const wikiIndex = resolve(docsRoot, "wiki-index.md");
const rootAuthorities = [
  "README.md",
  "PRODUCT.md",
  "ARCHITECTURE.md",
  "COLLECTORS.md",
  "ANALYSIS.md",
  "PRIVACY.md",
  "SECURITY.md",
  "ROADMAP.md",
];
const roots = [docsRoot, readme];
const files = [];
function collect(path) {
  if (statSync(path).isDirectory()) {
    for (const child of readdirSync(path)) collect(join(path, child));
  } else if (extname(path).toLowerCase() === ".md") files.push(path);
}
for (const path of roots) collect(path);

const errors = [];
const linkCache = new Map();

function displayPath(path) {
  return relative(root, path).split(sep).join("/");
}

function markdownLinks(file) {
  if (linkCache.has(file)) return linkCache.get(file);
  const content = readFileSync(file, "utf8");
  const links = [];
  let inFence = false;
  let fenceChar = "";
  let fenceLength = 0;
  for (const line of content.split(/\r?\n/)) {
    const fence = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fence) {
      if (!inFence) {
        inFence = true;
        fenceChar = fence[1][0];
        fenceLength = fence[1].length;
      } else if (fence[1][0] === fenceChar && fence[1].length >= fenceLength) {
        inFence = false;
      }
      continue;
    }
    if (inFence) continue;
    for (const match of line.matchAll(/(!?)\[[^\]]*\]\((<[^>]+>|[^)]+)\)/g)) {
      const raw = match[2].trim();
      const destination = raw.startsWith("<") ? raw.slice(1, raw.indexOf(">")) : raw.split(/\s+["']/)[0];
      links.push({ destination, isImage: match[1] === "!" });
    }
  }
  linkCache.set(file, links);
  return links;
}

function resolveLocalLink(file, destination) {
  if (/^(?:[a-z][a-z\d+.-]*:|#)/i.test(destination)) return null;
  const withoutFragment = destination.split(/[?#]/, 1)[0];
  if (!withoutFragment) return null;
  let decoded;
  try {
    decoded = decodeURIComponent(withoutFragment);
  } catch {
    errors.push(`${displayPath(file)}: malformed link ${destination}`);
    return null;
  }
  return resolve(dirname(file), decoded);
}

function isRepositoryPath(path) {
  const fromRoot = relative(root, path);
  return fromRoot !== ".." && !fromRoot.startsWith(`..${sep}`) && !isAbsolute(fromRoot);
}

function isNavigationPage(path) {
  if (!isRepositoryPath(path) || extname(path).toLowerCase() !== ".md") return false;
  const fromRoot = relative(root, path);
  return dirname(fromRoot) === "." || fromRoot.startsWith(`docs${sep}`);
}

function reachableFrom(start, maxHops = Number.POSITIVE_INFINITY) {
  const distances = new Map([[start, 0]]);
  const queue = [start];
  for (let index = 0; index < queue.length; index += 1) {
    const page = queue[index];
    const hops = distances.get(page);
    if (hops >= maxHops || !existsSync(page)) continue;
    for (const { destination, isImage } of markdownLinks(page)) {
      if (isImage) continue;
      const target = resolveLocalLink(page, destination);
      if (!target || !isNavigationPage(target) || !existsSync(target) || distances.has(target)) continue;
      distances.set(target, hops + 1);
      queue.push(target);
    }
  }
  return distances;
}

const packageJson = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
const cargoToml = readFileSync(resolve(root, "src-tauri/Cargo.toml"), "utf8");
const tauriConfig = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
if (packageJson.license !== "MIT") errors.push("package.json: license must be MIT");
if (!/^license\s*=\s*[\"']MIT[\"']/m.test(cargoToml)) errors.push("src-tauri/Cargo.toml: license must be MIT");
if (tauriConfig.bundle?.license !== "MIT") errors.push("src-tauri/tauri.conf.json: bundle.license must be MIT");
if (tauriConfig.bundle?.licenseFile !== "../LICENSE") errors.push("src-tauri/tauri.conf.json: bundle.licenseFile must point to LICENSE");
if (!existsSync(resolve(root, "LICENSE"))) errors.push("LICENSE: canonical MIT file is missing");
for (const file of files) {
  const text = readFileSync(file, "utf8");
  const isJournal = file.startsWith(`${join(docsRoot, "journals")}${sep}`);
  if (!isJournal && !/^\*\*Evidence state:/m.test(text)) {
    errors.push(`${displayPath(file)}: missing evidence-state label`);
  }
  if (file !== wikiIndex && !/^#\s+/m.test(text)) {
    errors.push(`${displayPath(file)}: missing a level-one heading`);
  }
  for (const { destination } of markdownLinks(file)) {
    const target = resolveLocalLink(file, destination);
    if (target && (!isRepositoryPath(target) || !existsSync(target))) {
      errors.push(`${displayPath(file)}: broken or non-repository link ${destination}`);
    }
  }
}

const fromReadme = reachableFrom(readme, 2);
const fromWiki = reachableFrom(wikiIndex, 2);
for (const authority of rootAuthorities) {
  const page = resolve(root, authority);
  if (!fromReadme.has(page)) errors.push(`README.md: root authority ${authority} is not reachable within 2 Markdown links`);
  if (!fromWiki.has(page)) errors.push(`docs/wiki-index.md: root authority ${authority} is not reachable within 2 Markdown links`);
}
if (fromReadme.get(wikiIndex) !== 1) errors.push("README.md: docs/wiki-index.md must be linked directly as the canonical guide index");

const indexedGuides = reachableFrom(wikiIndex);
for (const file of files) {
  if (!file.startsWith(`${docsRoot}${sep}`) || file === wikiIndex) continue;
  const fromDocs = relative(docsRoot, file).split(sep);
  if (["journals", "launch", "reports"].includes(fromDocs[0])) continue;
  if (!indexedGuides.has(file)) errors.push(`${displayPath(file)}: orphaned guide (not reachable from docs/wiki-index.md)`);
}

if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}
console.log(`check-docs: ${files.length} markdown files and relative links passed`);
