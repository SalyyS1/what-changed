import test from 'node:test';
import assert from 'node:assert/strict';
import { validateLaunchCopy, checkLaunchCopy, archiveLabel } from './check-launch-copy.mjs';
const evidence = '**Evidence state: development source; no approved release.**\n';

test('factual AI-builder positioning and bounded evidence are allowed', () => {
  const copy = `${evidence}Build with AI. Stay in charge. Review a Markdown brief with your coding AI. A passed scenario covers its declared expectation. The owner accepts, rejects, or defers.`;
  assert.deepEqual(validateLaunchCopy(copy), []);
});
test('unsupported safety, unbounded testing, autonomy, and causal claims are rejected', () => {
  for (const claim of ['100% safe', 'completely secure', 'fully secure', 'guarantees no regressions',
    'verifies every app', 'tests everything', 'AI autopilot', 'automatically accepts changes',
    'automatically fixes code', 'finds the root cause', 'an toàn tuyệt đối', 'tự sửa ứng dụng']) {
    assert.ok(validateLaunchCopy(`${evidence}${claim}`).length > 0, claim);
  }
});
test('archive disposition is mandatory and never presented as active validation', () => {
  assert.ok(validateLaunchCopy(evidence, { archived: true }).length);
  const archived = `${archiveLabel}\n[Current owner-workspace messaging](messaging.md)\n${evidence}Historical text is retained for provenance.`;
  assert.deepEqual(validateLaunchCopy(archived, { archived: true }), []);
});
test('all six legacy consumers are explicitly archived with the current fact sheet', async () => {
  assert.deepEqual(await checkLaunchCopy(), { archived: 6, active: 1 });
});
