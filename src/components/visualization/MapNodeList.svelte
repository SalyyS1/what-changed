<script lang="ts">
  import type { ChangeMapNode } from '../../lib/tauri/bindings';

  interface Props {
    nodes: ChangeMapNode[];
    selectedId: number | null;
    onhover: (id: number | null) => void;
    onselect: (id: number, opener: HTMLElement) => void;
    onescape: () => void;
  }
  let { nodes, selectedId, onhover, onselect, onescape }: Props = $props();
  let active = $state(0);
  let search = '';
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  const start = $derived(Math.max(0, Math.min(active - 20, nodes.length - 41)));
  const visible = $derived(nodes.slice(start, start + 41));
  const activeId = $derived.by(() => {
    const node = nodes[active];
    return node ? `map-node-${node.event_id}` : undefined;
  });

  function move(next: number): void {
    active = Math.max(0, Math.min(nodes.length - 1, next));
    onhover(nodes[active]?.event_id ?? null);
  }

  function typeahead(key: string): void {
    search += key.toLocaleLowerCase();
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => (search = ''), 700);
    const found = nodes.findIndex((node, index) => index > active && `${node.lane} ${node.label}`.toLocaleLowerCase().startsWith(search));
    const wrapped = found >= 0 ? found : nodes.findIndex((node) => `${node.lane} ${node.label}`.toLocaleLowerCase().startsWith(search));
    if (wrapped >= 0) move(wrapped);
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') move(active + 1);
    else if (event.key === 'ArrowUp') move(active - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(nodes.length - 1);
    else if (event.key === 'PageDown') move(active + 10);
    else if (event.key === 'PageUp') move(active - 10);
    else if (event.key === 'Enter' || event.key === ' ') {
      const node = nodes[active];
      if (node) onselect(node.event_id, event.currentTarget as HTMLElement);
    } else if (event.key === 'Escape') onescape();
    else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      typeahead(event.key);
    } else return;
    event.preventDefault();
  }
</script>

<div class="group absolute bottom-3 left-3 z-10 max-w-[min(34rem,calc(100%-6rem))]">
  <div
    class="h-11 overflow-hidden rounded-md border border-border-subtle bg-surface-1/95 px-3 py-2 text-sm text-text-secondary shadow-md outline-none focus-within:h-auto focus-within:max-h-[30vh] focus-within:overflow-y-auto"
  >
    <div
      role="listbox"
      aria-label="Explore map nodes"
      aria-activedescendant={activeId}
      tabindex="0"
      class="outline-none"
      onkeydown={keydown}
      onfocus={() => onhover(nodes[active]?.event_id ?? null)}
      onblur={() => onhover(null)}
    >
      <p class="mb-2 text-xs text-text-muted">Use arrows to explore · Enter opens details · Escape returns to map controls</p>
      {#each visible as node, offset (node.event_id)}
        {@const index = start + offset}
        <div
          id={`map-node-${node.event_id}`}
          role="option"
          aria-selected={selectedId === node.event_id}
          aria-setsize={nodes.length}
          aria-posinset={index + 1}
          class="truncate rounded px-2 py-1 {index === active ? 'bg-accent/15 text-text-primary' : 'text-text-secondary'}"
        >
          {index + 1} of {nodes.length} · <span class="capitalize">{node.lane}</span> · {node.label}
        </div>
      {/each}
    </div>
  </div>
</div>
