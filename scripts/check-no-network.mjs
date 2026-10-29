#!/usr/bin/env node
/**
 * Resolve the normal Windows dependency graph and reject common HTTP clients.
 * Do not grep Cargo.lock: Tauri's non-Windows dependency branches may contain
 * those names without linking them into the Windows binary.
 */
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const root = fileURLToPath(new URL("..", import.meta.url));
const cargoArgs = [
  "tree",
  "--manifest-path",
  resolve(root, "src-tauri/Cargo.toml"),
  "--target",
  "x86_64-pc-windows-msvc",
  "--edges",
  "normal",
];
let tree;
const provided = process.env.WHAT_CHANGED_CARGO_TREE_FILE;
if (provided) {
  tree = readFileSync(provided, "utf8");
} else {
  const result = spawnSync("cargo", cargoArgs, {
    cwd: resolve(root, "src-tauri"),
    encoding: "utf8",
  });
  if (result.error || result.status !== 0) {
    console.error("check-no-network: cargo tree could not resolve the Windows graph");
    if (result.error) console.error(result.error.message);
    if (result.stderr) console.error(result.stderr.trim());
    process.exit(result.status || 1);
  }
  tree = result.stdout;
}
if (!tree) {
  console.error("check-no-network: cargo tree could not resolve the Windows graph");
  process.exit(1);
}

const forbidden = new Set(["reqwest", "hyper", "ureq", "isahc"]);
const found = new Set();
for (const line of tree.split(/\r?\n/)) {
  // `cargo tree` uses Unicode branch glyphs on Windows (for example
  // `├── reqwest v...` and nested `│   └── hyper v...`). Strip that visual
  // prefix before reading the package name; an ASCII-only edge regex silently
  // turns this release gate into a false pass.
  const match = line.match(/^\s*(?:[├└│─\s]+)?([a-zA-Z0-9_-]+)\s+v/);
  if (match && forbidden.has(match[1])) found.add(match[1]);
}
if (found.size > 0) {
  console.error(`check-no-network: forbidden Windows HTTP clients: ${[...found].sort().join(", ")}`);
  process.exit(1);
}
console.log("check-no-network: Windows normal dependency graph contains no supported HTTP client");
