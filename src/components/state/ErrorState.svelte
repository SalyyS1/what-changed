<script lang="ts">
  import UiErrorState from '../ui/ErrorState.svelte';

  /**
   * Screen-level error state rendered by `StateShell`. Thin on purpose: the
   * message and retry come straight from the `AsyncState` error branch so a
   * screen cannot substitute a softer message for the real one.
   */
  interface Props {
    title?: string | undefined;
    message: string;
    retry?: (() => void) | undefined;
  }

  let { title, message, retry }: Props = $props();
</script>

<UiErrorState {title} {message} {retry} />
