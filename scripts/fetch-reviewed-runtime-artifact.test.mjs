import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { gzipSync, gunzipSync } from 'node:zlib';
import { execFileSync } from 'node:child_process';
import { downloadVerifiedArtifact, expandReviewedRuntimeArchive } from './fetch-reviewed-runtime-artifact.mjs';
import { prepareRunnerSourceMaterials } from './prepare-runner-source-materials.mjs';

const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const data = Buffer.from('reviewed runtime bytes');
const record = { url: 'https://example.invalid/browser.zip?generation=123', bytes: data.length, sha256: digest(data) };
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'what-changed-reviewed-runtime-'));
  t.after(async () => {
    assert.equal(dirname(root), resolve(tmpdir()));
    assert.ok(basename(root).startsWith('what-changed-reviewed-runtime-'));
    await rm(root, { recursive: true, force: true });
  });
  return { root, output: join(root, 'artifact.zip') };
}
const response = (chunks) => ({ status: 200, ok: true, body: (async function* () { yield* chunks; })() });
const absent = (path) => assert.rejects(stat(path), { code: 'ENOENT' });

test('streams reviewed bytes and preserves the source generation query', async (t) => {
  const { output } = await fixture(t);
  const result = await downloadVerifiedArtifact(record, output, { fetchImpl: async (url) => {
    assert.equal(url, record.url);
    return response([data.subarray(0, 4), data.subarray(4)]);
  } });
  assert.deepEqual(await readFile(output), data);
  assert.equal(result.sha256, record.sha256);
});
for (const [name, review, fetchImpl, pattern] of [
  ['short download', record, async () => response([data.subarray(1)]), /byte count differs/],
  ['hash mismatch', { ...record, sha256: '0'.repeat(64) }, async () => response([data]), /SHA-256 differs/],
  ['overflow', { ...record, bytes: 1 }, async () => response([data]), /exceeds/],
  ['network failure', record, async () => { throw new Error('network failure'); }, /network failure/],
  ['partial stream failure', record, async () => ({ status: 200, ok: true, body: (async function* () { yield data.subarray(0, 2); throw new Error('stream failure'); })() }), /stream failure/],
  ['HTTP source', { ...record, url: 'http://example.invalid/browser.zip' }, async () => response([data]), /HTTPS/],
  ['HTTP redirect', record, async () => ({ status: 302, headers: new Headers({ location: 'http://example.invalid/a' }) }), /HTTPS/],
]) test(`${name} rejects and removes only its own partial output`, async (t) => {
  const { output } = await fixture(t);
  await assert.rejects(downloadVerifiedArtifact(review, output, { fetchImpl }), pattern);
  await absent(output);
});
test('preserves a preexisting output and does not start its download', async (t) => {
  const { output } = await fixture(t);
  await writeFile(output, 'existing');
  await assert.rejects(downloadVerifiedArtifact(record, output, { fetchImpl: () => assert.fail('must not fetch') }), { code: 'EEXIST' });
  assert.equal(await readFile(output, 'utf8'), 'existing');
});
test('follows only HTTPS redirects', async (t) => {
  const { output } = await fixture(t);
  let count = 0;
  await downloadVerifiedArtifact(record, output, { fetchImpl: async (url, options) => {
    assert.equal(options.redirect, 'manual');
    count += 1;
    if (count === 1) return { status: 302, headers: new Headers({ location: 'https://cdn.example.invalid/archive?generation=123' }) };
    assert.equal(url, 'https://cdn.example.invalid/archive?generation=123');
    return response([data]);
  } });
  assert.equal(count, 2);
});
for (const format of ['tar.gz', ...(process.platform === 'win32' ? ['zip'] : [])]) test(`source packet cohosts verified ${format} members, descriptor, checksums and receipt`, async (t) => {
  const { root } = await fixture(t);
  const bytes = format === 'zip' ? zipBytes([{ name: 'source/file.txt', data: 'safe' }]) : sourceTarBytes();
  const source = { ...record, name: `source.${format}`, bytes: bytes.length, sha256: digest(bytes), requiredMembers: ['source/file.txt'] };
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1, sourceMaterials: [source] }));
  const result = await prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([bytes]) });
  assert.deepEqual(await readFile(join(result.output, source.name)), bytes);
  const material = JSON.parse(await readFile(join(result.output, 'source-materials-receipt.json'))).sourceMaterials[0];
  assert.equal(material.sha256, source.sha256);
  assert.equal(material.archiveEntryCount, 1);
  assert.deepEqual(material.matchedMembers, source.requiredMembers);
  assert.ok((await readFile(join(result.output, 'SHA256SUMS.txt'), 'utf8')).includes(source.name));
  await assert.rejects(prepareRunnerSourceMaterials({ root }), { code: 'EEXIST' });
});
test('source packet rejects unsafe and duplicate names before creating output', async (t) => {
  const { root } = await fixture(t);
  await mkdir(join(root, 'release'));
  for (const names of [['../source'], ['CON.txt'], ['source.tar.xz', 'SOURCE.tar.xz'], ['SHA256SUMS.txt']]) {
    await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1,
      sourceMaterials: names.map((name) => ({ ...record, name, requiredMembers: ['source/file.txt'] })) }));
    await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: () => assert.fail('must not fetch') }), /unsafe or duplicate/);
    await absent(join(root, 'preview-artifacts'));
  }
});
test('source download failure leaves no receipt or partial source file', async (t) => {
  const { root } = await fixture(t);
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1, sourceMaterials: [{ ...record, name: 'source.tar.xz', requiredMembers: ['source/file.txt'] }] }));
  await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([Buffer.from('wrong')]) }), /byte count differs/);
  await absent(join(root, 'preview-artifacts/runtime-sources/source.tar.xz'));
  await absent(join(root, 'preview-artifacts/runtime-sources/source-materials-receipt.json'));
});
for (const format of ['tar.gz', ...(process.platform === 'win32' ? ['zip'] : [])]) test(`missing required ${format} member blocks the source receipt`, async (t) => {
  const { root } = await fixture(t);
  const bytes = format === 'zip' ? zipBytes([{ name: 'source/file.txt', data: 'safe' }]) : sourceTarBytes();
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1, sourceMaterials: [{ ...record,
    name: `source.${format}`, bytes: bytes.length, sha256: digest(bytes), requiredMembers: ['source/missing.h'] }] }));
  await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([bytes]) }), /missing required members/);
  await absent(join(root, 'preview-artifacts/runtime-sources/source-materials-receipt.json'));
  await absent(join(root, 'preview-artifacts/runtime-sources/SHA256SUMS.txt'));
});
test('source packet requires safe nonempty relative members before downloading', async (t) => {
  const { root } = await fixture(t);
  await mkdir(join(root, 'release'));
  for (const requiredMembers of [undefined, [], ['../escape'], ['/absolute'], ['source//file'], ['source\\file'], ['source/./file']]) {
    await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1,
      sourceMaterials: [{ ...record, name: 'source.tar.gz', requiredMembers }] }));
    await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: () => assert.fail('must not fetch') }), /nonempty safe relative/);
    await absent(join(root, 'preview-artifacts'));
  }
});
test('source archive listing failure blocks the source receipt', async (t) => {
  const { root } = await fixture(t);
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1,
    sourceMaterials: [{ ...record, name: 'source.tar.gz', requiredMembers: ['source/file.txt'] }] }));
  await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([data]) }), /archive listing failed for source\.tar\.gz/);
  await absent(join(root, 'preview-artifacts/runtime-sources/source-materials-receipt.json'));
});

