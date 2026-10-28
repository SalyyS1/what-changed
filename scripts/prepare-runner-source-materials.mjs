import { lstat, mkdir, readFile, writeFile } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { downloadVerifiedArtifact, validateReviewedArtifact } from './fetch-reviewed-runtime-artifact.mjs';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const RESERVED = ['runner-browser-source.json', 'SHA256SUMS.txt', 'source-materials-receipt.json'];

function validateRequiredMembers(record) {
  if (!/\.(?:tar\.xz|tar\.gz|zip)$/i.test(record.name)) throw new Error('Browser source material has an unsupported archive format');
  const members = record.requiredMembers;
  if (!Array.isArray(members) || !members.length || new Set(members).size !== members.length
    || members.some((member) => typeof member !== 'string' || !member || member.length > 32768
      || /[\\:*?"<>|\x00-\x1f\x7f]/.test(member)
      || member.split('/').some((segment) => !segment || segment === '.' || segment === '..' || /[ .]$/.test(segment)))) {
    throw new Error('Browser source material requires nonempty safe relative archive members');
  }
}

function inspectSourceArchive(archive, requiredMembers) {
  return new Promise((accept, reject) => {
    const child = spawn(process.platform === 'win32' ? 'tar.exe' : 'tar', ['-tf', archive], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const required = new Set(requiredMembers), matched = new Set();
    let pending = '', stderr = '', archiveEntryCount = 0, failure;
    const fail = (error) => { failure ??= error; child.kill(); };
    const readLine = (line) => {
      if (line.length > 32768) throw new Error('Source archive member exceeds the supported path length');
      if (!line) return;
      archiveEntryCount += 1;
      const member = line.replace(/\r$/, '').replace(/^(?:\.\/)+/, '').replace(/\/$/, '');
      if (required.has(member)) matched.add(member);
    };
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      if (failure) return;
      try {
        pending += chunk;
        let boundary;
        while ((boundary = pending.indexOf('\n')) !== -1) {
          readLine(pending.slice(0, boundary));
          pending = pending.slice(boundary + 1);
        }
        if (pending.length > 32768) throw new Error('Source archive member exceeds the supported path length');
      } catch (error) { fail(error); }
    });
    child.stderr.on('data', (chunk) => { stderr = (stderr + chunk).slice(-8192); });
    child.stdout.on('error', fail);
    child.stderr.on('error', fail);
    child.once('error', (error) => { failure ??= error; });
    child.once('close', (code, signal) => {
      try {
        if (failure) throw failure;
        if (code !== 0) throw new Error(`Source archive listing failed (${signal ?? code}): ${stderr.trim()}`);
        if (pending) readLine(pending);
        const missing = requiredMembers.filter((member) => !matched.has(member));
        if (missing.length) throw new Error(`Source archive is missing required members: ${missing.slice(0, 8).join(', ')}`);
        accept({ archiveEntryCount, matchedMembers: requiredMembers.filter((member) => matched.has(member)) });
      } catch (error) { reject(error); }
    });
  });
}

/** Source and binary downloads share one artifact, access policy and retention. */
export async function prepareRunnerSourceMaterials({ root = ROOT, fetchImpl = fetch } = {}) {
  const descriptorPath = resolve(root, 'release/runner-browser-source.json');
  const descriptorBytes = await readFile(descriptorPath);
  const descriptor = JSON.parse(descriptorBytes);
  if (descriptor.schemaVersion !== 1 || !Array.isArray(descriptor.sourceMaterials) || !descriptor.sourceMaterials.length) {
    throw new Error('Reviewed browser source materials are missing');
  }
  const names = new Set(RESERVED.map((name) => name.toLowerCase()));
  for (const record of descriptor.sourceMaterials) {
    if (typeof record?.name !== 'string' || record.name.length > 160 || basename(record.name) !== record.name
      || !/^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(record.name) || /[.]$/.test(record.name)
      || /^(?:CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])(?:\.|$)/i.test(record.name)
      || names.has(record.name.toLowerCase())) throw new Error('Browser source material has an unsafe or duplicate name');
    names.add(record.name.toLowerCase());
    validateReviewedArtifact(record);
    validateRequiredMembers(record);
  }
  const artifacts = resolve(root, 'preview-artifacts');
  try { await mkdir(artifacts); } catch (error) { if (error.code !== 'EEXIST') throw error; }
  const parent = await lstat(artifacts);
  if (!parent.isDirectory() || parent.isSymbolicLink()) throw new Error('Preview artifacts must be a regular directory');
  const output = join(artifacts, 'runtime-sources');
  await mkdir(output); // Refuse an existing source packet, including a partial previous run.
  const materials = [];
  for (const record of descriptor.sourceMaterials) {
    const result = await downloadVerifiedArtifact(record, join(output, record.name), { fetchImpl });
    const members = await inspectSourceArchive(join(output, record.name), record.requiredMembers);
    materials.push({ name: record.name, url: result.url, bytes: result.bytes, sha256: result.sha256, ...members });
  }
  await writeFile(join(output, RESERVED[0]), descriptorBytes, { flag: 'wx' });
  const descriptorSha256 = createHash('sha256').update(descriptorBytes).digest('hex');
  const receipt = { schemaVersion: 1, descriptorSha256, sourceMaterials: materials };
  const receiptBytes = `${JSON.stringify(receipt, null, 2)}\n`;
  await writeFile(join(output, RESERVED[2]), receiptBytes, { flag: 'wx' });
  const checksums = [...materials.map(({ name, sha256 }) => `${sha256}  ${name}`),
    `${descriptorSha256}  ${RESERVED[0]}`, `${createHash('sha256').update(receiptBytes).digest('hex')}  ${RESERVED[2]}`];
  await writeFile(join(output, RESERVED[1]), `${checksums.join('\n')}\n`, { flag: 'wx' });
  return { output, sourceMaterials: materials.length, bytes: materials.reduce((sum, item) => sum + item.bytes, 0), descriptorSha256 };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 2) throw new Error('Usage: node scripts/prepare-runner-source-materials.mjs');
    console.log(JSON.stringify(await prepareRunnerSourceMaterials()));
  } catch (error) {
    console.error(`Runner source materials blocked: ${error.message}`);
    process.exitCode = 1;
  }
}
