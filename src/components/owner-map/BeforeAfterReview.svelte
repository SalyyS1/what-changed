<script lang="ts">
  import type { OwnerEvidenceReviewDto } from '../../lib/tauri/bindings';
  import { compareOwnerRuns } from '../../lib/owner-graph/comparison';
  import { evidenceNodeId } from '../../lib/owner-graph/types';
  interface Props { reviews: readonly OwnerEvidenceReviewDto[]; oninspect: (runId: string, nodeId: string) => void; }
  let { reviews, oninspect }: Props = $props();
  let beforeId = $state('');
  let afterId = $state('');
  let filter = $state('all');
  const after = $derived(reviews.find((review) => review.run.id === afterId) ?? reviews[0]);
  const before = $derived(reviews.find((review) => review.run.id === beforeId) ?? reviews.find((review) => review.run.id !== after?.run.id));
  const comparisons = $derived(before && after ? compareOwnerRuns(before, after) : []);
  const rows = $derived(comparisons.filter((row) => filter === 'all' || filter === 'failed' && (row.before?.status === 'failed' || row.after?.status === 'failed') || filter === 'gaps' && (!row.comparable || [row.before?.status, row.after?.status].some((status) => status === 'not_run' || status === 'blocked')) || filter === 'stale' && (before?.freshness === 'stale' || after?.freshness === 'stale')));
  const label = (review: OwnerEvidenceReviewDto) => `${review.run.manifest.candidate.identity} · ${review.run.id} · ${review.freshness}`;
</script>

<details class="rounded-md border border-border-subtle p-3">
  <summary class="min-h-11 cursor-pointer py-3 text-sm font-medium text-text-primary">Before / after evidence review</summary>
  {#if reviews.length < 2}<p class="mt-2 text-sm text-text-secondary">Record two attempts to compare their frozen scenarios. A single attempt cannot establish a before/after change.</p>
  {:else}
    <div class="mt-2 grid gap-3 sm:grid-cols-2">
      <label class="min-w-0 text-xs text-text-secondary">Before<select class="mt-1 min-h-11 w-full rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary" value={before?.run.id ?? ''} onchange={(event) => { beforeId = event.currentTarget.value; }}>{#each reviews as review (review.run.id)}<option value={review.run.id}>{label(review)}</option>{/each}</select></label>
      <label class="min-w-0 text-xs text-text-secondary">After<select class="mt-1 min-h-11 w-full rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary" value={after?.run.id ?? ''} onchange={(event) => { afterId = event.currentTarget.value; }}>{#each reviews as review (review.run.id)}<option value={review.run.id}>{label(review)}</option>{/each}</select></label>
    </div>
    {#if before && after}
      <div class="mt-3 grid gap-3 sm:grid-cols-2">{#each [before, after] as review, index}<p class="rounded-md bg-surface-2 p-3 text-xs text-text-secondary">{index === 0 ? 'Before' : 'After'}: {review.counts.passed} passed · {review.counts.failed} failed · {review.counts.blocked} blocked · {review.counts.not_run} not run. Backend result: {review.status}; freshness: {review.freshness}. Environment {review.run.manifest.environment.label} r{review.run.manifest.environment.revision}; {review.run.method.replaceAll('_', ' ')}.</p>{/each}</div>
      <label class="mt-3 block text-xs text-text-secondary">Compare filter<select bind:value={filter} class="ml-2 min-h-11 rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary"><option value="all">All scenarios</option><option value="failed">Failures</option><option value="gaps">Gaps / incomparable</option><option value="stale">Stale</option></select></label>
      {#if before.run.id === after.run.id}<p class="mt-2 text-xs text-warning">Both sides name the same attempt.</p>{/if}
      <p class="mt-2 text-xs text-text-secondary">Comparable means the same scenario revision, rules, assertions and verification semantics. Build, environment and freshness remain separate context. Missing scenarios never become an improvement score.</p>
      <ul class="mt-3 space-y-3">{#each rows as row (row.id)}<li class="rounded-md border border-border-subtle p-3"><h4 class="text-sm font-medium text-text-primary">{row.after?.scenario.title ?? row.before?.scenario.title}</h4><p class="mt-1 text-xs {row.comparable ? 'text-text-secondary' : 'text-warning'}">{row.comparable ? 'Same frozen scenario and expectations' : `Incomparable: ${row.reasons.join(' ')}`}</p><div class="mt-2 grid gap-2 sm:grid-cols-2">{#each [{ evidence: row.before, review: before, side: 'Before' }, { evidence: row.after, review: after, side: 'After' }] as item}<div class="min-w-0 rounded-md bg-surface-2 p-2 text-xs text-text-secondary"><p>{item.side}: {item.evidence?.status.replaceAll('_', ' ') ?? 'Absent'} · scenario revision {item.evidence?.scenario.revision ?? 'absent'}</p>{#if item.evidence}<p class="mt-1 whitespace-pre-wrap">Expected: {item.evidence.scenario.expected_behavior}</p><p class="mt-1 break-words">Rules: {item.evidence.scenario.rules.map((ref) => `${ref.rule_id} r${ref.revision}`).join(', ') || 'none'}</p><p class="mt-1 whitespace-pre-wrap">Observed: {item.evidence.result?.assertions.map((assertion) => `${assertion.expected}: ${assertion.observed || 'No observation'}`).join('\n') || 'No observation recorded.'}</p><button type="button" class="mt-2 min-h-11 underline" onclick={() => oninspect(item.review.run.id, evidenceNodeId(item.review.run.id, item.evidence!.scenario.id, item.evidence!.scenario.revision))}>Inspect {item.side.toLowerCase()} evidence</button>{/if}</div>{/each}</div></li>{/each}</ul>
      {#if !rows.length}<p class="mt-2 text-sm text-text-secondary">No scenarios match this comparison filter.</p>{/if}
    {/if}
  {/if}
</details>
