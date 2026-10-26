import { readFileSync } from 'node:fs';
import { TARGET, sha256, sourceFile } from './filesystem.mjs';

export function collectNativeComponents(directory, candidate) {
  if (candidate.ecosystem !== 'cargo') return [];
  if (candidate.name === 'libsqlite3-sys') return [sqliteComponent(directory)];
  if (candidate.name === 'webview2-com-sys') return [webview2Component(directory)];
  return [];
}

function sqliteComponent(directory) {
  const file = 'sqlite3/sqlite3.c';
  const bytes = readFileSync(sourceFile(directory, file));
  const text = bytes.toString('utf8');
  const version = /^#define\s+SQLITE_VERSION\s+"([^"]+)"/m.exec(text)?.[1];
  const sourceId = /^#define\s+SQLITE_SOURCE_ID\s+"([^"]+)"/m.exec(text)?.[1];
  if (!version || !sourceId) throw new Error('bundled SQLite version/source ID are missing from sqlite3.c');
  return { name: 'SQLite', version, sourceId, sourceFile: file, sourceSha256: sha256(bytes),
    sourceBytes: bytes.length, licenseExpression: null, obligationsReviewed: false,
    evidenceScope: 'Bundled sqlite3/sqlite3.c source; selection and license obligations require release review.' };
}

function requiredEvidence(directory, file) {
  const bytes = readFileSync(sourceFile(directory, file));
  if (!bytes.length) throw new Error(`required WebView2Loader evidence file is empty: ${file}`);
  return { sourceFile: file, sourceSha256: sha256(bytes), sourceBytes: bytes.length };
}

function webview2Component(directory) {
  // The crate version/license describes its wrapper, not the packaged SDK loader.
  // Hash candidate bytes in place; binary files are never copied as license text.
  const nativeFiles = ['x64/WebView2Loader.dll', 'x64/WebView2Loader.dll.lib', 'x64/WebView2LoaderStatic.lib']
    .map((file) => requiredEvidence(directory, file));
  const selectionEvidence = ['build.rs', 'src/lib.rs'].map((file) => requiredEvidence(directory, file));
  return { name: 'WebView2Loader', target: TARGET, version: null, licenseExpression: null,
    obligationsReviewed: false, nativeFiles, selectionEvidence,
    evidenceScope: 'Packaged x64 loader candidates and crate selection source only; not final binary link proof. SDK version and native license obligations require release review.' };
}
