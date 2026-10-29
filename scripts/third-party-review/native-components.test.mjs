import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { TARGET, sha256 } from './filesystem.mjs';
import { fixture } from './test-fixture.mjs';

const PACKAGE = '.cargo/registry/src/index/webview2-com-sys-0.38.2';
const EVIDENCE = {
  'x64/WebView2Loader.dll': Buffer.from([0x4d, 0x5a, 0x00, 0xff, 0x10]),
  'x64/WebView2Loader.dll.lib': Buffer.from([0x21, 0x3c, 0x61, 0x72, 0x63, 0x68, 0x3e]),
  'x64/WebView2LoaderStatic.lib': Buffer.from([0x21, 0x3c, 0x61, 0x72, 0x63, 0x68, 0x3e, 0xff]),
  'build.rs': Buffer.from('fn main() { println!("cargo:rustc-link-search=native=x64"); }\r\n'),
  'src/lib.rs': Buffer.from('#[cfg_attr(target_env = "msvc", link(name = "WebView2LoaderStatic", kind = "static"))]\nextern "C" {}\n'),
};
const MIT = `MIT License

Copyright (c) 2026 fixture authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
`;

function webviewFixture(t) {
  const item = fixture(t);
  item.metadata.packages.push({ name: 'webview2-com-sys', version: '0.38.2', license: 'MIT',
    source: 'registry+https://github.com/rust-lang/crates.io-index',
    manifest_path: join(item.root, PACKAGE, 'Cargo.toml') });
  item.write(`${PACKAGE}/Cargo.toml`, '[package]\nname="webview2-com-sys"\nversion="0.38.2"\nlicense="MIT"\n');
  item.write(`${PACKAGE}/LICENSE`, MIT);
  for (const [file, bytes] of Object.entries(EVIDENCE)) item.write(`${PACKAGE}/${file}`, bytes);
  item.write('cargo-tree.txt', `${readFileSync(item.options.cargoTree, 'utf8')}webview2-com-sys v0.38.2\n`);
  item.saveInputs();
  return item;
}

test('WebView2 hashes exact native and selection bytes, preserving pending review and deterministic drafts', (t) => {
  const item = webviewFixture(t);
  const manifestBefore = readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json'));
  const result = item.run();
  const draftBytes = readFileSync(result.output);
  const draft = JSON.parse(draftBytes);
  const entry = draft.entries.find((candidate) => candidate.name === 'webview2-com-sys');
  assert.equal(entry.version, '0.38.2');
  assert.equal(entry.licenseExpression, 'MIT');
  assert.equal(entry.licenseFiles.length, 1);
  assert.equal(entry.licenseFiles[0].sha256, sha256(Buffer.from(MIT)));
  assert.deepEqual(entry.issues, ['native-component-license-review-required']);
  assert.equal(entry.nativeComponents.length, 1);
  const component = entry.nativeComponents[0];
  assert.equal(component.name, 'WebView2Loader');
  assert.equal(component.target, TARGET);
  assert.equal(component.version, null);
  assert.equal(component.licenseExpression, null);
  assert.equal(component.obligationsReviewed, false);
  assert.match(component.evidenceScope, /candidates.*not final binary link proof/);
  assert.deepEqual(component.nativeFiles.map((record) => record.sourceFile), Object.keys(EVIDENCE).slice(0, 3));
  assert.deepEqual(component.selectionEvidence.map((record) => record.sourceFile), ['build.rs', 'src/lib.rs']);
  for (const record of [...component.nativeFiles, ...component.selectionEvidence]) {
    const original = EVIDENCE[record.sourceFile];
    assert.equal(record.sourceSha256, sha256(original));
    assert.equal(record.sourceBytes, original.length);
    assert.equal(existsSync(join(item.root, `legal/third-party/cargo/webview2-com-sys/0.38.2/${record.sourceFile}`)), false);
  }
  assert.equal(draft.status, 'pending');
  assert.equal(draft.publishable, false);
  assert.equal(draft.obligationsReviewed, false);
  assert.equal(draft.reviewedBy, null);
  assert.equal(entry.obligationsReviewed, false);
  assert.equal(entry.selectedLicense, null);
  assert.deepEqual(readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json')), manifestBefore);
  assert.equal(existsSync(join(item.root, 'dist-installer/THIRD_PARTY_NOTICES.txt')), false);
  assert.equal(item.run().sha256, result.sha256);
  assert.deepEqual(readFileSync(result.output), draftBytes);
});

for (const file of Object.keys(EVIDENCE)) {
  for (const condition of ['missing', 'empty']) {
    test(`${condition} required WebView2 evidence ${file} fails before any output write`, (t) => {
      const item = webviewFixture(t);
      const manifestBefore = readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json'));
      const path = join(item.root, PACKAGE, file);
      if (condition === 'missing') rmSync(path);
      else item.write(`${PACKAGE}/${file}`, Buffer.alloc(0));
      assert.throws(item.run, condition === 'missing' ? /ENOENT/ : /required WebView2Loader evidence file is empty/);
      assert.equal(existsSync(join(item.root, 'legal')), false);
      assert.equal(existsSync(join(item.root, 'dist-installer/THIRD_PARTY_NOTICES.txt')), false);
      assert.deepEqual(readFileSync(join(item.root, 'scripts/third-party-notices-manifest.json')), manifestBefore);
    });
  }
}

test('WebView2 native candidate through an escaping directory junction fails before output writes', (t) => {
  const item = webviewFixture(t);
  const outside = mkdtempSync(join(tmpdir(), 'what-changed-webview-outside-'));
  t.after(() => rmSync(outside, { recursive: true, force: true }));
  writeFileSync(join(outside, 'WebView2Loader.dll'), EVIDENCE['x64/WebView2Loader.dll']);
  const directory = join(item.root, PACKAGE, 'x64');
  rmSync(directory, { recursive: true });
  symlinkSync(outside, directory, 'junction');
  assert.throws(item.run, /package source file escapes its package directory/);
  assert.equal(existsSync(join(item.root, 'legal')), false);
  assert.equal(existsSync(join(item.root, 'dist-installer/THIRD_PARTY_NOTICES.txt')), false);
});
