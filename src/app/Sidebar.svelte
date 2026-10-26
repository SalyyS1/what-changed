<script lang="ts">
  import Icon from '../components/ui/Icon.svelte';
  import KeyHint from '../components/ui/KeyHint.svelte';
  import { router } from '../lib/stores/router.svelte';
  import { ui } from '../lib/stores/ui.svelte';
  import { hrefFor, navRoutes } from './routes';

  /**
   * The seven destinations. Below 1040px of shell width this collapses to an
   * icon rail; the user can also pin it either way. Labels stay in the DOM
   * when collapsed (visually hidden) so the accessible name never changes
   * with the width. The active indicator is one element that moves, which is
   * what makes the transition read as "the selection slid" rather than "two
   * things flashed".
   */
  const activeIndex = $derived(navRoutes.findIndex((r) => router.isActive(r)));

  /** Width of the shell container below which the sidebar becomes a rail. */
  const RAIL_BREAKPOINT = 1040;

  let nav = $state<HTMLElement | null>(null);
  let narrow = $state(false);

  // Measure the parent (the shell container) rather than the viewport so the
  // same rule works inside the dev harness frames. jsdom has no
  // ResizeObserver; the sidebar then simply stays expanded.
  $effect(() => {
    const parent = nav?.parentElement;
    if (!parent || typeof ResizeObserver === 'undefined') {
      return;
    }
    const ro = new ResizeObserver(([entry]) => {
      narrow = (entry?.contentRect.width ?? Infinity) < RAIL_BREAKPOINT;
    });
    ro.observe(parent);
    return () => ro.disconnect();
  });

  const rail = $derived(ui.sidebarCollapsed ?? narrow);
</script>

<nav
  bind:this={nav}
  aria-label="Main"
  data-testid="sidebar"
  data-rail={rail ? '' : undefined}
  class="wc-sidebar relative flex shrink-0 flex-col self-stretch border-r border-border-subtle bg-surface-1 py-2"
>
  <ul class="wc-nav-list relative m-0 flex list-none flex-col p-0 px-2">
    {#if activeIndex >= 0}
      <span
        aria-hidden="true"
        class="wc-indicator pointer-events-none absolute left-0 w-0.5 rounded-r-full bg-accent"
        style="top: calc({activeIndex} * (var(--nav-row-height) + var(--nav-row-gap)))"
      ></span>
    {/if}
    {#each navRoutes as route (route.id)}
      {@const active = router.isActive(route)}
      <li>
        <a
          href={hrefFor(route)}
          aria-current={active ? 'page' : undefined}
          title={route.shortcut ? `${route.label} (Ctrl+${route.shortcut})` : route.label}
          class="wc-nav-item flex items-center gap-3 rounded-md px-2.5 text-base motion-fast transition-colors {active
            ? 'bg-surface-2 text-text-primary'
            : 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
        >
          <Icon name={route.icon} size={16} class="shrink-0" />
          <span class="wc-label flex-1 truncate">{route.label}</span>
          {#if route.shortcut}
            <KeyHint keys={`Ctrl+${route.shortcut}`} class="wc-label wc-hint opacity-0" />
          {/if}
        </a>
      </li>
    {/each}
  </ul>

  <div class="flex-1"></div>

  <p class="wc-label px-4 pb-2 text-xs leading-relaxed text-text-muted">
    Your history stays on this PC.
  </p>
</nav>

<style>
  .wc-sidebar {
    width: var(--sidebar-width);
    transition: width var(--t-base) var(--ease-standard);
  }
  .wc-nav-list {
    gap: var(--nav-row-gap);
  }
  .wc-nav-item {
    height: var(--nav-row-height);
  }
  .wc-indicator {
    height: var(--nav-row-height);
    transition: top var(--t-base) var(--ease-spring);
  }
  .wc-nav-item:hover :global(.wc-hint),
  .wc-nav-item:focus-visible :global(.wc-hint) {
    opacity: 1;
  }

  /*
   * Rail mode (`data-rail`): the user pinned it collapsed, or the shell is
   * below the breakpoint and the user has not pinned it open. Labels stay in
   * the DOM (visually hidden) so the accessible name never changes; the
   * shortcut hint is a child component, hence `:global`.
   */
  .wc-sidebar[data-rail] {
    width: var(--sidebar-rail-width);
  }
  .wc-sidebar[data-rail] :global(.wc-label) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }
  .wc-sidebar[data-rail] .wc-nav-item {
    justify-content: center;
    padding-inline: 0;
  }
</style>
