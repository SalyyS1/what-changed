import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { checkApprovals, gateIds, planDirectory, requiredGates, scopeDigest } from './check-ownvero-approvals.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'ownvero-approval-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, planDirectory, 'reports'), { recursive: true });
  const scope = `${planDirectory}/scope.json`;
  const evidencePath = `${planDirectory}/reports/decision.json`;
  const ledgerPath = `${planDirectory}/approval-ledger.json`;
  await writeFile(join(root, scope), '{"scope":"test fixture"}');
  const digest = await scopeDigest(root, [scope]);
  const evidence = {
    schemaVersion: 1, source: 'user_message', actor: 'user', questionItemId: 'fixture',
    question: 'Authorize fixture?', answer: 'Yes', recordedAt: '2026-10-27T15:00:00Z',
    approvals: { G0: digest, G1: digest },
  };
  const raw = JSON.stringify(evidence);
  await writeFile(join(root, evidencePath), raw);
  const ledger = { schemaVersion: 1, plan: planDirectory.split('/').at(-1), gates: gateIds.map((id) => ({
    id, status: ['G0', 'G1'].includes(id) ? 'approved' : 'pending', scopeFiles: [scope],
    scopeDigest: digest, evidence: evidencePath,
    evidenceSha256: createHash('sha256').update(raw).digest('hex'),
  })) };
  async function save() { await writeFile(join(root, ledgerPath), JSON.stringify(ledger)); }
  await save();
  return { root, scope, evidencePath, ledger, save, check: (required = ['G0', 'G1']) => checkApprovals({ root, required }) };
}

test('phase dispatch never treats implementation permission as publication permission', () => {
  assert.deepEqual(requiredGates(['--phase', '1']), ['G0', 'G1']);
  assert.deepEqual(requiredGates(['--phase', '2']), ['G0', 'G1']);
  assert.deepEqual(requiredGates(['--phase', '7']), ['G0', 'G1']);
  assert.deepEqual(requiredGates(['--phase', '10']), ['G0', 'G1', 'G2']);
  assert.deepEqual(requiredGates(['--publish']), gateIds);
  for (const args of [[], ['--phase', '13'], ['--phase', '02'], ['--phase', '7', '--publish']]) {
    assert.throws(() => requiredGates(args));
  }
});

test('valid scoped human record authorizes only its recorded gates', async (t) => {
  const f = await fixture(t);
  assert.deepEqual(await f.check(), ['G0', 'G1']);
  await assert.rejects(f.check(['G2']), /approval is missing/);
  await assert.rejects(f.check(['G3']), /approval is missing/);
});

test('all pending ledger denies contract, runner and publication dispatch', async (t) => {
  const f = await fixture(t);
  f.ledger.gates.forEach((gate) => { gate.status = 'pending'; });
  await f.save();
  for (const required of [requiredGates(['--phase', '1']), ['G0', 'G1'], ['G2'], gateIds]) await assert.rejects(f.check(required));
});

test('scope changes invalidate approval', async (t) => {
  const f = await fixture(t);
  await writeFile(join(f.root, f.scope), '{"scope":"broader permission"}');
  await assert.rejects(f.check(), /approved scope changed/);
});

test('editing evidence or claiming an unrecorded approval fails closed', async (t) => {
  const f = await fixture(t);
  f.ledger.gates.find((gate) => gate.id === 'G2').status = 'approved';
  await f.save();
  await assert.rejects(f.check(['G2']), /invalid human decision record/);
  const raw = await readFile(join(f.root, f.evidencePath), 'utf8');
  await writeFile(join(f.root, f.evidencePath), raw.replace('"actor":"user"', '"actor":"assistant"'));
  await assert.rejects(f.check(), /approval evidence changed/);
});

test('unknown gates, duplicate ledger entries and path escapes are rejected', async (t) => {
  const f = await fixture(t);
  await assert.rejects(f.check(['G4']), /Unknown/);
  await assert.rejects(scopeDigest(f.root, ['../outside']), /canonical/);
  await assert.rejects(scopeDigest(f.root, ['C:\\outside']), /canonical/);
  f.ledger.gates[3] = f.ledger.gates[0];
  await f.save();
  await assert.rejects(f.check(), /Invalid approval ledger/);
});
