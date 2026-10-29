<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  import { routeById } from '../../app/routes';
  import { investigation } from '../../lib/stores/investigation.svelte';
  import { router } from '../../lib/stores/router.svelte';

  let symptom = $state(investigation.draft);

  $effect(() => investigation.setDraft(symptom));

  function investigate(): void {
    const route = routeById('investigations');
    investigation.setDraft(symptom);
    if (route) router.navigate(route);
  }
</script>

<div>
  <p class="mb-3 font-display text-lg text-text-primary">What stopped working?</p>
  <div class="flex flex-col gap-3 @[720px]/shell:flex-row">
    <Input
      bind:value={symptom}
      label="Describe the symptom"
      placeholder="Example: Discord stopped detecting my microphone this morning"
      class="flex-1"
    />
    <Button variant="primary" disabled={symptom.trim() === ''} onclick={investigate}>Investigate</Button>
  </div>
</div>
