<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { TimelineSnapshot } from '../../lib/stores/timeline.svelte';
  import TimelineList from '../../components/timeline/TimelineList.svelte';
  import ChangeMapFrame from '../../components/visualization/ChangeMapFrame.svelte';
  import SystemMapFrame from '../../components/system/SystemMapFrame.svelte';
  import { ready } from '../../lib/types/async-state';
  import { changeMapDevRequest, changeMapSample } from './change-map-dev-fixture';
  import { performanceTimeline, type PerfCorpus } from './perf-timeline-fixture';
  import { systemMapSample } from './system-map-dev-fixture';

  let corpus = $state<PerfCorpus>(100_000);
  let snapshot = $state<TimelineSnapshot | null>(null);
  let measuredFrames = $state<number | null>(null);
  let frameP95 = $state<number | null>(null);
  let blankFrames = $state<number | null>(null);
  let maxObservedIndex = $state<number | null>(null);
  let systemMapMedian = $state<number | null>(null);
  let measuring = $state(false);
  let timelineHost: HTMLDivElement;
  let frameHandle: number | null = null;

  const frameBudgetMs = 20;
  const targetFrames = 600;
  const measurementPassed = $derived(measuredFrames === targetFrames
    && frameP95 !== null && frameP95 <= frameBudgetMs
    && blankFrames === 0
    && maxObservedIndex !== null && maxObservedIndex >= corpus * 0.95);

  function loadCorpus(): void {
    snapshot = performanceTimeline(corpus);
  }

  function percentile(samples: number[], rank: number): number {
    const ordered = [...samples].sort((a, b) => a - b);
    const index = Math.max(0, Math.ceil((ordered.length * rank) / 100) - 1);
    return ordered[index] ?? 0;
  }

  function stopMeasurement(): void {
    if (frameHandle !== null) cancelAnimationFrame(frameHandle);
    frameHandle = null;
    measuring = false;
  }

  function measureScroll(): void {
    stopMeasurement();
    const viewport = timelineHost?.querySelector<HTMLDivElement>('[data-testid="timeline-viewport"]');
    if (!viewport || !snapshot) return;

    measuredFrames = null;
    frameP95 = null;
    blankFrames = null;
    maxObservedIndex = null;
    const scrollRange = viewport.scrollHeight - viewport.clientHeight;
    if (scrollRange <= 0) {
      measuredFrames = 0;
      return;
    }

    measuring = true;
    const samples: number[] = [];
    let blanks = 0;
    let furthest = 0;
    let last = 0;
    let segment = 0;
    let segmentFrame = 0;
    const segmentStarts = [0, 0.5, 0.95] as const;
    const framesPerSegment = targetFrames / segmentStarts.length;
    const scrollStepPx = 37;
    const tick = (now: number): void => {
      samples.push(now - last);
      last = now;

      // Mounted rows may be far from the new scroll position. Count only
      // rows intersecting the viewport so a blank virtual canvas cannot pass.
      const visible = viewport.getBoundingClientRect();
      const rows = viewport.querySelectorAll<HTMLElement>('[data-index]');
      let eventVisible = false;
      for (const row of rows) {
        const bounds = row.getBoundingClientRect();
        if (bounds.bottom <= visible.top || bounds.top >= visible.bottom) continue;
        furthest = Math.max(furthest, Number(row.dataset.index ?? 0));
        if (row.querySelector('[data-testid="timeline-row"]')) eventVisible = true;
      }
      if (!eventVisible) blanks += 1;

      segmentFrame += 1;
      if (samples.length === targetFrames) {
        measuredFrames = samples.length;
        frameP95 = percentile(samples, 95);
        blankFrames = blanks;
        maxObservedIndex = furthest;
        measuring = false;
        frameHandle = null;
        return;
      }

      if (segmentFrame === framesPerSegment) {
        segment += 1;
        segmentFrame = 0;
        beginSegment();
      } else {
        viewport.scrollTop = Math.min(scrollRange, scrollRange * segmentStarts[segment]! + segmentFrame * scrollStepPx);
        frameHandle = requestAnimationFrame(tick);
      }
    };
    const beginSegment = (): void => {
      // Reposition between timed segments, then allow the virtualizer two
      // frames to paint before sampling continuous 37 px/frame scrolling.
      viewport.scrollTop = scrollRange * segmentStarts[segment]!;
      frameHandle = requestAnimationFrame(() => {
        frameHandle = requestAnimationFrame((now) => {
          last = now;
          frameHandle = requestAnimationFrame(tick);
        });
      });
    };
    beginSegment();
  }

  function setCorpus(value: string): void {
    stopMeasurement();
    corpus = value === '10000' ? 10_000 : 100_000;
    measuredFrames = null;
    frameP95 = null;
    blankFrames = null;
    maxObservedIndex = null;
    loadCorpus();
  }

  onMount(() => {
    loadCorpus();
    return stopMeasurement;
  });

  onDestroy(stopMeasurement);
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5" data-testid="perf-harness">
  <header class="mb-4 flex flex-wrap items-start justify-between gap-3">
    <div>
      <h1 class="font-display text-xl text-text-primary">Performance harness</h1>
      <p class="mt-1 text-sm text-text-secondary">Development-only measurements on the production TimelineList and map renderers.</p>
    </div>
    <div class="flex items-center gap-2">
      <label class="text-sm text-text-secondary" for="perf-corpus">Corpus</label>
      <select id="perf-corpus" class="rounded border border-border-active bg-surface-1 px-2 py-1 text-sm" value={corpus} onchange={(event) => setCorpus((event.currentTarget as HTMLSelectElement).value)}>
        <option value="10000">10,000 events</option>
        <option value="100000">100,000 events</option>
      </select>
      <button class="rounded border border-border-active px-3 py-1 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent" type="button" disabled={measuring || !snapshot} onclick={measureScroll}>
        {measuring ? 'Measuring…' : 'Measure 600 frames'}
      </button>
    </div>
  </header>

  <div class="mb-3 grid gap-3 text-sm @[720px]/shell:grid-cols-3">
    <div class="rounded border border-border-subtle bg-surface-1 px-3 py-2" data-testid="perf-corpus-size">
      <span class="block text-xs text-text-muted">Loaded synthetic rows</span>
      <strong class="font-tabular text-text-primary">{snapshot?.items.length.toLocaleString('en-US') ?? 'Loading…'} events</strong>
    </div>
    <div class="rounded border border-border-subtle bg-surface-1 px-3 py-2" data-testid="perf-frame-p95">
      <span class="block text-xs text-text-muted">Scroll p95</span>
      <strong class="font-tabular text-text-primary">{frameP95 === null ? 'Not measured' : `${frameP95.toFixed(2)} ms`}</strong>
    </div>
    <div class="rounded border border-border-subtle bg-surface-1 px-3 py-2" data-testid="perf-frame-status">
      <span class="block text-xs text-text-muted">Budget (p95 ≤ {frameBudgetMs} ms)</span>
      <strong class="font-tabular {measurementPassed ? 'text-success' : 'text-text-primary'}">{measuredFrames === null ? 'Pending' : measurementPassed ? 'Pass' : 'Review'}</strong>
    </div>
  </div>

  <div class="mb-3 text-xs text-text-muted" data-testid="perf-scroll-coverage" aria-live="polite">
    {#if measuredFrames !== null}
      {measuredFrames} frame intervals · {blankFrames ?? 0} blank frames · furthest virtual row {(maxObservedIndex ?? 0).toLocaleString()}
    {:else}
      Three 200-frame scroll segments sample the start, middle and late history while checking visible rows.
    {/if}
  </div>

  <div bind:this={timelineHost} class="flex h-[360px] shrink-0 flex-col overflow-hidden rounded-lg border border-border-active bg-background" data-testid="perf-timeline-host">
    {#if snapshot}
      <TimelineList {snapshot} onopen={() => {}} onexpand={() => {}} />
    {/if}
  </div>

  <section class="mt-6 shrink-0" aria-labelledby="change-map-perf-heading">
    <h2 id="change-map-perf-heading" class="font-display text-lg text-text-primary">Change Map renderer</h2>
    <p class="mt-1 mb-3 text-sm text-text-secondary">3,000-node fixture; the renderer reports its 300-frame median in development builds.</p>
    <div class="h-[520px] overflow-hidden rounded-lg border border-border-active bg-background">
      <ChangeMapFrame
        request={changeMapDevRequest}
        subtitle="3,000-node performance fixture"
        onclose={() => {}}
        data={ready(changeMapSample(3000))}
        mode="3d"
        quality="low"
      />
    </div>
  </section>

  <section class="mt-6 shrink-0" aria-labelledby="system-map-perf-heading">
    <h2 id="system-map-perf-heading" class="font-display text-lg text-text-primary">System Map renderer</h2>
    <p class="mt-1 mb-3 text-sm text-text-secondary">400-node cap; the shared renderer reports a 300-frame median.</p>
    <div class="relative h-[520px] overflow-hidden rounded-lg border border-border-active bg-background">
      <SystemMapFrame
        data={ready(systemMapSample(400))}
        mode="3d"
        quality="low"
        onmetrics={(median) => (systemMapMedian = median)}
      />
      {#if systemMapMedian !== null}
        <p class="absolute top-3 right-3 z-40 rounded bg-surface-1 px-2 py-1 text-xs text-text-muted" data-testid="system-map-frame-median">
          300-frame median: {systemMapMedian.toFixed(2)} ms
        </p>
      {/if}
    </div>
  </section>

  <p class="mt-3 text-xs text-text-muted" aria-live="polite">
    Dev-source evidence only: 100k fixture events are loaded into the virtualised component, unlike the product's bounded page cache. The 600-frame result samples three 200-frame, 37 px/frame segments; repositioning is excluded. Headless browser results do not satisfy the installed WebView2 gate. Fixture rows never reach the backend. Build output is gated by <code>scripts/bundle-size.mjs</code>.
  </p>
</div>
