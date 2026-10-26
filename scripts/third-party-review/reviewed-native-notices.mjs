import { execFileSync } from 'node:child_process';
import { readFileSync, realpathSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { collectPackage } from './collect-package.mjs';
import { TARGET, packageKey, readJson, sha256, sourceFile } from './filesystem.mjs';
import { isHttpsUrl, noticeSection, reviewedLicense } from './reviewed-license-notices.mjs';

const REQUIRED = new Map([['libsqlite3-sys', 'SQLite'], ['webview2-com-sys', 'WebView2Loader']]);

function evidenceRecords(component) {
  const files = component.nativeFiles ?? component.sourceFiles;
  const records = files ?? (component.sourceFile ? [component] : []);
  if (!Array.isArray(records) || !records.length
    || component.selectionEvidence !== undefined && !Array.isArray(component.selectionEvidence)) {
    throw new Error(`native source evidence is missing or invalid: ${component.name}`);
  }
  return [...records, ...(component.selectionEvidence ?? [])];
}

function exactRecord(record, actual) {
  return record?.sourceFile === actual.sourceFile && record?.sourceSha256 === actual.sourceSha256
    && record?.sourceBytes === actual.sourceBytes;
}

function compareKnownEvidence(component, actual) {
  const fields = actual.name === 'SQLite'
    ? ['version', 'sourceId', 'sourceFile', 'sourceSha256', 'sourceBytes'] : ['target'];
  if (fields.some((field) => component[field] !== actual[field])) {
    throw new Error(`native source identity differs from installed package: ${component.name}`);
  }
  const records = evidenceRecords(component);
  const expected = evidenceRecords(actual);
  if (records.length !== expected.length || expected.some((file) => !records.some((record) => exactRecord(record, file)))
    || actual.nativeFiles && (!Array.isArray(component.nativeFiles) || component.nativeFiles.length !== actual.nativeFiles.length
      || actual.nativeFiles.some((file) => !component.nativeFiles.some((record) => exactRecord(record, file))))
    || actual.selectionEvidence && (!Array.isArray(component.selectionEvidence)
      || component.selectionEvidence.length !== actual.selectionEvidence.length
      || actual.selectionEvidence.some((file) => !component.selectionEvidence.some((record) => exactRecord(record, file))))) {
    throw new Error(`native source evidence differs from installed package: ${component.name}`);
  }
}

function validateSourceBytes(directory, component) {
  const seen = new Set();
  for (const record of evidenceRecords(component)) {
    if (typeof record?.sourceFile !== 'string' || seen.has(record.sourceFile)
      || !/^[a-f0-9]{64}$/.test(record.sourceSha256 ?? '')
      || !Number.isSafeInteger(record.sourceBytes) || record.sourceBytes <= 0) {
      throw new Error(`native source evidence is incomplete or duplicated: ${component.name}`);
    }
    seen.add(record.sourceFile);
    const bytes = readFileSync(sourceFile(directory, record.sourceFile));
    if (bytes.length !== record.sourceBytes || sha256(bytes) !== record.sourceSha256) {
      throw new Error(`native source evidence differs from installed package: ${component.name}`);
    }
  }
}

function cargoMetadata(root, override) {
  const metadata = override ? readJson(override, 'native Cargo metadata') : JSON.parse(execFileSync('cargo', [
    'metadata', '--locked', '--offline', '--manifest-path', 'src-tauri/Cargo.toml',
    '--filter-platform', TARGET, '--format-version', '1',
  ], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
  if (metadata.version !== 1 || !Array.isArray(metadata.packages)) throw new Error('native Cargo metadata schema is invalid');
  return metadata;
}

export function reviewedNativeSections({ root, manifest, cargoMetadata: metadataFile }) {
  const entries = manifest.entries.filter((entry) => {
    const required = entry.ecosystem === 'cargo' && REQUIRED.get(entry.name);
    if (entry.nativeComponents !== undefined && !Array.isArray(entry.nativeComponents)) {
      throw new Error(`native components must be an array: ${packageKey(entry)}`);
    }
    if (required && entry.nativeComponents?.filter((component) => component?.name === required).length !== 1) {
      throw new Error(`exactly one reviewed ${required} native component is required: ${packageKey(entry)}`);
    }
    return required || entry.nativeComponents?.length;
  });
  if (!entries.length) return '';
  const expected = manifest.cargoManifestSha256;
  if (!/^[a-f0-9]{64}$/.test(expected ?? '') || sha256(readFileSync(resolve(root, 'src-tauri/Cargo.toml'))) !== expected) {
    throw new Error('notice review is stale for src-tauri/Cargo.toml');
  }
  const metadata = entries.some((entry) => entry.ecosystem === 'cargo') ? cargoMetadata(root, metadataFile) : null;
  const sections = [];
  for (const entry of entries.sort((a, b) => packageKey(a).localeCompare(packageKey(b)))) {
    const matches = metadata?.packages.filter((item) => item.name === entry.name && item.version === entry.version);
    if (entry.ecosystem === 'cargo' && matches.length !== 1) {
      throw new Error(`native Cargo package identity is missing or ambiguous: ${packageKey(entry)}`);
    }
    const candidate = { ecosystem: entry.ecosystem, name: entry.name, version: entry.version, metadata: matches?.[0] };
    const installed = collectPackage(root, candidate).entry.nativeComponents;
    const directory = entry.ecosystem === 'cargo' ? realpathSync(dirname(matches[0].manifest_path))
      : realpathSync(resolve(root, 'node_modules', entry.name));
    const names = new Set();
    for (const component of [...entry.nativeComponents].sort((a, b) => String(a?.name).localeCompare(String(b?.name)))) {
      if (!component || typeof component.name !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9 ._-]*$/.test(component.name)
        || names.has(component.name)) throw new Error(`native component identity is invalid or duplicated: ${packageKey(entry)}`);
      names.add(component.name);
      const key = `native:${packageKey(entry)}/${component.name}@${component.version ?? 'version-unresolved'}`;
      if (component.status !== undefined && component.status !== 'reviewed') throw new Error(`native review is pending: ${key}`);
      const reviewer = component.reviewedBy ?? manifest.reviewedBy;
      const reviewedAt = component.reviewedAt ?? manifest.reviewedAt;
      if (typeof reviewer !== 'string' || !reviewer.trim() || typeof reviewedAt !== 'string' || !reviewedAt.trim()) {
        throw new Error(`native reviewer and review date are missing: ${key}`);
      }
      const license = reviewedLicense(root, component, key);
      if (component.version !== null && (typeof component.version !== 'string' || !component.version.trim())) {
        throw new Error(`native version decision is missing: ${key}`);
      }
      const actual = installed.find((item) => item.name === component.name);
      if ((component.version === null || actual?.version === null)
        && (component.provenanceReviewed !== true || !isHttpsUrl(component.provenanceUrl ?? component.sourceUrl))) {
        throw new Error(`native provenance review is required for unresolved source version: ${key}`);
      }
      const provenanceUrl = component.provenanceUrl ?? component.sourceUrl;
      if (component.provenanceUrl !== undefined && !isHttpsUrl(component.provenanceUrl)
        || component.provenanceReviewed === true && !isHttpsUrl(provenanceUrl)) {
        throw new Error(`authoritative HTTPS native provenance URL is invalid: ${key}`);
      }
      if (actual) compareKnownEvidence(component, actual);
      validateSourceBytes(directory, component);
      const provenance = component.provenanceReviewed === true ? `Reviewed provenance: ${provenanceUrl}\n` : '';
      sections.push(`${noticeSection(key, license)}Reviewed by: ${reviewer}\nReviewed at: ${reviewedAt}\n${provenance}`);
    }
  }
  return sections.join('');
}
