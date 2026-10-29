import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { verifyNsisNoticeCopy } from './verify-nsis-notice-copy.mjs';

test('NSIS notice verifier requires a real root resource copy with reviewed bytes', (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'what-changed-nsis-notice-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const source = join(directory, 'THIRD_PARTY_NOTICES.txt');
  const reviewed = Buffer.from('reviewed exact-version notice text\n');
  writeFileSync(source, reviewed);
  const script = `Section Install\n  SetOutPath $INSTDIR\n  ; Copy resources\n  File /a "/oname=THIRD_PARTY_NOTICES.txt" "${source}"\n  ; Copy external binaries\nSectionEnd\n`;
  assert.equal(verifyNsisNoticeCopy(script, reviewed), source);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('  File /a', '  ; File /a'), reviewed), /exactly one active/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('/oname=THIRD_PARTY_NOTICES.txt', '/oname=other.txt'), reviewed), /does not copy the notice resource/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('SetOutPath $INSTDIR', 'SetOutPath $SYSDIR'), reviewed), /installation directory/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('  ; Copy external binaries', `  File /a "/oname=THIRD_PARTY_NOTICES.txt" "${source}"\n  ; Copy external binaries`), reviewed), /exactly one active/);
  assert.throws(() => verifyNsisNoticeCopy(script, Buffer.from('different notice')), /differs from the reviewed payload/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('Section Install', 'Section Uninstall'), reviewed), /install section/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('  ; Copy resources', '  ; Copy resources\n  ; File /a "/oname=THIRD_PARTY_NOTICES.txt" "ignored"').replace('  File /a', '  ; File /a'), reviewed), /exactly one active/);
});

test('desktop and runner notices coexist and each verifies its own exact filename and bytes', (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'what-changed-nsis-notice-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const desktopSource = join(directory, 'THIRD_PARTY_NOTICES.txt');
  const runnerSource = join(directory, 'RUNNER_THIRD_PARTY_NOTICES.txt');
  const desktop = Buffer.from('desktop terms\n'), runner = Buffer.from('runtime terms\n');
  writeFileSync(desktopSource, desktop);
  writeFileSync(runnerSource, runner);
  const desktopCopy = `  File /a "/oname=THIRD_PARTY_NOTICES.txt" "${desktopSource}"`;
  const runnerCopy = `  File /a "/oname=RUNNER_THIRD_PARTY_NOTICES.txt" "${runnerSource}"`;
  const script = `Section Install\n  SetOutPath $INSTDIR\n  ; Copy resources\n${desktopCopy}\n${runnerCopy}\n  ; Copy external binaries\nSectionEnd\n`;
  assert.equal(verifyNsisNoticeCopy(script, desktop), desktopSource);
  assert.equal(verifyNsisNoticeCopy(script, runner, 'RUNNER_THIRD_PARTY_NOTICES.txt'), runnerSource);
  assert.throws(() => verifyNsisNoticeCopy(script, desktop, 'RUNNER_THIRD_PARTY_NOTICES.txt'), /differs from the reviewed payload/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace(desktopCopy + '\n', ''), desktop), /exactly one active/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace(runnerCopy, runnerCopy + '\n' + runnerCopy), runner, 'RUNNER_THIRD_PARTY_NOTICES.txt'), /exactly one active/);
  assert.throws(() => verifyNsisNoticeCopy(script.replace(runnerCopy, '  SetOutPath $SYSDIR\n' + runnerCopy), runner, 'RUNNER_THIRD_PARTY_NOTICES.txt'), /installation directory/);
});

test('custom notice filenames are safe and matched literally', (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'what-changed-nsis-notice-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const source = join(directory, 'NOTICE.version.txt'), reviewed = Buffer.from('terms');
  writeFileSync(source, reviewed);
  const script = `Section Install\n  SetOutPath $INSTDIR\n  ; Copy resources\n  File /a "/oname=NOTICE.version.txt" "${source}"\n  ; Copy external binaries\nSectionEnd\n`;
  assert.equal(verifyNsisNoticeCopy(script, reviewed, 'NOTICE.version.txt'), source);
  assert.throws(() => verifyNsisNoticeCopy(script.replace('/oname=NOTICE.version.txt', '/oname=NOTICE_version.txt'), reviewed, 'NOTICE.version.txt'), /does not copy/);
  for (const invalid of ['../NOTICE.txt', 'folder/NOTICE.txt', 'NOTICE:stream', 'NOTICE.txt"', 'NUL.txt', '', null]) {
    assert.throws(() => verifyNsisNoticeCopy(script, reviewed, invalid), /safe resource filename/);
  }
});
