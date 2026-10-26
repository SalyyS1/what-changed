import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const checker = fileURLToPath(new URL("./check-docs.mjs", import.meta.url));
const authorities = ["PRODUCT.md", "ARCHITECTURE.md", "COLLECTORS.md", "ANALYSIS.md", "PRIVACY.md", "SECURITY.md", "ROADMAP.md"];

function write(root, path, content) {
  const target = join(root, path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, content);
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "what-changed-docs-check-"));
  write(root, "package.json", JSON.stringify({ license: "MIT" }));
  write(root, "src-tauri/Cargo.toml", 'license = "MIT"\n');
  write(root, "src-tauri/tauri.conf.json", JSON.stringify({ bundle: { license: "MIT", licenseFile: "../LICENSE" } }));
  write(root, "LICENSE", "MIT License\n");
  for (const authority of authorities) write(root, authority, `# ${authority}\n`);
  write(root, "README.md", `# Readme\n\n**Evidence state: available.**\n\n${authorities.map((name) => `[${name}](${name})`).join("\n")}\n[Docs](docs/wiki-index.md)\n`);
  write(root, "docs/wiki-index.md", `# Wiki\n\n**Evidence state: available.**\n\n[Readme](../README.md)\n${authorities.map((name) => `[${name}](../${name})`).join("\n")}\n[Guide](guide.md)\n`);
  write(root, "docs/guide.md", "# Guide\n\n**Evidence state: available.**\n");
  mkdirSync(join(root, "scripts"));
  copyFileSync(checker, join(root, "scripts/check-docs.mjs"));
  return root;
}

function check(root) {
  return spawnSync(process.execPath, [join(root, "scripts/check-docs.mjs")], { encoding: "utf8" });
}

test("accepts direct and two-hop root routes and indexed guides", () => {
  const root = fixture();
  try {
    write(root, "docs/wiki-index.md", `# Wiki\n\n**Evidence state: available.**\n\n[Readme](../README.md)\n[Architecture](../ARCHITECTURE.md)\n${authorities.filter((name) => name !== "PRODUCT.md" && name !== "ARCHITECTURE.md").map((name) => `[${name}](../${name})`).join("\n")}\n[Guide](guide.md)\n`);
    write(root, "ARCHITECTURE.md", "# Architecture\n\n[Product](PRODUCT.md)\n");
    const result = check(root);
    assert.equal(result.status, 0, result.stderr);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("rejects missing authority routes, broken links and orphaned guides", () => {
  const root = fixture();
  try {
    write(root, "docs/wiki-index.md", "# Wiki\n\n**Evidence state: available.**\n\n[Architecture](../ARCHITECTURE.md)\n[Guide](guide.md)\n");
    write(root, "docs/orphan.md", "# Orphan\n\n**Evidence state: available.**\n\n[Broken](missing.md)\n");
    const result = check(root);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /root authority PRODUCT\.md is not reachable within 2 Markdown links/);
    assert.match(result.stderr, /docs\/orphan\.md: orphaned guide/);
    assert.match(result.stderr, /docs\/orphan\.md: broken or non-repository link missing\.md/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("ignores journal and launch files for orphan detection", () => {
  const root = fixture();
  try {
    write(root, "docs/journals/history.md", "# History\n");
    write(root, "docs/launch/ad-copy.md", "# Ad copy\n\n**Evidence state: available.**\n");
    const result = check(root);
    assert.equal(result.status, 0, result.stderr);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
