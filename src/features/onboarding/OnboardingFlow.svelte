<script lang="ts">
  import { onMount } from 'svelte';
  import Card from '../../components/ui/Card.svelte';
  import Button from '../../components/ui/Button.svelte';
  import { collectors } from '../../lib/stores/collectors.svelte';
  import { settings } from '../../lib/stores/settings.svelte';
  import { watchedLocations } from '../../lib/stores/watched-locations.svelte';
  import { completeOnboarding, setAutostart } from '../../lib/tauri/client';
  import type { CollectorStatusDto } from '../../lib/tauri/bindings';
  import { onboardingCopy } from './copy';

  interface Props {
    oncomplete?: () => void;
  }

  let { oncomplete }: Props = $props();
  let step = $state(0);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let startWithWindows = $state(true);
  // Runtime rows are intentionally disabled until consent is completed. Keep
  // the user's onboarding choices locally so the controls still show the
  // desired defaults instead of the fenced worker state.
  let choices = $state<Record<string, boolean>>({});

  const rows = $derived(collectors.rows);

  onMount(() => {
    void (async () => {
      await Promise.all([watchedLocations.load(), collectors.load(), settings.load()]);
      const persisted = settings.current?.collectors;
      if (persisted) choices = { ...persisted };
    })();
  });

  async function finish(): Promise<void> {
    saving = true;
    error = null;
    try {
      try { await setAutostart(startWithWindows); } catch { /* optional preference; finish remains available */ }
      await completeOnboarding();
      oncomplete?.();
    } catch {
      error = 'The tour could not be saved. You can try again.';
    } finally {
      saving = false;
    }
  }

  async function skip(): Promise<void> {
    saving = true;
    error = null;
    try {
      await completeOnboarding();
      oncomplete?.();
    } catch {
      error = 'The tour could not be saved. You can try again.';
    } finally {
      saving = false;
    }
  }

  async function toggleCollector(id: string, enabled: boolean): Promise<void> {
    choices[id] = enabled;
    if (!(await collectors.setEnabled(id, enabled))) {
      error = 'That collector could not be changed right now.';
    } else {
      error = null;
    }
  }

  function selected(row: CollectorStatusDto): boolean {
    return choices[row.id] ?? (row.state !== 'unavailable' && row.id !== 'docker' && row.id !== 'dev_env');
  }

  function next(): void {
    if (step < 2) step += 1;
    else void finish();
  }
</script>

<div class="flex min-h-full items-center justify-center bg-background px-6 py-10" data-testid="onboarding-flow">
  <Card level={2} class="w-full max-w-2xl" role="dialog" aria-labelledby="onboarding-title" aria-describedby="onboarding-body">
    <div class="mb-6 flex items-center justify-between gap-4">
      <p class="text-caps text-text-muted">Welcome · {step + 1} of 3</p>
      <Button variant="ghost" size="sm" disabled={saving} onclick={() => void skip()}>Skip tour</Button>
    </div>

    {#if step === 0}
      <h1 id="onboarding-title" class="text-2xl text-text-primary">{onboardingCopy.introTitle}</h1>
      <p id="onboarding-body" class="mt-3 text-base leading-relaxed text-text-secondary">{onboardingCopy.introBody}</p>
      <p class="mt-6 rounded-md bg-accent/10 px-4 py-3 text-sm text-text-primary">{onboardingCopy.promise}</p>
      <p class="mt-3 text-sm text-text-muted">No account, email address, telemetry or cloud connection.</p>
    {:else if step === 1}
      <h1 id="onboarding-title" class="text-2xl text-text-primary">What we watch</h1>
      <p id="onboarding-body" class="mt-3 text-sm text-text-secondary">{onboardingCopy.privacy}</p>
      <ul class="mt-5 max-h-72 space-y-2 overflow-auto" aria-label="Collectors">
        {#each rows as row (row.id)}
          <li class="rounded-md border border-border-subtle px-3 py-2">
            <label class="flex items-start gap-3">
              <input
                class="mt-1 size-4"
                type="checkbox"
                checked={selected(row)}
                disabled={row.state === 'unavailable' || row.requires_admin || row.descriptor.requires_admin || collectors.pending.has(row.id)}
                onchange={(event) => void toggleCollector(row.id, (event.currentTarget as HTMLInputElement).checked)}
              />
              <span class="min-w-0">
                <span class="block text-sm text-text-primary">{row.descriptor.display_name}</span>
                <span class="mt-1 block text-xs text-text-muted">{row.descriptor.what_it_watches} {row.descriptor.why_it_matters}</span>
              </span>
            </label>
          </li>
        {:else}
          <li class="text-sm text-text-muted">Collectors will appear after the local service starts.</li>
        {/each}
      </ul>
    {:else}
      <h1 id="onboarding-title" class="text-2xl text-text-primary">{onboardingCopy.foldersTitle}</h1>
      <p id="onboarding-body" class="mt-3 text-base leading-relaxed text-text-secondary">{onboardingCopy.foldersBody}</p>
      <p class="mt-4 text-sm text-text-muted">You can choose folders later from Settings. Nothing is required to finish.</p>
      <div class="mt-5 rounded-md border border-border-subtle px-3 py-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p class="m-0 text-sm text-text-primary">Watch a project folder</p>
            <p class="m-0 mt-1 text-xs text-text-muted">We record that a value changed, never the value itself.</p>
          </div>
          <Button
            variant="secondary"
            size="sm"
            disabled={watchedLocations.adding || watchedLocations.atLimit}
            onclick={() => void watchedLocations.chooseAndAdd()}
          >
            {watchedLocations.adding ? 'Adding…' : 'Choose folder'}
          </Button>
        </div>
        {#if watchedLocations.rows.length > 0}
          <ul class="mt-3 space-y-1 text-xs text-text-muted" aria-label="Selected project folders">
            {#each watchedLocations.rows as location (location.id)}
              <li class="selectable break-all">{location.path}</li>
            {/each}
          </ul>
        {/if}
        {#if watchedLocations.error}<p class="mt-2 text-xs text-danger" role="alert">{watchedLocations.error}</p>{/if}
      </div>
      <label class="mt-6 flex items-start gap-3 rounded-md border border-border-subtle px-3 py-3 text-sm text-text-primary">
        <input class="mt-1 size-4" type="checkbox" bind:checked={startWithWindows} />
        <span><strong>Start with Windows</strong><span class="mt-1 block text-xs text-text-muted">Launch quietly to the tray and keep watching locally.</span></span>
      </label>
    {/if}

    {#if error}<p class="mt-4 text-sm text-danger" role="alert">{error}</p>{/if}
    <div class="mt-8 flex justify-between gap-3">
      <Button variant="ghost" disabled={step === 0 || saving} onclick={() => step -= 1}>Back</Button>
      <Button variant="primary" disabled={saving} onclick={next}>{saving ? 'Saving…' : step === 2 ? 'Start watching' : 'Continue'}</Button>
    </div>
  </Card>
</div>
