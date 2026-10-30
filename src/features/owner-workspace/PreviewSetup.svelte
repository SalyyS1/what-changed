<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { EnvironmentKind, OwnerEnvironmentRevisionDto } from '../../lib/tauri/bindings';
  import type { OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';
  import { environmentOrigins, originKey, previewUrl as parsePreviewUrl, urlOrigin } from './owner-origins';

  interface Props { environments: OwnerEnvironmentRevisionDto[]; model: OwnerWorkspaceStore; active?: boolean; }
  let { environments, model, active = true }: Props = $props();
  let label = $state('Test preview');
  let previewUrl = $state('');
  let additionalOrigins = $state('');
  let kind = $state<EnvironmentKind>('test');
  let previous = $state<OwnerEnvironmentRevisionDto | undefined>();
  let error = $state<string | null>(null);
  const latest = $derived(environments.filter((item) => !environments.some((other) => other.id === item.id && other.revision > item.revision)));
  const setupBrief = 'Prepare a safe test preview for this app. Use isolated test data and test accounts. Provide stable role/name/test-id labels, a revision marker at /.well-known/ownvero.json that changes with relevant deployments, and two ordinary test-role sessions. Configure reviewed, typed read-back and role-isolation adapters with exact approved origins. Document safe cleanup. Never use production data or paste credentials into Ownvero. This setup does not itself verify the app.';

  async function save(): Promise<void> {
    error = null;
    try {
      const url = parsePreviewUrl(previewUrl);
      const origins = environmentOrigins(url, additionalOrigins);
      if (await model.saveEnvironment({ label: label.trim(), preview_url: url.href, kind, revoked: false, origins }, previous)) { previewUrl = ''; additionalOrigins = ''; previous = undefined; }
      else error = model.error ?? 'The preview settings could not be saved.';
    } catch (reason) { error = reason instanceof Error ? reason.message : 'Use an HTTP or HTTPS test URL without credentials, query parameters or a fragment.'; }
  }
  function edit(environment: OwnerEnvironmentRevisionDto): void {
    previous = environment; label = environment.label; previewUrl = environment.preview_url; kind = environment.kind;
    const mainOrigin = originKey(urlOrigin(parsePreviewUrl(environment.preview_url)));
    additionalOrigins = environment.origins.filter((origin) => originKey(origin) !== mainOrigin).map(originKey).join('\n');
  }
</script>

<Card level={1}>
  <div class="flex flex-wrap items-center gap-2"><h2 class="font-medium text-text-primary">Safe preview setup</h2><Badge tone="neutral">Opt-in runner</Badge></div>
  <p class="mt-2 text-sm text-text-secondary">Saving a URL is offline configuration. It does not contact the preview, approve a run, or establish that any behavior passed.</p>
  <ul class="mt-3 space-y-1 text-sm text-text-secondary"><li>Test-data separation: not checked.</li><li>Revision marker and build identity: not checked.</li><li>Stable labels, test roles and read-back adapters: not configured here.</li><li>Automated verification runs only when this app is launched from a packaged candidate with its reviewed runner. A source or core build can still save this setup and record manual observations.</li></ul>
  <details class="mt-3 text-sm"><summary class="cursor-pointer text-text-primary">One-time builder setup brief</summary><p class="selectable mt-2 whitespace-pre-wrap text-text-secondary">{setupBrief}</p></details>
  {#if latest.length}<ul class="mt-4 space-y-2">{#each latest as environment (environment.id)}<li class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border-subtle p-3"><div class="min-w-0"><p class="text-sm text-text-primary">{environment.label} · revision {environment.revision} · {environment.revoked ? 'Revoked' : environment.kind}</p><p class="selectable break-all text-xs text-text-secondary">{environment.preview_url}</p></div><Button size="sm" variant="ghost" disabled={!active || model.saving} onclick={() => edit(environment)}>Edit preview</Button></li>{/each}</ul>{/if}
  <details class="mt-4" open={previous !== undefined}>
    <summary class="cursor-pointer text-sm font-medium text-text-primary">{previous ? 'Revise preview settings' : 'Add preview settings'}</summary>
    <div class="mt-3 space-y-3">
      <label class="block text-sm text-text-secondary">Preview name<Input bind:value={label} maxlength={120} /></label>
      <label class="block text-sm text-text-secondary">Authorized preview URL<Input bind:value={previewUrl} type="url" placeholder="http://localhost:3000/" /></label>
      <label class="block text-sm text-text-secondary">Additional exact approved origins<Textarea bind:value={additionalOrigins} rows={3} placeholder="https://api.example.test:443" /></label>
      <p class="text-xs text-text-secondary">The preview URL’s origin is included automatically. Add a separate API origin on its own line when needed, up to 16 distinct origins in total. Use scheme, host and port without a route path. These settings are saved offline and contact no target.</p>
      <Select value={kind} onchange={(value) => { if (value === 'test' || value === 'local') kind = value; }} label="Preview kind" options={[{ value: 'test', label: 'Isolated test environment' }, { value: 'local', label: 'Local preview' }]} />
      {#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}
      <Button size="sm" variant="secondary" disabled={!active || model.saving || !label.trim() || !previewUrl.trim()} onclick={() => void save()}>{model.saving ? 'Saving…' : 'Save offline settings'}</Button>
    </div>
  </details>
</Card>
