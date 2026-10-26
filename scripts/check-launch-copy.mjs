#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const launchDir = path.join(root, "docs", "launch");
const publicCopy = [
  "launch-article-en.md",
  "launch-article-vi.md",
  "community-posts.md",
  "demo-script.md",
  "press-kit.md",
  "release-notes-template.md",
];
const forbidden = [
  "ai",
  "root cause",
  "caused",
  "fixes your pc",
  "real-time everything",
  "zero memory",
  "fully secure",
  "replacement for process monitor",
];
const errors = [];

for (const name of publicCopy) {
  const file = path.join(launchDir, name);
  if (!fs.existsSync(file)) {
    errors.push(`missing launch copy: docs/launch/${name}`);
    continue;
  }
  const text = fs.readFileSync(file, "utf8");
  if (!/^\*\*Evidence state:/m.test(text)) {
    errors.push(`docs/launch/${name}: missing evidence-state label`);
  }
  const lower = text.toLowerCase();
  for (const phrase of forbidden) {
    const found = phrase === "ai" ? /\bai\b/i.test(text) : lower.includes(phrase);
    if (found) {
      errors.push(`docs/launch/${name}: forbidden public claim: ${phrase}`);
    }
  }
}

const messaging = fs.readFileSync(path.join(launchDir, "messaging.md"), "utf8");
if (!/^# Launch messaging/m.test(messaging)) errors.push("docs/launch/messaging.md: missing canonical heading");
if (!/## Anti-claims/m.test(messaging)) errors.push("docs/launch/messaging.md: missing anti-claims section");
const vietnamese = fs.readFileSync(path.join(launchDir, "launch-article-vi.md"), "utf8");
if (!/Đã chạy được|tài liệu tiếng Anh chính thức/i.test(vietnamese)) {
  errors.push("docs/launch/launch-article-vi.md: adapted Vietnamese copy is missing");
}

if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}
console.log(`check-launch-copy: ${publicCopy.length} public files passed anti-claim and evidence-state checks`);
