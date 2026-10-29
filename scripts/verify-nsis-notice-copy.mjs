import { readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Tauri 2.11's NSIS template emits resources inside Section Install as
// File /a "/oname=<target>" "<source>". Check the active command and its
// source bytes, not a filename mention in a comment or uninstall stanza.
export function verifyNsisNoticeCopy(script, reviewedBytes, targetFilename = 'THIRD_PARTY_NOTICES.txt') {
  if (typeof targetFilename !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,159}$/.test(targetFilename)
    || targetFilename.endsWith('.') || /^(?:CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])(?:\.|$)/i.test(targetFilename)) {
    throw new Error('NSIS notice target must be a safe resource filename');
  }
  const escapedFilename = targetFilename.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const filenameMention = new RegExp(`(?:^|[\\\\/"=\\t ])${escapedFilename}(?=["\\t ]|$)`, 'i');
  const install = /^[ \t]*Section[ \t]+Install[ \t]*\r?\n(?<body>[\s\S]*?)^[ \t]*SectionEnd[ \t]*\r?$/m.exec(script)?.groups?.body;
  if (!install) throw new Error('NSIS install section is missing or incomplete');
  const resourceStart = install.search(/^[ \t]*;[ \t]*Copy resources[ \t]*\r?$/m);
  const externalStart = install.search(/^[ \t]*;[ \t]*Copy external binaries[ \t]*\r?$/m);
  if (resourceStart < 0 || externalStart <= resourceStart) {
    throw new Error('NSIS resource-copy block is missing or incomplete');
  }
  const resourceBlock = install.slice(resourceStart, externalStart);
  const activeMentions = install.split(/\r?\n/).filter((line) =>
    /^[ \t]*File\b/i.test(line) && filenameMention.test(line));
  if (activeMentions.length !== 1) {
    throw new Error('NSIS must contain exactly one active third-party notice copy');
  }
  const copyCommand = new RegExp(`^[ \\t]*File[ \\t]+/a[ \\t]+"/oname=${escapedFilename}"[ \\t]+"(?<source>[^"\\r\\n]+)"[ \\t]*\\r?$`, 'm');
  const copy = copyCommand.exec(resourceBlock);
  if (!copy || copy[0].trim() !== activeMentions[0].trim()) {
    throw new Error('NSIS does not copy the notice resource to the installation root');
  }
  const preceding = install.slice(0, resourceStart + copy.index);
  const outputPaths = [...preceding.matchAll(/^[ \t]*SetOutPath[ \t]+([^\r\n;]+)$/gm)];
  if (outputPaths.at(-1)?.[1].trim() !== '$INSTDIR') {
    throw new Error('NSIS notice copy does not target the installation directory');
  }
  const source = copy.groups.source;
  if (!isAbsolute(source)) throw new Error('NSIS notice copy source is not an absolute path');
  if (!readFileSync(source).equals(reviewedBytes)) {
    throw new Error('NSIS notice copy source differs from the reviewed payload');
  }
  return source;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (![4, 5].includes(process.argv.length)) throw new Error('usage: node scripts/verify-nsis-notice-copy.mjs <installer.nsi> <reviewed-notice.txt> [target-filename]');
    verifyNsisNoticeCopy(readFileSync(process.argv[2], 'utf8'), readFileSync(process.argv[3]), process.argv[4]);
    process.stdout.write('NSIS notice copy: PASS\n');
  } catch (error) {
    process.stderr.write(`NSIS notice copy blocked: ${error.message}\n`);
    process.exitCode = 1;
  }
}
