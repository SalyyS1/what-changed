import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import { ready } from '../../lib/types/async-state';
import { toInvestigationView } from './investigation-adapter';
import { NO_CAUSE_COPY, type InvestigationView, type NoCauseReason } from './investigation-types';
import InvestigationsScreen from './InvestigationsScreen.svelte';
import ProblemForm from './ProblemForm.svelte';

const now = Date.now();
const raw = {
  id: 907,
  project_id: 'project:api',
  problem: 'my app cannot connect to the api',
  window: { kind: 'today' },
  resolved_window: { from: now - 3_600_000, to: now },
  total_events_in_window: 5_148,
  created_at: now,
  dropped_event_count: 148,
  no_cause_reason: null,
  suspects: [{
    event: { id: 3_004, timestamp: now - 2_400_000, user_friendly_summary: '.env PORT changed', entity_display_name: 'API service' },
    score: { total: 9.4, temporal: 0.7, relationship: 1, category: 1, severity: 0.6, rarity: 1, failure_proximity: 1, chain_bonus: 1.5 },
    evidence: 'confirmed',
    explanation: {
      confidence_sentence: 'This change directly affected API service.',
      relative_time: '40 minutes before you noticed the problem',
      why_ranked: ['Changed a file that this service reads', 'First time this setting changed in 90 days'],
      remediation: 'Restore the expected port or update the client configuration.',
    },
    chain_steps: [
      { event_id: 3004, title: '.env PORT changed', timestamp: now - 2_400_000 },
      { event_id: 3005, title: 'API service restarted', timestamp: now - 2_399_700 },
      { event_id: 3006, title: 'Port 4000 opened', timestamp: now - 2_399_400 },
    ],
  }],
};

describe('InvestigationsScreen', () => {
  it('adapts the generated timestamp_ms field without a legacy nested event', () => {
    const result = toInvestigationView({
      id: 1,
      problem: 'network stopped',
      window: { kind: 'today' },
      resolved_window: { from_ms: 100, to_ms: 500 },
      total_events_in_window: 1,
      analyzed_events: 1,
      dropped_events: 0,
      created_at: 500,
      no_cause_reason: null,
      no_cause_message: null,
      suspects: [{
        event_id: 7,
        title: 'Service failed',
        timestamp_ms: 321,
        score: 2,
        breakdown: {},
        confidence: 'correlated',
        confidence_text: 'This changed around the same time.',
        relative_time: 'just before',
        why_ranked: [],
        chain: [],
        what_to_try: 'Compare against a checkpoint.',
      }],
    });
    expect(result.suspects[0]?.event.timestamp).toBe(321);
  });

  it('disables a reversed custom time range', () => {
    render(ProblemForm, {
      problem: 'network stopped',
      window: { kind: 'custom', from: 500, to: 100 },
      onsubmit: () => {},
    });
    expect((screen.getByRole('button', { name: 'Investigate' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('renders numeric backend ids, evidence, explanations, chains and downsampling disclosure', () => {
    const result = toInvestigationView(raw);
    expect(result.id).toBe(907);
    expect(result.suspects[0]?.event.id).toBe(3004);
    render(InvestigationsScreen, { data: ready([result]) });

    expect(screen.getByText('Confirmed')).toBeTruthy();
    expect(screen.getByText('This change directly affected API service.')).toBeTruthy();
    expect(screen.getByText('Changed a file that this service reads')).toBeTruthy();
    expect(screen.getByText('Port 4000 opened')).toBeTruthy();
    expect(screen.getByTestId('downsample-disclosure').textContent).toContain('5,000 most important changes');
    expect(screen.getByTestId('result-workspace').textContent).toContain('Unavailable');
  });

  it.each(Object.entries(NO_CAUSE_COPY))('renders exact no-cause copy for %s', (reason, copy) => {
    const result: InvestigationView = {
      ...toInvestigationView({ ...raw, id: 908, suspects: [], dropped_event_count: 0 }),
      noCauseReason: reason as NoCauseReason,
    };
    render(InvestigationsScreen, { data: ready([result]) });
    expect(screen.getByText(copy)).toBeTruthy();
  });

  it('switches between persisted investigation history entries', async () => {
    const first = toInvestigationView(raw);
    const second = toInvestigationView({ ...raw, id: 909, problem: 'wifi stopped working', suspects: [], no_cause_reason: 'NothingChanged' });
    render(InvestigationsScreen, { data: ready([first, second]) });

    await fireEvent.click(screen.getByRole('button', { name: /wifi stopped working/i }));
    expect(screen.getByRole('heading', { name: 'wifi stopped working' })).toBeTruthy();
    expect(screen.getByText(NO_CAUSE_COPY.nothing_changed)).toBeTruthy();
  });
});
