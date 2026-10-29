<script lang="ts">
  import type { Snippet } from 'svelte';
  import DetailsDrawer from '../components/event/DetailsDrawer.svelte';
  import SearchPalette from '../components/search/SearchPalette.svelte';
  import Sidebar from './Sidebar.svelte';
  import TitleBar from './TitleBar.svelte';
  import ToastRegion from './ToastRegion.svelte';
  import DegradedBanner from '../components/feedback/DegradedBanner.svelte';

  /**
   * Titlebar over sidebar-plus-content. The content column is the container
   * named `shell`, which is what the sidebar's collapse query and every
   * screen's small-window layout measure against — the window, not the
   * viewport, because inside a webview those are the same thing and the
   * container query is the one that also works inside the dev harness frame.
   */
  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<div class="flex h-full w-full flex-col bg-background text-text-primary">
  <TitleBar />
  <DegradedBanner />
  <div class="@container/shell flex min-h-0 flex-1">
    <Sidebar />
    <main
      id="main"
      tabindex="-1"
      data-testid="main"
      class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden outline-none"
    >
      {@render children()}
    </main>
  </div>
  <DetailsDrawer />
  <SearchPalette />
  <ToastRegion />
</div>
