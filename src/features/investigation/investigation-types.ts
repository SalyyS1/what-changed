import type { AsyncState } from '../../lib/types/async-state';

export type ConfidenceClass = 'confirmed' | 'likely' | 'correlated';

export type InvestigationWindow =
  | { kind: 'last_hour' }
  | { kind: 'today' }
  | { kind: 'since_yesterday' }
  | { kind: 'since_last_restart' }
  | { kind: 'since_checkpoint'; id: number }
  | { kind: 'custom'; from: number; to: number };

export interface InvestigationEventView {
  id: number;
  timestamp: number;
  title: string;
  entity: string;
}

export interface ScoreBreakdownView {
  total: number;
  temporal: number;
  relationship: number;
  category: number;
  severity: number;
  rarity: number;
  failureProximity: number;
  chainBonus: number;
}

export interface ExplanationView {
  confidenceSentence: string;
  relativeTime: string;
  whyRanked: string[];
  remediation: string;
}

export interface ChainStepView {
  id: number;
  title: string;
  timestamp: number | null;
}

export interface SuspectView {
  event: InvestigationEventView;
  evidence: ConfidenceClass;
  score: ScoreBreakdownView;
  explanation: ExplanationView;
  chain: ChainStepView[];
}

export type NoCauseReason =
  | 'nothing_changed'
  | 'only_routine_changes'
  | 'window_too_narrow'
  | 'not_watching_yet'
  | 'no_recorded_evidence_in_workspace';

export interface InvestigationView {
  id: number;
  projectId: string | null;
  problem: string;
  window: InvestigationWindow;
  from: number;
  to: number;
  suspects: SuspectView[];
  totalEvents: number;
  createdAt: number;
  droppedEvents: number;
  noCauseReason: NoCauseReason | null;
}

export type InvestigationScreenState = AsyncState<InvestigationView[]>;

export const NO_CAUSE_COPY: Record<NoCauseReason, string> = {
  nothing_changed: 'Nothing meaningful changed in this window.',
  only_routine_changes: 'Only routine changes here — updates and background tasks.',
  window_too_narrow: 'Try widening the time range.',
  not_watching_yet: 'We started watching after this window. Try a more recent range.',
  no_recorded_evidence_in_workspace: 'No recorded evidence was associated with this workspace in this window.',
};
