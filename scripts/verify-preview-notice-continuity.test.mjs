import test from 'node:test';
import assert from 'node:assert/strict';
import { assertFeatureOnlyChange } from './verify-preview-notice-continuity.mjs';

const before = '[dependencies]\nwindows = { features = [\n    "Other",\n] }\n';
const after = before.replace('    "Other",', '    "Other",\n    "Win32_System_JobObjects",');
test('only the existing Windows JobObjects feature preserves reviewed dependency terms', () => {
  assert.doesNotThrow(() => assertFeatureOnlyChange(before, after));
  assert.doesNotThrow(() => assertFeatureOnlyChange(before.replaceAll('\n', '\r\n'), after));
  assert.throws(() => assertFeatureOnlyChange(before, `${after}\nnew_dependency = "1"`));
  assert.throws(() => assertFeatureOnlyChange(before, after.replace('"Other"', '"Changed"')));
  assert.throws(() => assertFeatureOnlyChange(before, before));
});
