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
});
