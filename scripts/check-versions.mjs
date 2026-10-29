#!/usr/bin/env node
/**
 * Check the version sources that are intentionally kept in sync.
 *
 * Cargo.toml is the release source of truth (the Tauri config deliberately
 * omits `version`).  Keeping this as a dependency-free Node script means it
 * can run before Bun installs or a Rust build starts.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const packageJson = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
const cargo = readFileSync(resolve(root, "src-tauri/Cargo.toml"), "utf8");
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

if (!cargoVersion) {
  console.error("check-versions: src-tauri/Cargo.toml has no package version");
  process.exit(1);
}

const sources = {
  "package.json": packageJson.version,
  "src-tauri/Cargo.toml": cargoVersion,
};
const mismatches = Object.entries(sources).filter(([, version]) => version !== cargoVersion);
if (mismatches.length > 0) {
  console.error("check-versions: version drift detected:");
  for (const [source, version] of Object.entries(sources)) {
    console.error(`  ${source}: ${version ?? "<missing>"}`);
  }
  process.exit(1);
}

console.log(`check-versions: ${cargoVersion} (package + Cargo agree)`);
