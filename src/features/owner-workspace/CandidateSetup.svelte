<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { OwnerCandidateRevisionDto, OwnerEnvironmentRevisionDto } from '../../lib/tauri/bindings';
  import type { OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';
  interface Props { environments: OwnerEnvironmentRevisionDto[]; candidates: OwnerCandidateRevisionDto[]; model: OwnerWorkspaceStore; active?: boolean; }
  let { environments, candidates, model, active = true }: Props = $props();
  let environmentId = $state('');
  let identity = $state('');
  let message = $state<string | null>(null);
  const latest = $derived(environments.filter((item) => !item.revoked && !environments.some((other) => other.id === item.id && other.revision > item.revision)));
  const options = $derived(latest.map((item) => ({ value: `${item.id}:${item.revision}`, label: `${item.label} · revision ${item.revision}` })));
  async function save(): Promise<void> {
    const environment = latest.find((item) => `${item.id}:${item.revision}` === environmentId);
    if (!environment || !identity.trim()) { message = 'Choose a preview and give this candidate a recognizable label.'; return; }
    if (await model.saveCandidate({ environment_id: environment.id, environment_revision: environment.revision, identity: identity.trim(), source: 'owner_label', trust: 'self_reported', observed_at: Date.now() })) { identity = ''; message = 'Candidate label saved. Its deployed identity has not been verified.'; }
    else message = model.error ?? 'The candidate could not be saved.';
  }
</script>

<Card level={1}>
  <h2 class="font-medium text-text-primary">Candidate to review</h2>
  <p class="mt-1 text-sm text-text-secondary">A label identifies the version you intend to check. It does not prove what is deployed.</p>
  {#if candidates.length}<ul class="mt-3 space-y-2">{#each candidates as candidate (candidate.id)}<li class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border-subtle p-3"><div><p class="break-words text-sm text-text-primary">{candidate.identity}</p><div class="mt-1 flex flex-wrap gap-2"><Badge tone="neutral">{candidate.trust === 'self_reported' ? 'Owner label · unverified' : candidate.trust.replaceAll('_', ' ')}</Badge>{#if model.current?.app.current_candidate_id === candidate.id}<Badge tone="neutral">Selected candidate</Badge>{/if}</div></div><Button size="sm" variant="secondary" disabled={!active || model.saving || model.current?.app.current_candidate_id === candidate.id} onclick={() => void model.selectCandidate(candidate.id)}>Select candidate</Button></li>{/each}</ul>{/if}
  {#if options.length}
    <details class="mt-4"><summary class="cursor-pointer text-sm font-medium text-text-primary">Add a candidate label</summary><div class="mt-3 space-y-3"><Select value={environmentId} onchange={(value) => (environmentId = value)} {options} label="Candidate preview revision" /><label class="block text-sm text-text-secondary">Version or build label<Input bind:value={identity} maxlength={256} placeholder="Booking reminder change, first build" /></label><Button size="sm" variant="secondary" disabled={!active || model.saving} onclick={() => void save()}>Save candidate label</Button></div></details>
  {:else}<p class="mt-3 text-sm text-text-muted">Save a test preview first, then add a candidate label.</p>{/if}
  {#if message}<p class="mt-3 text-sm text-text-secondary" role="status">{message}</p>{/if}
</Card>