for (const format of ['tar.xz', 'tar.gz', 'zip']) test(`Python source inspection preserves every Unicode/PAX ${format} member`, async (t) => {
  const { root } = await fixture(t);
  const requiredMembers = ['source/LICENSE', 'source/日本語-𐐀.txt', 'source/final-config.gn'];
  const bytes = unicodeSourceBytes(format, requiredMembers);
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1, sourceMaterials: [{ ...record,
    name: `source.${format}`, bytes: bytes.length, sha256: digest(bytes), requiredMembers }] }));
  const result = await prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([bytes]) });
  const material = JSON.parse(await readFile(join(result.output, 'source-materials-receipt.json'))).sourceMaterials[0];
  assert.equal(material.archiveEntryCount, 3);
  assert.deepEqual(material.matchedMembers, requiredMembers);
});
for (const format of ['tar.xz', 'tar.gz', 'zip']) test(`corrupt ${format} metadata blocks the source packet after a matching download hash`, async (t) => {
  const { root } = await fixture(t);
  const original = unicodeSourceBytes(format, ['source/file.txt']);
  const bytes = original.subarray(0, Math.floor(original.length / 2));
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1, sourceMaterials: [{ ...record,
    name: `source.${format}`, bytes: bytes.length, sha256: digest(bytes), requiredMembers: ['source/file.txt'] }] }));
  await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([bytes]) }), new RegExp(`archive listing failed for source\\.${format.replaceAll('.', '\\.')}`));
  await absent(join(root, 'preview-artifacts/runtime-sources/source-materials-receipt.json'));
});
test('a corrupt later tar header cannot be hidden by finding every required member earlier', async (t) => {
  const { root } = await fixture(t);
  const tar = gunzipSync(unicodeSourceBytes('tar.gz', ['source/file.txt', 'source/final.txt']));
  tar[1024] ^= 1; // Invalidate the second header checksum after one valid file.
  const bytes = gzipSync(tar);
  await mkdir(join(root, 'release'));
  await writeFile(join(root, 'release/runner-browser-source.json'), JSON.stringify({ schemaVersion: 1, sourceMaterials: [{ ...record,
    name: 'source.tar.gz', bytes: bytes.length, sha256: digest(bytes), requiredMembers: ['source/file.txt'] }] }));
  await assert.rejects(prepareRunnerSourceMaterials({ root, fetchImpl: async () => response([bytes]) }), /archive listing failed for source\.tar\.gz.*bad checksum/s);
  await absent(join(root, 'preview-artifacts/runtime-sources/source-materials-receipt.json'));
});
function unicodeSourceBytes(format, members) {
  const program = `import io,json,sys,tarfile,zipfile\nformat,names=json.loads(sys.argv[1])\noutput=io.BytesIO()\nif format=='zip':\n with zipfile.ZipFile(output,'w') as archive:\n  for name in names: archive.writestr(name,b'source')\nelse:\n with tarfile.open(fileobj=output,mode='w:'+('xz' if format=='tar.xz' else 'gz'),format=tarfile.PAX_FORMAT) as archive:\n  for name in names:\n   info=tarfile.TarInfo(name); info.size=6; archive.addfile(info,io.BytesIO(b'source'))\nsys.stdout.buffer.write(output.getvalue())`;
  return execFileSync(process.platform === 'win32' ? 'python' : 'python3', ['-I', '-c', program, JSON.stringify([format, members])], { windowsHide: true, maxBuffer: 1024 * 1024 });
}

