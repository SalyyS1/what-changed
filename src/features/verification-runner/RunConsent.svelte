<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import type { PreviewRunPlanDto } from '../../lib/tauri/bindings';
  interface Props { open?: boolean; plan: PreviewRunPlanDto; busy?: boolean; onGrant: () => void; }
  let { open = $bindable(false), plan, busy = false, onGrant }: Props = $props();
  let authorized = $state(false); let isolated = $state(false); let temporaryAuth = $state(false);
</script>

<Dialog bind:open title="Allow this test session?" description="Permission applies only to this frozen scope. It expires after five minutes; changes require a new review." width="lg">
  <div class="space-y-4">
    <p class="text-sm text-text-primary">{plan.manifest.candidate.identity} · environment revision {plan.manifest.environment.revision} · request version {plan.manifest.change.version}</p>
    <p class="text-sm text-text-secondary">Test roles: {plan.roles.join(', ')}. Sign in visibly with separate test accounts. At most 10 scenarios, 50 steps per scenario, 30 seconds per action and 10 minutes per session.</p>
    <p class="text-sm text-text-secondary">Screenshots: {plan.capture_screenshots ? 'Explicit capture requests allowed; retain each image only after a separate review.' : 'Off for this session.'} Raw traces, HAR and video are off.</p>
    <ul class="list-inside list-disc space-y-1 text-sm text-text-secondary">{#each plan.effects as effect (effect)}<li>{effect}</li>{/each}</ul>
    <details open><summary class="cursor-pointer text-sm font-medium text-text-primary">Frozen scenarios and actions</summary><ul class="mt-2 space-y-3 text-sm text-text-secondary">{#each plan.manifest.scenarios as scenario (`${scenario.id}:${scenario.revision}`)}<li><strong>{scenario.title} · revision {scenario.revision}</strong><p>{scenario.expected_behavior}</p><ol class="mt-1 list-inside list-decimal">{#each scenario.recipe.steps as step (step.id)}<li>{step.action.kind.replaceAll('_', ' ')} · {step.assertions.length} named checks</li>{/each}</ol></li>{/each}</ul></details>
    <details open><summary class="cursor-pointer text-sm font-medium text-text-primary">Allowed network requests</summary><ul class="mt-2 space-y-1 break-all font-mono text-xs text-text-secondary">{#each plan.routes as route, index (index)}<li>{route.method} {route.origin}{route.path}</li>{/each}</ul></details>
    <ul class="list-inside list-disc space-y-1 text-sm text-warning">{#each plan.limitations as limitation (limitation)}<li>{limitation}</li>{/each}</ul>
    <div class="space-y-3 border-t border-border-subtle pt-3 text-sm text-text-secondary">
      <label class="flex items-start gap-2"><input type="checkbox" bind:checked={authorized} disabled={busy} />I own this test environment or have permission to run these exact actions.</label>
      <label class="flex items-start gap-2"><input type="checkbox" bind:checked={isolated} disabled={busy} />This uses isolated test accounts and data. I reviewed mutations, cleanup and external effects.</label>
      <label class="flex items-start gap-2"><input type="checkbox" bind:checked={temporaryAuth} disabled={busy} />I understand temporary profiles may contain login/cache data. Deleting them does not guarantee secure erasure.</label>
    </div>
  </div>
  {#snippet footer()}<Button variant="ghost" disabled={busy} onclick={() => (open = false)}>Keep reviewing</Button><Button variant="primary" disabled={busy || !authorized || !isolated || !temporaryAuth} onclick={onGrant}>{busy ? 'Starting…' : 'Allow this session'}</Button>{/snippet}
</Dialog>
