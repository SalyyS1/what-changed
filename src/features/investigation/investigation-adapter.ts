import type {
  ChainStepView,
  ConfidenceClass,
  InvestigationView,
  InvestigationWindow,
  NoCauseReason,
  ScoreBreakdownView,
  SuspectView,
} from './investigation-types';

type RecordValue = Record<string, unknown>;

const record = (value: unknown): RecordValue =>
  typeof value === 'object' && value !== null ? value as RecordValue : {};
const text = (value: unknown, fallback = ''): string => typeof value === 'string' ? value : fallback;
const number = (value: unknown, fallback = 0): number => typeof value === 'number' && Number.isFinite(value) ? value : fallback;
const list = (value: unknown): unknown[] => Array.isArray(value) ? value : [];

function confidence(value: unknown): ConfidenceClass {
  const normalized = text(value).toLowerCase();
  if (normalized === 'confirmed' || normalized === 'high') return 'confirmed';
  if (normalized === 'likely' || normalized === 'medium') return 'likely';
  return 'correlated';
}

function windowValue(value: unknown, from: number, to: number): InvestigationWindow {
  const candidate = record(value);
  const kind = text(candidate.kind);
  switch (kind) {
    case 'last_hour': return { kind };
    case 'today': return { kind };
    case 'since_yesterday': return { kind };
    case 'since_last_restart': return { kind };
    case 'since_checkpoint': return { kind, id: number(candidate.id ?? candidate.checkpoint_id) };
    case 'custom': return { kind, from: number(candidate.from ?? candidate.from_ms, from), to: number(candidate.to ?? candidate.to_ms, to) };
    default: return { kind: 'custom', from, to };
  }
}

function score(value: unknown, fallback: number): ScoreBreakdownView {
  const item = record(value);
  return {
    total: number(item.total, fallback),
    temporal: number(item.temporal),
    relationship: number(item.relationship),
    category: number(item.category),
    severity: number(item.importance ?? item.severity),
    rarity: number(item.rarity),
    failureProximity: number(item.failure_proximity ?? item.failureProximity),
    chainBonus: number(item.chain_bonus ?? item.chainBonus),
  };
}

function chainStep(value: unknown, index: number): ChainStepView {
  const item = record(value);
  const event = record(item.event);
  return {
    id: number(item.id ?? item.event_id ?? list(item.event_ids)[0] ?? event.id, index),
    title: text(item.title ?? item.label ?? item.summary ?? event.user_friendly_summary, `Related change ${index + 1}`),
    timestamp: typeof (item.timestamp ?? item.timestamp_ms ?? event.timestamp) === 'number' ? number(item.timestamp ?? item.timestamp_ms ?? event.timestamp) : null,
  };
}

function suspect(value: unknown): SuspectView {
  const item = record(value);
  const event = record(item.event);
  const explanation = record(item.explanation);
  const evidence = confidence(item.evidence ?? item.confidence_class ?? item.confidence);
  const entity = text(event.entity_display_name ?? item.entity, 'this item');
  const legacyExplanation = text(item.explanation);
  const defaultSentence = evidence === 'confirmed'
    ? `This change directly affected ${entity}.`
    : evidence === 'likely'
      ? `This change likely affected ${entity}.`
      : 'This changed around the same time.';
  return {
    event: {
      id: number(event.id ?? item.event_id),
      timestamp: number(item.timestamp_ms ?? event.timestamp ?? item.timestamp),
      title: text(event.user_friendly_summary ?? item.title, 'Recorded change'),
      entity,
    },
    evidence,
    score: score(item.breakdown ?? item.score_breakdown ?? item.score, number(item.score)),
    explanation: {
      confidenceSentence: text(item.confidence_text ?? explanation.confidence_sentence, defaultSentence),
      relativeTime: text(item.relative_time ?? explanation.relative_time),
      whyRanked: list(item.why_ranked ?? explanation.why_ranked).map((entry) => text(entry)).filter(Boolean),
      remediation: text(item.what_to_try ?? explanation.remediation, legacyExplanation || 'Compare against a checkpoint from when this worked.'),
    },
    chain: list(item.chain_steps ?? record(item.chain).steps ?? item.chain).map(chainStep),
  };
}

function noCause(value: unknown): NoCauseReason | null {
  const normalized = text(value).toLowerCase();
  switch (normalized) {
    case 'nothingchanged':
    case 'nothing_changed': return 'nothing_changed';
    case 'onlyroutinechanges':
    case 'only_routine_changes': return 'only_routine_changes';
    case 'windowtoonarrow':
    case 'window_too_narrow': return 'window_too_narrow';
    case 'notwatchingyet':
    case 'not_watching_yet': return 'not_watching_yet';
    case 'norecordedevidenceinworkspace':
    case 'no_recorded_evidence_in_workspace': return 'no_recorded_evidence_in_workspace';
    default: return null;
  }
}

/** One deliberate seam between generated Rust DTOs and presentation data. */
export function toInvestigationView(value: unknown): InvestigationView {
  const item = record(value);
  const resolved = record(item.resolved_window);
  const from = number(item.from ?? resolved.from ?? resolved.from_ms);
  const to = number(item.to ?? resolved.to ?? resolved.to_ms);
  const drop = record(item.drop ?? item.downsample);
  return {
    id: number(item.id),
    projectId: typeof item.project_id === 'string' ? item.project_id : null,
    problem: text(item.problem),
    window: windowValue(item.window, from, to),
    from,
    to,
    suspects: list(item.suspects).map(suspect),
    totalEvents: number(item.total_events_in_window ?? item.total),
    createdAt: number(item.created_at),
    droppedEvents: number(item.dropped_events ?? item.dropped_event_count ?? drop.dropped),
    noCauseReason: noCause(item.no_cause_reason),
  };
}
