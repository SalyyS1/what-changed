<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import type { OwnerGraphNode } from '../../lib/owner-graph/types';
  interface Props { nodes: readonly OwnerGraphNode[]; selectedId: string | null; onselect: (id: string) => void; onescape: () => void; }
  let { nodes, selectedId, onselect, onescape }: Props = $props();
  const instance = $props.id();
  let active = $state(0);
  let list: HTMLDivElement;
  let previous: readonly OwnerGraphNode[] = [];
  let search = '';
  let timer: ReturnType<typeof setTimeout> | undefined;
  let revision = 0;
  const start = $derived(Math.max(0, Math.min(active - 20, nodes.length - 41)));
  const visible = $derived(nodes.slice(start, start + 41));
  const activeId = $derived(nodes[active] ? `${instance}-owner-node-${active}` : undefined);
  $effect.pre(() => {
    const next = nodes;
    if (next === previous) return;
    const retained = previous[active]?.id;
    previous = next;
    const index = next.findIndex((node) => node.id === (selectedId ?? retained));
    active = index >= 0 ? index : Math.max(0, Math.min(active, next.length - 1));
    search = ''; if (timer) clearTimeout(timer);
  });
  $effect(() => {
    const index = nodes.findIndex((node) => node.id === selectedId);
    if (index >= 0) active = index;
  });
  $effect(() => { activeId; void reveal(); });
  async function reveal(): Promise<void> {
    const request = ++revision;
    await tick();
    if (request !== revision || !list?.isConnected || document.activeElement !== list || !activeId) return;
    const option = document.getElementById(activeId);
    if (!option) return;
    const viewport = list.getBoundingClientRect(); const rect = option.getBoundingClientRect();
    if (rect.top < viewport.top) list.scrollTop -= viewport.top - rect.top;
    else if (rect.bottom > viewport.bottom) list.scrollTop += rect.bottom - viewport.bottom;
  }
  function move(index: number): void { active = Math.max(0, Math.min(nodes.length - 1, index)); const node = nodes[active]; if (node) onselect(node.id); void reveal(); }
  function keydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') move(active + 1);
    else if (event.key === 'ArrowUp') move(active - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(nodes.length - 1);
    else if (event.key === 'PageDown') move(active + 10);
    else if (event.key === 'PageUp') move(active - 10);
    else if (event.key === 'Escape') onescape();
    else if (event.key === 'Enter' || event.key === ' ') move(active);
    else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      search += event.key.toLocaleLowerCase(); if (timer) clearTimeout(timer);
      timer = setTimeout(() => { search = ''; }, 700);
      const matches = (node: OwnerGraphNode) => node.label.toLocaleLowerCase().startsWith(search) || `${node.kind} ${node.label}`.toLocaleLowerCase().startsWith(search);
      const next = nodes.findIndex((node, index) => index > active && matches(node));
      const wrapped = next < 0 ? nodes.findIndex(matches) : next;
      if (wrapped >= 0) move(wrapped);
    } else return;
    event.preventDefault();
  }
  onDestroy(() => { ++revision; if (timer) clearTimeout(timer); });
</script>

<div class="space-y-2">
  <p class="text-xs text-text-secondary">{nodes.length} matching nodes. Arrows, Home, End, Page Up/Down or type a name to select. Escape returns to view controls.</p>
  <div bind:this={list} role="listbox" aria-label="Capability and evidence nodes" aria-activedescendant={activeId} tabindex="0" onkeydown={keydown} onfocus={() => void reveal()} class="max-h-80 min-h-20 overflow-y-auto rounded-md border border-border-subtle p-2 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
    {#each visible as node, offset (node.id)}
      <div id={`${instance}-owner-node-${start + offset}`} role="option" tabindex="-1" aria-selected={selectedId === node.id} aria-posinset={start + offset + 1} aria-setsize={nodes.length} class="min-h-11 cursor-pointer rounded-md border p-2 {selectedId === node.id ? 'border-border-active bg-surface-3' : 'border-transparent hover:bg-surface-2'}" onclick={() => move(start + offset)} onkeydown={undefined}>
        <span class="block break-words text-sm text-text-primary">{node.label}</span><span class="block text-xs text-text-secondary">{node.kind} {node.revision !== null ? `· revision ${node.revision}` : ''}{node.status ? ` · ${node.status.replaceAll('_', ' ')}` : ''}{node.freshness ? ` · ${node.freshness}` : ''}{node.selectedChange ? ' · linked to selected request' : ''}{node.gaps.length ? ` · ${node.gaps.length} gaps or limitations` : ''}</span>
      </div>
    {/each}
    {#if !nodes.length}<p class="p-3 text-sm text-text-secondary">No nodes match these filters.</p>{/if}
  </div>
</div>
