import { createHash } from 'node:crypto';
import { execFile } from 'node:child_process';
import { lstat, open, unlink } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const run = promisify(execFile);
const archiveScript = fileURLToPath(new URL('./expand-reviewed-runtime-archive.ps1', import.meta.url));

export function validateReviewedArtifact(record) {
  let url;
  try { url = new URL(record?.url); } catch { throw new Error('Reviewed artifact requires an HTTPS URL'); }
  if (url.protocol !== 'https:' || url.username || url.password) throw new Error('Reviewed artifact requires an HTTPS URL without credentials');
  if (!Number.isSafeInteger(record?.bytes) || record.bytes <= 0) throw new Error('Reviewed artifact requires a positive safe byte count');
  if (!/^[a-f0-9]{64}$/.test(record?.sha256 ?? '')) throw new Error('Reviewed artifact requires a SHA-256 hash');
}

async function regularAncestors(destination) {
  let directory = dirname(destination);
  while (true) {
    const item = await lstat(directory);
    if (item.isSymbolicLink() || !item.isDirectory()) throw new Error(`Artifact ancestor is not a regular directory: ${directory}`);
    const parent = dirname(directory);
    if (parent === directory) return;
    directory = parent;
  }
}

async function httpsResponse(url, fetchImpl) {
  for (let redirects = 0; redirects <= 5; redirects += 1) {
    const response = await fetchImpl(url, { redirect: 'manual' });
    if (![301, 302, 303, 307, 308].includes(response.status)) {
      if (!response.ok || !response.body) throw new Error(`Reviewed artifact download failed: HTTP ${response.status}`);
      return response;
    }
    const location = response.headers?.get('location');
    await response.body?.cancel?.();
    if (!location || redirects === 5) throw new Error('Reviewed artifact redirect is missing or exceeds the limit');
    const next = new URL(location, url);
    if (next.protocol !== 'https:' || next.username || next.password) throw new Error('Reviewed artifact redirect requires HTTPS without credentials');
    url = next.href;
  }
  throw new Error('Reviewed artifact redirect limit exceeded');
}

/** Stream only reviewed bytes; a failed invocation owns only the file it created. */
export async function downloadVerifiedArtifact(record, destination, { fetchImpl = fetch } = {}) {
  validateReviewedArtifact(record);
  const output = resolve(destination);
  await regularAncestors(output);
  const file = await open(output, 'wx');
  const identity = await file.stat();
  try {
    const response = await httpsResponse(record.url, fetchImpl);
    const hash = createHash('sha256');
    let bytes = 0;
    for await (const value of response.body) {
      const chunk = Buffer.from(value);
      if (chunk.length > record.bytes - bytes) throw new Error('Reviewed artifact exceeds its approved byte count');
      await file.writeFile(chunk);
      hash.update(chunk);
      bytes += chunk.length;
    }
    const sha256 = hash.digest('hex');
    if (bytes !== record.bytes) throw new Error('Reviewed artifact byte count differs');
    if (sha256 !== record.sha256) throw new Error('Reviewed artifact SHA-256 differs');
    await file.close();
    return { url: record.url, destination: output, bytes, sha256 };
  } catch (error) {
    await file.close().catch(() => {});
    try {
      const current = await lstat(output);
      if (current.isFile() && current.dev === identity.dev && current.ino === identity.ino) await unlink(output);
    } catch (cleanupError) {
      if (cleanupError.code !== 'ENOENT') throw new AggregateError([error, cleanupError], 'Artifact download and partial-file cleanup failed');
    }
    throw error;
  }
}

export async function expandReviewedRuntimeArchive(archive, destination, { sha256, expectedRoot } = {}) {
  if (process.platform !== 'win32') throw new Error('Reviewed runtime expansion requires Windows');
  if (!/^[a-f0-9]{64}$/.test(sha256 ?? '') || typeof expectedRoot !== 'string' || !expectedRoot) {
    throw new Error('Reviewed runtime expansion requires the archive SHA-256 and expected root');
  }
  const { stdout } = await run('powershell.exe', [
    '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', archiveScript,
    '-Archive', resolve(archive), '-Destination', resolve(destination), '-Sha256', sha256, '-ExpectedRoot', expectedRoot,
  ], { windowsHide: true, maxBuffer: 1024 * 1024 });
  return JSON.parse(stdout.trim());
}
