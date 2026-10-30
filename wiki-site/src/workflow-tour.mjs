import { tourSteps } from './workflow-exhibit.mjs';

export function attachWorkflow(root) {
  const vi = root.dataset.language === 'vi';
  const poster = root.querySelector('[data-model-poster]');
  const canvas = root.querySelector('[data-model-canvas]');
  const toggle = root.querySelector('[data-model-toggle]');
  const message = root.querySelector('[data-model-message]');
  const extra = [...root.querySelectorAll('[data-model-rotate], [data-model-reset]')];
  let scene, pending = false, disposed = false, stepIndex = 0, generation = 0, wants3d = true;
  root.querySelectorAll('[data-tour-enhanced]').forEach((element) => { element.hidden = false; });
  const selectStep = (index) => {
    stepIndex = index;
    const step = tourSteps[index], copy = step[vi ? 'vi' : 'en'];
    root.querySelectorAll('[data-tour-step]').forEach((button, i) => button.setAttribute('aria-pressed', String(i === index)));
    root.querySelector('[data-tour-title]').textContent = copy[1];
    root.querySelector('[data-tour-detail]').textContent = copy[2];
    root.querySelectorAll('[data-poster-node]').forEach((node) => node.classList.toggle('is-current', node.dataset.posterNode === step.id));
    scene?.select(step.id);
  };
  const showPoster = () => {
    scene?.dispose(); scene = undefined;
    canvas.hidden = true; poster.hidden = false;
    toggle.textContent = '3D'; toggle.setAttribute('aria-pressed', 'false');
    root.querySelector('[data-model-status]').textContent = vi ? 'Sơ đồ 2D' : '2D map';
    extra.forEach((button) => { button.hidden = true; });
  };
  const load = async () => {
    if (pending || disposed || scene) return;
    const current = generation;
    pending = true; toggle.disabled = true; toggle.textContent = '…';
    try {
      const { createWorkbench } = await import('./workbench-3d.mjs');
      if (disposed || current !== generation) return;
      canvas.hidden = false;
      scene = createWorkbench(canvas, (id) => {
        const index = id === 'deny' ? 3 : id === 'denial' ? 4 : tourSteps.findIndex((step) => step.id === id);
        if (index >= 0) { selectStep(index); scene?.select(id); }
      }, { language: vi ? 'vi' : 'en' });
      scene.select(tourSteps[stepIndex].id);
      poster.hidden = true;
      toggle.textContent = '2D'; toggle.setAttribute('aria-pressed', 'true');
      root.querySelector('[data-model-status]').textContent = vi ? 'Mô hình 3D' : '3D workbench';
      extra.forEach((button) => { button.hidden = false; });
      message.textContent = '';
    } catch {
      wants3d = false;
      showPoster();
      message.textContent = vi ? 'Trình duyệt chưa hỗ trợ 3D. Sơ đồ và các bước vẫn dùng được.' : '3D is unavailable here. The complete map and walkthrough still work.';
    } finally { if (current === generation) { pending = false; toggle.disabled = false; } }
  };
  root.querySelectorAll('[data-tour-step]').forEach((button) => button.addEventListener('click', () => selectStep(Number(button.dataset.tourStep))));
  toggle.addEventListener('click', () => {
    wants3d = !scene;
    if (scene) showPoster(); else void load();
  });
  root.querySelector('[data-model-rotate]').addEventListener('click', () => scene?.rotate?.());
  root.querySelector('[data-model-reset]').addEventListener('click', () => scene?.reset());
  canvas.addEventListener('webglcontextlost', () => {
    wants3d = false;
    showPoster(); message.textContent = vi ? 'Đã chuyển về sơ đồ 2D.' : 'Graphics context lost. Switched to the complete 2D map.';
  }, true);
  window.addEventListener('pagehide', () => { disposed = true; generation++; pending = false; scene?.dispose(); scene = undefined; });
  window.addEventListener('pageshow', (event) => {
    if (!event.persisted) return;
    disposed = false; showPoster(); toggle.disabled = false;
    if (wants3d) void load();
  });
  // The static poster reserves the exact space before the renderer arrives.
  window.requestAnimationFrame(() => { if (!disposed) void load(); });
}