function sourceTarBytes() {
  const header = Buffer.alloc(512), bytes = Buffer.from('safe');
  header.write('source/file.txt'); header.write('0000644\0', 100); header.write('0000000\0', 108); header.write('0000000\0', 116);
  header.write('00000000004\0', 124); header.write('00000000000\0', 136); header.fill(32, 148, 156);
  header.write('0', 156); header.write('ustar\0', 257); header.write('00', 263);
  header.write([...header].reduce((sum, byte) => sum + byte, 0).toString(8).padStart(6, '0') + '\0 ', 148);
  return gzipSync(Buffer.concat([header, bytes, Buffer.alloc(512 - bytes.length + 1024)]));
}

// Stored ZIP fixtures exercise the real Windows extractor without external archives.
function zipBytes(entries) {
  const local = [], central = [];
  let offset = 0;
  for (const entry of entries) {
    const name = Buffer.from(entry.name), bytes = Buffer.from(entry.data ?? '');
    let crc = 0xffffffff;
    for (const byte of bytes) { crc ^= byte; for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0); }
    crc = (crc ^ 0xffffffff) >>> 0;
    const header = Buffer.alloc(30), directory = Buffer.alloc(46);
    header.writeUInt32LE(0x04034b50); header.writeUInt16LE(20, 4); header.writeUInt32LE(crc, 14);
    header.writeUInt32LE(bytes.length, 18); header.writeUInt32LE(entry.length ?? bytes.length, 22); header.writeUInt16LE(name.length, 26);
    directory.writeUInt32LE(0x02014b50); directory.writeUInt16LE(0x0314, 4); directory.writeUInt16LE(20, 6);
    directory.writeUInt32LE(crc, 16); directory.writeUInt32LE(bytes.length, 20); directory.writeUInt32LE(entry.length ?? bytes.length, 24);
    directory.writeUInt16LE(name.length, 28); directory.writeUInt32LE((entry.attributes ?? 0) >>> 0, 38); directory.writeUInt32LE(offset, 42);
    local.push(header, name, bytes); central.push(directory, name); offset += header.length + name.length + bytes.length;
  }
  const directoryBytes = Buffer.concat(central), footer = Buffer.alloc(22);
  footer.writeUInt32LE(0x06054b50); footer.writeUInt16LE(entries.length, 8); footer.writeUInt16LE(entries.length, 10);
  footer.writeUInt32LE(directoryBytes.length, 12); footer.writeUInt32LE(offset, 16);
  return Buffer.concat([...local, directoryBytes, footer]);
}
test('Windows extractor accepts a reviewed single-root archive', { skip: process.platform !== 'win32' }, async (t) => {
  const { root, output } = await fixture(t), bytes = zipBytes([{ name: 'browser/file.txt', data: 'safe' }]);
  await writeFile(output, bytes);
  const destination = join(root, 'expanded');
  assert.equal((await expandReviewedRuntimeArchive(output, destination, { sha256: digest(bytes), expectedRoot: 'browser' })).bytes, 4);
  assert.equal(await readFile(join(destination, 'browser/file.txt'), 'utf8'), 'safe');
  await assert.rejects(expandReviewedRuntimeArchive(output, destination, { sha256: digest(bytes), expectedRoot: 'browser' }), /already exists/);
});
for (const [name, entries] of [
  ['traversal', [{ name: 'browser/../escape' }]],
  ['case collision', [{ name: 'browser/File' }, { name: 'browser/file' }]],
  ['implicit directory case collision', [{ name: 'browser/Dir/a' }, { name: 'browser/dir/b' }]],
  ['file-directory collision', [{ name: 'browser/a' }, { name: 'browser/a/b' }]],
  ['symlink', [{ name: 'browser/link', attributes: 0xa1ff0000 }]],
  ['ADS', [{ name: 'browser/file:stream' }]],
  ['reserved Windows name', [{ name: 'browser/CON.txt' }]],
  ['spaced reserved Windows name', [{ name: 'browser/CON .txt' }]],
  ['nonregular DOS device', [{ name: 'browser/device', attributes: 0x40 }]],
  ['backslash', [{ name: 'browser\\file' }]],
  ['empty segment', [{ name: 'browser//file' }]],
  ['wrong root', [{ name: 'other/file' }]],
  ['declared extraction overflow', [{ name: 'browser/file', length: 1610612737 }]],
  ['entry count overflow', Array.from({ length: 10001 }, (_, index) => ({ name: `browser/file-${index}` }))],
]) test(`Windows extractor rejects ${name} before creating output`, { skip: process.platform !== 'win32' }, async (t) => {
  const { root, output } = await fixture(t), bytes = zipBytes(entries);
  await writeFile(output, bytes);
  const destination = join(root, 'expanded');
  await assert.rejects(expandReviewedRuntimeArchive(output, destination, { sha256: digest(bytes), expectedRoot: 'browser' }));
  await absent(destination);
});
