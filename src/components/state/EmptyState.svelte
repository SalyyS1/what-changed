<script lang="ts">
  import type { Snippet } from 'svelte';
  import UiEmptyState from '../ui/EmptyState.svelte';
  import type { IconName } from '../ui/ui-types';

  /**
   * The screen-level empty state `StateShell` renders. It exists as its own
   * component (rather than the ui primitive directly) so a screen can pass a
   * `reason` from the `AsyncState` value — "Recording started 4 minutes ago"
   * beats a generic "No events" — while the primitive stays context-free.
   */
  interface Props {
    title: string;
    description?: string | undefined;
    reason?: string | undefined;
    icon?: IconName | undefined;
    action?: Snippet | undefined;
  }

  let { title, description, reason, icon, action }: Props = $props();

  const text = $derived(reason ? (description ? `${description} ${reason}` : reason) : description);
</script>

<UiEmptyState {title} description={text} {icon} {action} />
