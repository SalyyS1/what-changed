<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';
  import type { AsyncState } from '../../lib/types/async-state';
  import type { IconName } from '../ui/ui-types';
  import EmptyState from './EmptyState.svelte';
  import ErrorState from './ErrorState.svelte';
  import LoadingBlock from './LoadingBlock.svelte';

  /**
   * Renders exactly one branch of an `AsyncState<T>`. This is the six-state
   * rule (§70) as a component: a screen hands over its state and its ready
   * snippet, and the loading, empty and error faces come from here — same
   * shapes, same copy style, on every screen. `large-data` and
   * `small-window` are layout concerns handled by virtualization and
   * container queries in the ready branch, not by this switch.
   */
  interface Props {
    state: AsyncState<T>;
    /** Copy for the empty face; the `reason` on the state value is appended. */
    emptyTitle: string;
    emptyDescription?: string | undefined;
    emptyIcon?: IconName | undefined;
    emptyAction?: Snippet | undefined;
    errorTitle?: string | undefined;
    loadingRows?: number | undefined;
    class?: string;
    children: Snippet<[T]>;
  }

  let {
    state,
    emptyTitle,
    emptyDescription,
    emptyIcon,
    emptyAction,
    errorTitle,
    loadingRows,
    class: className = '',
    children,
  }: Props = $props();
</script>

<div data-testid="state-shell" data-state={state.status} class="min-h-0 flex-1 {className}">
  {#if state.status === 'loading'}
    <LoadingBlock rows={loadingRows} />
  {:else if state.status === 'empty'}
    <EmptyState
      title={emptyTitle}
      description={emptyDescription}
      reason={state.reason}
      icon={emptyIcon}
      action={emptyAction}
    />
  {:else if state.status === 'error'}
    <ErrorState title={errorTitle} message={state.message} retry={state.retry} />
  {:else}
    {@render children(state.data)}
  {/if}
</div>
