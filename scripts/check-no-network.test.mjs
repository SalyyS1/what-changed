import assert from "node:assert/strict";

const packagePattern = /^\s*(?:[├└│─\s]+)?([a-zA-Z0-9_-]+)\s+v/;
const lines = [
  "what-changed v0.1.0",
  "├── reqwest v0.12.0",
  "│   └── hyper v1.7.0",
  "└── tauri v2.8.0",
];
assert.equal(lines[0].match(packagePattern)?.[1], "what-changed");
assert.equal(lines[1].match(packagePattern)?.[1], "reqwest");
assert.equal(lines[2].match(packagePattern)?.[1], "hyper");
assert.equal(lines[3].match(packagePattern)?.[1], "tauri");
console.log("check-no-network regex: Unicode cargo tree edges covered");
