import { existsSync, readFileSync, realpathSync } from 'node:fs';
import { isAbsolute, relative, resolve, sep } from 'node:path';
import { sha256 } from './filesystem.mjs';

export function isHttpsUrl(value) {
  if (typeof value !== 'string') return false;
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && !!url.hostname;
  } catch { return false; }
}

function licenseTextPath(root, textFile) {
  if (typeof textFile !== 'string' || isAbsolute(textFile)) {
    throw new Error('each approved license text must use a relative legal/third-party path');
  }
  const legalRoot = resolve(root, 'legal', 'third-party');
  const path = resolve(root, textFile);
  const inside = relative(legalRoot, path);
  if (!inside || inside === '..' || inside.startsWith(`..${sep}`) || isAbsolute(inside)) {
    throw new Error(`license text is outside legal/third-party: ${textFile}`);
  }
  if (existsSync(path)) {
    const resolved = realpathSync(path);
    const realInside = relative(realpathSync(legalRoot), resolved);
    if (!realInside || realInside === '..' || realInside.startsWith(`..${sep}`) || isAbsolute(realInside)) {
      throw new Error(`license text resolves outside legal/third-party: ${textFile}`);
    }
  }
  return path;
}

export function reviewedLicense(root, entry, key) {
  if (!entry.licenseExpression || typeof entry.licenseExpression !== 'string') {
    throw new Error(`license expression is missing: ${key}`);
  }
  if (!entry.selectedLicense || typeof entry.selectedLicense !== 'string'
    || entry.obligationsReviewed !== true || !Array.isArray(entry.noticeFiles)) {
    throw new Error(`license choice, obligation review or NOTICE file decision is missing: ${key}`);
  }
  if (/\b(?:MPL|GPL|LGPL|AGPL)-/i.test(entry.selectedLicense)
    && !isHttpsUrl(entry.sourceAvailabilityUrl)) {
    throw new Error(`source-availability URL is missing for ${key}`);
  }
  if (entry.sourceAvailabilityUrl !== undefined && !isHttpsUrl(entry.sourceAvailabilityUrl)) {
    throw new Error(`authoritative HTTPS source-availability URL is invalid: ${key}`);
  }
  if (!isHttpsUrl(entry.sourceUrl)) {
    throw new Error(`authoritative HTTPS source URL is missing or invalid: ${key}`);
  }
  const path = licenseTextPath(root, entry.textFile);
  if (!existsSync(path)) throw new Error(`license text is missing: ${key}`);
  const bytes = readFileSync(path);
  if (!/^[a-f0-9]{64}$/.test(entry.textSha256 ?? '') || sha256(bytes) !== entry.textSha256) {
    throw new Error(`license text hash does not match reviewed bytes: ${key}`);
  }
  if (!bytes.length) throw new Error(`license text is empty: ${key}`);
  const notices = entry.noticeFiles.map((notice) => {
    const noticePath = licenseTextPath(root, notice?.file);
    if (!existsSync(noticePath)) throw new Error(`NOTICE text is missing: ${key}`);
    const noticeBytes = readFileSync(noticePath);
    if (!noticeBytes.length || !/^[a-f0-9]{64}$/.test(notice?.sha256 ?? '')
      || sha256(noticeBytes) !== notice.sha256) {
      throw new Error(`NOTICE text hash does not match reviewed bytes: ${key}`);
    }
    return noticeBytes.toString('utf8').trimEnd();
  });
  return { entry, text: bytes.toString('utf8').trimEnd(), notices };
}

export function noticeSection(key, { entry, text, notices }) {
  const sourceAvailability = entry.sourceAvailabilityUrl ? `Source availability: ${entry.sourceAvailabilityUrl}\n` : '';
  return `\n--- ${key} ---\nDeclared license: ${entry.licenseExpression}\nSelected license: ${entry.selectedLicense}\nReviewed source: ${entry.sourceUrl}\n${sourceAvailability}\n${text}\n${notices.map((notice) => `\nNOTICE\n${notice}\n`).join('')}`;
}
