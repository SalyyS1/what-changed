import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readFileSync, realpathSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';

export const TARGET = 'x86_64-pc-windows-msvc';
export const CARGO_EDGES = 'normal,no-proc-macro';
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const compare = (left, right) => left < right ? -1 : left > right ? 1 : 0;

export function readJson(path, label) {
  try { return JSON.parse(readFileSync(path, 'utf8')); }
  catch (error) { throw new Error(`${label} is missing or invalid: ${error.message}`); }
}

export function isInside(directory, path, allowRoot = false) {
  const part = relative(directory, path);
  return (allowRoot || !!part) && part !== '..' && !part.startsWith(`..${sep}`) && !isAbsolute(part);
}

export function sourceFile(directory, file) {
  const path = resolve(directory, file);
  if (!isInside(directory, path) || !isInside(realpathSync(directory), realpathSync(path))) {
    throw new Error(`package source file escapes its package directory: ${file}`);
  }
  if (!lstatSync(realpathSync(path)).isFile()) throw new Error(`package source is not a file: ${file}`);
  return path;
}

// Resolve the nearest existing ancestor too: a missing leaf can still be below
// a junction that points outside the workspace or the legal text directory.
export function outputPath(root, file) {
  const legalRoot = resolve(root, 'legal/third-party');
  const path = resolve(root, file);
  if (!isInside(legalRoot, path)) throw new Error(`output is outside legal/third-party: ${file}`);
  let ancestor = path;
  while (!existsSync(ancestor)) {
    if (lstatExists(ancestor)) throw new Error(`output has a dangling symlink: ${file}`);
    ancestor = dirname(ancestor);
  }
  if (!isInside(realpathSync(root), realpathSync(ancestor), true)) {
    throw new Error(`output symlink escapes the workspace: ${file}`);
  }
  if (existsSync(legalRoot) && !isInside(realpathSync(legalRoot), realpathSync(ancestor), true)) {
    throw new Error(`output symlink escapes legal/third-party: ${file}`);
  }
  return path;
}

function lstatExists(path) {
  try { lstatSync(path); return true; } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}

export function validateIdentity(ecosystem, name, version) {
  const pattern = ecosystem === 'cargo' ? /^[A-Za-z0-9_-]+$/ : /^(?:@[a-z0-9._-]+\/)?[a-z0-9._-]+$/;
  if (typeof name !== 'string' || !pattern.test(name)
    || name.split('/').some((part) => part === '.' || part === '..')
    || typeof version !== 'string' || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.+-]+)?$/.test(version)) {
    throw new Error(`unsafe or invalid ${ecosystem} package identity`);
  }
}

export const packageKey = ({ ecosystem, name, version }) => `${ecosystem}:${name}@${version}`;
