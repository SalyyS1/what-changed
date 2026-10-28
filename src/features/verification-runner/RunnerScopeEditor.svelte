<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { RunnerRouteDto } from '../../lib/tauri/bindings';
  interface Props { routes: RunnerRouteDto[]; fixture: Record<string, string>; origins: string[]; disabled?: boolean; }
  let { routes = $bindable(), fixture = $bindable(), origins, disabled = false }: Props = $props();
  const methods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].map((value) => ({ value, label: value }));
</script>

<fieldset class="mt-4 space-y-4" {disabled}>
  <legend class="text-sm font-medium text-text-primary">Test data and exact request scope</legend>
  <p class="text-sm text-text-secondary">Use synthetic values only. Add every needed page, script, image and API route explicitly. A missing route blocks the attempt. Requests may change the test server, including GET requests.</p>
  {#each Object.keys(fixture) as field (field)}
    <label class="block text-sm text-text-secondary">Synthetic value for {field}<Input value={fixture[field] ?? ''} oninput={(event) => (fixture[field] = event.currentTarget.value)} maxlength={1000} autocomplete="off" /></label>
  {/each}
  <div class="space-y-3">
    {#each routes as route, index (index)}
      <fieldset class="space-y-2 rounded-md border border-border-subtle p-3" {disabled}>
        <legend class="px-1 text-xs text-text-secondary">Request {index + 1}</legend>
        <Select value={route.origin} options={origins.map((value) => ({ value, label: value }))} label={`Request ${index + 1} origin`} onchange={(value) => (routes[index]!.origin = value)} {disabled} />
        <div class="flex flex-wrap gap-2"><div class="w-28"><Select value={route.method} options={methods} label={`Request ${index + 1} method`} onchange={(value) => (routes[index]!.method = value)} {disabled} /></div><div class="min-w-48 flex-1"><Input value={route.path} oninput={(event) => (routes[index]!.path = event.currentTarget.value)} label={`Request ${index + 1} path`} placeholder="/api/bookings" maxlength={300} /></div><Button size="sm" variant="ghost" {disabled} onclick={() => (routes = routes.filter((_, position) => position !== index))}>Remove request {index + 1}</Button></div>
      </fieldset>
    {/each}
    <Button size="sm" variant="secondary" disabled={disabled || !origins.length || routes.length >= 150} onclick={() => (routes = [...routes, { origin: origins[0]!, method: 'GET', path: '/' }])}>Add exact request</Button>
    <p class="text-xs text-text-muted">No wildcards or query strings. A read-back path may contain one {'{fixture_id}'} slot. Login and cleanup requests need their own entries.</p>
  </div>
</fieldset>
