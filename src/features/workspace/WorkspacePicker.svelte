<script lang="ts">
  import Select from '../../components/ui/Select.svelte';
  import { projects, type ProjectsStore } from '../../lib/stores/projects.svelte';

  interface Props {
    model?: ProjectsStore;
    disabled?: boolean;
    onchange?: (projectId: string | null) => void;
  }

  let { model = projects, disabled = false, onchange }: Props = $props();
  const options = $derived([
    { value: '', label: 'All recorded changes' },
    ...model.rows.map((project) => ({
      value: project.id,
      label: project.baseline_checkpoint_id === null ? `${project.name} · needs baseline` : project.name,
    })),
  ]);

  function change(value: string): void {
    const projectId = value || null;
    model.select(projectId);
    onchange?.(projectId);
  }
</script>

<Select
  label="Recovery workspace"
  value={model.selectedId ?? ''}
  {options}
  {disabled}
  onchange={change}
  class="min-w-52"
/>
