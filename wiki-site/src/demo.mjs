import { booking } from './demo-data.mjs';

export function attachDemo(root) {
  root.querySelectorAll('[data-enhanced]').forEach((item) => { item.hidden = false; });
  const state = root.querySelector('[data-state]'), build = root.querySelector('[data-build]');
  let scene, selected = 'intent', pending = false, disposed = false, generation = 0;
  const update = () => {
    const record = booking.states[state.value], verdict = record[build.value];
    root.querySelector('[data-verdict]').textContent = `${verdict[0].toUpperCase()}${verdict.slice(1)} · ${booking[build.value]}`;
    root.querySelector('[data-verdict]').dataset.verdict = verdict;
    root.querySelector('[data-explanation]').textContent = record.explanation;
    scene?.setResult(verdict);
  };
  state.addEventListener('change', update); build.addEventListener('change', update);
  for (const button of root.querySelectorAll('[data-select]')) button.addEventListener('click', () => {
    selected = button.dataset.select;
    const node = booking.nodes.find((item) => item.id === selected);
    const edges = booking.edges.filter((item) => item.from === selected || item.to === selected);
    root.querySelector('[data-inspector]').textContent = `${node.kind}: ${node.label}. ${node.detail} Connections: ${edges.map((item) => `${item.from} ${item.type} ${item.to}`).join('; ')}.`;
    root.querySelectorAll('[data-node]').forEach((item) => { item.dataset.selected = String(item.dataset.node === selected); });
    scene?.select(selected);
  });
  const load = root.querySelector('[data-load-3d]');
  const panel = root.querySelector('[data-three-panel]');
  const canvas = root.querySelector('[data-three-canvas]');
  const load3d = async () => {
    if (scene || !panel || pending || disposed) return;
    const current = generation;
    pending = true;
    if (load) { load.disabled = true; load.textContent = 'Loading 3D…'; }
    try {
      const { createWorkbench } = await import('./workbench-3d.mjs');
      if (disposed || current !== generation) return;
      panel.hidden = false;
      canvas.querySelector('.three-fallback')?.remove();
      scene = createWorkbench(canvas, (id) => root.querySelector(`[data-select="${id}"]`).click());
      // Retained controls also own the view after context recovery or page restore.
      scene.zoom(Number(root.querySelector('[data-zoom]').value));
      scene.explode(root.querySelector('[data-explode]').checked);
      scene.layer(root.querySelector('[data-layer]').value);
      scene.select(selected); update();
      if (load) load.textContent = '3D loaded';
    } catch {
      panel.hidden = true;
      if (load) { load.disabled = false; load.textContent = 'Retry 3D workbench'; }
      root.querySelector('[data-inspector]').textContent = '3D is unavailable in this browser. The complete 2D cards and connection list remain available.';
    } finally { if (current === generation) pending = false; }
  };
  canvas?.addEventListener('workbenchfallback', () => {
    scene = undefined;
    if (load) { load.disabled = false; load.textContent = 'Retry 3D workbench'; }
  });
  load?.addEventListener('click', load3d);
  if (root.getAttribute('data-auto-3d') === 'true') {
    void load3d();
  }
  root.querySelector('[data-reset]')?.addEventListener('click', () => { root.querySelector('[data-zoom]').value = '1'; scene?.reset(); });
  root.querySelector('[data-fit]')?.addEventListener('click', () => scene?.fit());
  root.querySelector('[data-zoom]')?.addEventListener('input', (event) => scene?.zoom(Number(event.target.value)));
  root.querySelector('[data-explode]')?.addEventListener('change', (event) => scene?.explode(event.target.checked));
  root.querySelector('[data-layer]')?.addEventListener('change', (event) => scene?.layer(event.target.value));
  root.querySelector('[data-close-3d]')?.addEventListener('click', () => {
    scene?.dispose(); scene = undefined; panel.hidden = true;
    if (load) { load.disabled = false; load.textContent = 'Load 3D workbench'; load.focus(); }
  });
  window.addEventListener('pagehide', () => { disposed = true; generation++; pending = false; scene?.dispose(); scene = undefined; });
  window.addEventListener('pageshow', (event) => {
    if (!event.persisted) return;
    disposed = false;
    if (!panel.hidden) void load3d();
    else if (load) { load.disabled = false; load.textContent = 'Load 3D workbench'; }
  });
  update();
}
