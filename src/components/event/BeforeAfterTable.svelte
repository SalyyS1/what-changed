<script lang="ts">
  import { displayValue } from '../../lib/formatting/eventCopy';
  import type { JsonValue } from '../../lib/tauri/bindings';
  interface Props { before: JsonValue | null; after: JsonValue | null; }
  let { before, after }: Props = $props();
</script>

<div class="overflow-hidden rounded-lg border border-border-subtle">
  <table class="w-full table-fixed border-collapse text-sm">
    <thead class="bg-surface-2 text-left text-caps text-text-muted">
      <tr><th class="w-1/2 px-3 py-2 font-medium">Before</th><th class="w-1/2 px-3 py-2 font-medium">After</th></tr>
    </thead>
    <tbody>
      <tr class="border-t border-border-subtle align-top">
        <td class="selectable break-words px-3 py-2 font-mono text-xs text-text-secondary">{displayValue(before)}</td>
        <td class="selectable break-words border-l border-border-subtle px-3 py-2 font-mono text-xs text-text-primary">{displayValue(after)}</td>
      </tr>
    </tbody>
  </table>
</div>
