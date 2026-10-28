import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import { OwnerEvidenceStore } from '../../lib/stores/owner-evidence.svelte';
import { fixtureEvidenceReview } from './owner-evidence-fixtures';
import RunReviewScreen from './RunReviewScreen.svelte';

beforeEach(() => { vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} }); });
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
describe('Evidence review method boundaries', () => {
  it('retains manual observation recording and lifecycle controls for a running manual attempt', () => {
    render(RunReviewScreen, { review: fixtureEvidenceReview(), model: new OwnerEvidenceStore() });
    expect(screen.getByRole('button', { name: 'Record this observation' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Finish observation attempt' })).toBeTruthy();
  });
  it.each(['queued', 'running'] as const)('does not offer manual recording, completion, retry or owner decision during automated %s', (lifecycle) => {
    const review = fixtureEvidenceReview(); review.run.method = 'automated'; review.run.lifecycle = lifecycle;
    render(RunReviewScreen, { review, model: new OwnerEvidenceStore() });
    for (const name of ['Record this observation', 'Finish observation attempt', 'Cancel attempt', 'Start separate retry', 'Record owner decision']) expect(screen.queryByRole('button', { name })).toBeNull();
  });
  it('shows the owner decision for terminal automated runs without exposing manual retry', () => {
    const review = fixtureEvidenceReview(); review.run.method = 'automated'; review.run.lifecycle = 'completed';
    render(RunReviewScreen, { review, model: new OwnerEvidenceStore() });
    expect(screen.getByRole('button', { name: 'Record owner decision' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Start separate retry' })).toBeNull();
  });
});
