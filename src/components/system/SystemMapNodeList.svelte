<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { SystemMapNode } from '../../lib/tauri/bindings';
  import { entityKindName } from '../../features/system-map/copy';

  interface Props {
    nodes: SystemMapNode[];
    selectedId: string | null;
    onhover: (id: string | null) => void;
    onfocus: (node: SystemMapNode) => void;
    onescape: () => void;
  }
  let { nodes, selectedId, onhover, onfocus, onescape }: Props = $props();
  let active = $state(0);
  let search = '';
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let previousNodes: SystemMapNode[] | null = null;
  const start = $derived(Math.max(0, Math.min(active - 20, nodes.length - 41)));
  const visible = $derived(nodes.slice(start, start + 41));
  const activeId = $derived(nodes[active] ? `system-node-${active}` : undefined);

  $effect.pre(() => {
    const current = nodes;
    if (current === previousNodes) return;
    const previousId = previousNodes?.[active]?.entity_id;
    previousNodes = current;
    if (current.length === 0) {
      active = 0;
    } else {
      const retained = previousId ? current.findIndex((node) => node.entity_id === previousId) : -1;
      active = retained >= 0 ? retained : Math.min(active, current.length - 1);
    }
    search = '';
    if (searchTimer) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
  });

  function move(next: number): void {
    active = Math.max(0, Math.min(nodes.length - 1, next));
    onhover(nodes[active]?.entity_id ?? null);
  }
  function typeahead(key: string): void {
    search += key.toLocaleLowerCase();
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => (search = ''), 700);
    const label = (node: SystemMapNode) => `${node.entity_type} ${node.display_name}`.toLocaleLowerCase();
    const found = nodes.findIndex((node, index) => index > active && label(node).startsWith(search));
    const wrapped = found >= 0 ? found : nodes.findIndex((node) => label(node).startsWith(search));
    if (wrapped >= 0) move(wrapped);
  }
  function keydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') move(active + 1);
    else if (event.key === 'ArrowUp') move(active - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(nodes.length - 1);
    else if (event.key === 'PageDown') move(active + 10);
    else if (event.key === 'PageUp') move(active - 10);
    else if (event.key === 'Enter' || event.key === ' ') { const node = nodes[active]; if (node) onfocus(node); }
    else if (event.key === 'Escape') onescape();
    else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) typeahead(event.key);
    else return;
    event.preventDefault();
  }

  onDestroy(() => { if (searchTimer) clearTimeout(searchTimer); });
</script>

<div class="group absolute bottom-3 left-3 z-10 max-w-[min(32rem,calc(100%-1.5rem))]">
  <div class="h-11 overflow-hidden rounded-md border border-border-subtle bg-surface-1/95 px-3 py-2 text-sm text-text-secondary shadow-md outline-none focus-within:h-auto focus-within:max-h-[30vh] focus-within:overflow-y-auto">
    <div role="listbox" aria-label="Explore system map nodes" aria-activedescendant={activeId} tabindex="0" class="outline-none" onkeydown={keydown} onfocus={() => onhover(nodes[active]?.entity_id ?? null)} onblur={() => onhover(null)}>
      <p class="mb-2 text-xs text-text-muted">Use arrows to explore · Enter focuses · Escape returns to map controls</p>
      {#each visible as node, offset (node.entity_id)}
        {@const index = start + offset}
        <div id={`system-node-${index}`} role="option" aria-selected={selectedId === node.entity_id} aria-setsize={nodes.length} aria-posinset={index + 1} class="truncate rounded px-2 py-1 {index === active ? 'bg-accent/15 text-text-primary' : 'text-text-secondary'}">
          {index + 1} of {nodes.length} · {entityKindName(node.entity_type)} · {node.display_name}
        </div>
      {/each}
    </div>
  </div>
</div>
