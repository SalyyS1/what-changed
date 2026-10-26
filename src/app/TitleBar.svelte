<script lang="ts">
  import { onMount } from 'svelte';
  import brandMark from '../../assets/brand/logo-mark.svg';
  import Badge from '../components/ui/Badge.svelte';
  import IconButton from '../components/ui/IconButton.svelte';
  import { theme } from '../lib/design/theme.svelte';
  import { collectors, isCollectorDegraded } from '../lib/stores/collectors.svelte';
  import { monitoring } from '../lib/stores/monitoring.svelte';
  import { windowControls } from '../lib/tauri/window-controls';

  /**
   * The application-drawn window frame. `decorations: false` means Windows
   * paints nothing, so this bar has to do everything a title bar does:
   * drag and double-click-to-maximize (both handled natively by Tauri on
   * `data-tauri-drag-region`), and the three buttons — all reachable by
   * keyboard. The status pill on the right is the one place monitoring
   * state is always visible.
   *
   * The close button's label changes when the tray could not be created:
   * closing then really exits, and a button that says "Close" while doing
   * something else is how a user loses a recorder they thought was running.
   */
  let maximized = $state(false);

  onMount(() => {
    let unlisten: (() => void) | null = null;
    windowControls.isMaximized().then((m) => (maximized = m));
    windowControls.onMaximizedChange((m) => (maximized = m)).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  const themeLabel = $derived(
    theme.mode === 'system'
      ? `Theme: follows Windows (${theme.resolved}). Switch to dark`
      : theme.mode === 'dark'
        ? 'Theme: dark. Switch to light'
        : 'Theme: light. Follow Windows',
  );
  const themeIcon = $derived(
    theme.mode === 'system' ? 'monitor' : theme.mode === 'dark' ? 'moon' : 'sun',
  );
  const closeLabel = $derived(
    monitoring.closeWouldExit ? 'Quit — the tray is unavailable' : 'Close to tray',
  );
  const globalLabel = $derived(
    !monitoring.resolved
      ? 'Starting'
      : monitoring.needsRecovery
      ? 'Recovery needed'
      : collectors.rows.some(isCollectorDegraded)
        ? 'Degraded'
        : monitoring.label,
  );
  const globalTone = $derived(
    globalLabel === 'Starting' ? 'neutral' : globalLabel === 'Recovery needed' ? 'danger' : globalLabel === 'Degraded' || globalLabel === 'Paused' ? 'warning' : 'success',
  );
</script>

<header
  data-tauri-drag-region
  data-testid="titlebar"
  class="flex h-(--titlebar-height) shrink-0 items-center border-b border-border-subtle bg-surface-1 select-none"
>
  <div data-tauri-drag-region class="flex items-center gap-2 pl-3 pr-2">
    <img src={brandMark} alt="" aria-hidden="true" class="h-4 w-4 shrink-0" draggable="false" />
    <span data-tauri-drag-region class="text-sm font-medium tracking-wide text-text-secondary">
      What Changed
    </span>
  </div>

  <div data-tauri-drag-region class="flex-1"></div>

  <div class="flex items-center gap-1 pr-1">
    <Badge tone={globalTone} dot>
      {globalLabel}
    </Badge>
    <span class="sr-only" aria-live="polite" aria-atomic="true">{collectors.announcement}</span>
    <IconButton icon={themeIcon} label={themeLabel} size="sm" onclick={() => theme.cycle()} />
    <span class="mx-1 h-4 w-px bg-border-subtle" aria-hidden="true"></span>
    <IconButton
      icon="minimize"
      label="Minimize"
      size="sm"
      class="rounded-none"
      onclick={() => windowControls.minimize()}
    />
    <IconButton
      icon={maximized ? 'restore' : 'maximize'}
      label={maximized ? 'Restore down' : 'Maximize'}
      size="sm"
      class="rounded-none"
      onclick={() => windowControls.toggleMaximize()}
    />
    <IconButton
      icon="close"
      label={closeLabel}
      size="sm"
      tone="danger"
      class="rounded-none"
      onclick={() => windowControls.close()}
    />
  </div>
</header>
