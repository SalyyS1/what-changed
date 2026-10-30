import { Scene, OrthographicCamera, Vector2, Vector3, Raycaster, Color } from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { createSharedRenderer } from '../shared/renderer';
import { disposeRenderer } from '../shared/dispose';
import type { OwnerGraphWindow } from '../../lib/owner-graph/types';
import { layoutOwnerWorkbench } from './layout';
import { createOwnerContent } from './scene-content';
import { projectOwnerLabels, type OwnerLabel } from './labels';

export interface OwnerSceneOptions {
  container: HTMLElement; graph: OwnerGraphWindow; separation?: number; lowQuality?: boolean;
  onselect: (id: string) => void; onlabels: (labels: OwnerLabel[]) => void;
  onunavailable: (reason: string) => void;
}
export function createOwnerScene(options: OwnerSceneOptions) {
  const renderer = createSharedRenderer(options.container, { maxPixelRatio: options.lowQuality ? 1 : 1.5, antialias: !options.lowQuality });
  const scene = new Scene();
  scene.background = new Color(getComputedStyle(document.documentElement).getPropertyValue('--surface-2').trim() || 'gray');
  const camera = new OrthographicCamera(-20, 20, 20, -20, 0.1, 10000);
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = false; controls.autoRotate = false; controls.minZoom = 0.2; controls.maxZoom = 8;
  controls.maxPolarAngle = Math.PI / 2.1;
  let graph = options.graph;
  let separation = options.separation ?? 6;
  let layout = layoutOwnerWorkbench(graph, separation);
  let content = createOwnerContent(graph, layout);
  scene.add(content.group);
  let selectedId: string | null = null;
  let width = 800; let height = 400;
  let visible = true; let destroyed = false; let unavailable = false;
  let pending: number | null = null;
  let samples: number[] = []; let slowWindows = 0;
  let pointer: { x: number; y: number } | null = null;
  const raycaster = new Raycaster();

  function fail(reason: string): void { if (unavailable || destroyed) return; unavailable = true; cancel(); options.onunavailable(reason); }
  function cancel(): void { if (pending !== null) cancelAnimationFrame(pending); pending = null; }
  function request(): void {
    if (destroyed || unavailable || !visible || document.hidden || pending !== null) return;
    pending = requestAnimationFrame(render);
  }
  function render(): void {
    pending = null;
    if (destroyed || unavailable || !visible || document.hidden) return;
    try {
      const started = performance.now(); renderer.render(scene, camera);
      samples.push(performance.now() - started);
      options.onlabels(projectOwnerLabels(graph, layout, camera, width, height, selectedId));
      if (samples.length >= 12) {
        const median = samples.sort((a, b) => a - b)[6] ?? 0;
        slowWindows = median > 33.4 ? slowWindows + 1 : 0; samples = [];
        if (slowWindows >= 3) fail('3D repeatedly exceeded the interaction frame budget. The complete 2D and list views remain available.');
      }
    } catch { fail('3D rendering stopped. The complete 2D and list views remain available.'); }
  }
  function resize(w = options.container.clientWidth, h = options.container.clientHeight): void {
    width = Math.max(1, w || 800); height = Math.max(1, h || 400);
    renderer.setSize(width, height, false);
    const half = layout.extent * 0.68;
    camera.left = -half * width / height; camera.right = half * width / height; camera.top = half; camera.bottom = -half;
    camera.updateProjectionMatrix(); request();
  }
  function reset(): void {
    const center = new Vector3(layout.center.x, layout.center.y, layout.center.z);
    controls.target.copy(center); camera.position.copy(center).add(new Vector3(layout.extent, layout.extent, layout.extent));
    camera.zoom = 1; camera.lookAt(center); camera.updateProjectionMatrix(); controls.update(); request();
  }
  function fitSelected(): void {
    const point = selectedId ? layout.positions.get(selectedId) : undefined; if (!point) { reset(); return; }
    const target = new Vector3(point.x, point.y, point.z); const offset = camera.position.clone().sub(controls.target);
    controls.target.copy(target); camera.position.copy(target).add(offset); camera.zoom = Math.min(8, Math.max(1, layout.extent / 16));
    camera.lookAt(target); camera.updateProjectionMatrix(); controls.update(); request();
  }
  function zoom(factor: number): void { camera.zoom = Math.max(0.2, Math.min(8, camera.zoom * factor)); camera.updateProjectionMatrix(); request(); }
  function setSelected(id: string | null): void { selectedId = graph.nodes.some((node) => node.id === id) ? id : null; content.emphasize(selectedId); request(); }
  function update(next: OwnerGraphWindow, distance = separation): void {
    graph = next; separation = distance; scene.remove(content.group); content.dispose();
    layout = layoutOwnerWorkbench(graph, separation); content = createOwnerContent(graph, layout); scene.add(content.group);
    if (!graph.nodes.some((node) => node.id === selectedId)) selectedId = null;
    content.emphasize(selectedId); resize(width, height); reset();
  }
  function down(event: PointerEvent): void { pointer = { x: event.clientX, y: event.clientY }; }
  function up(event: PointerEvent): void {
    if (!pointer || Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > 5) { pointer = null; return; }
    pointer = null; const rect = renderer.domElement.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    raycaster.setFromCamera(new Vector2((event.clientX - rect.left) / rect.width * 2 - 1, -(event.clientY - rect.top) / rect.height * 2 + 1), camera);
    const hit = raycaster.intersectObjects(content.meshes, false)[0]?.object.userData.ownerNodeId;
    if (typeof hit === 'string') { setSelected(hit); options.onselect(hit); }
  }
  function lost(event: Event): void { event.preventDefault(); fail('The 3D context was lost. The complete 2D and list views remain available.'); }
  function visibility(): void { if (document.hidden) cancel(); else request(); }
  function setVisible(value: boolean): void { visible = value; if (!value) cancel(); else request(); }
  function refreshPalette(): void { scene.background = new Color(getComputedStyle(document.documentElement).getPropertyValue('--surface-2').trim() || 'gray'); content.emphasize(selectedId); request(); }
  function destroy(): void {
    if (destroyed) return; destroyed = true; cancel(); controls.removeEventListener('change', request); controls.dispose();
    renderer.domElement.removeEventListener('pointerdown', down); renderer.domElement.removeEventListener('pointerup', up);
    renderer.domElement.removeEventListener('webglcontextlost', lost); document.removeEventListener('visibilitychange', visibility);
    scene.remove(content.group); content.dispose(); scene.clear(); disposeRenderer(renderer);
  }
  controls.addEventListener('change', request);
  renderer.domElement.addEventListener('pointerdown', down); renderer.domElement.addEventListener('pointerup', up);
  renderer.domElement.addEventListener('webglcontextlost', lost); document.addEventListener('visibilitychange', visibility);
  renderer.domElement.setAttribute('aria-hidden', 'true'); resize(); reset();
  return { canvas: renderer.domElement, update, resize, reset, fitSelected, zoom, setSelected, setVisible, refreshPalette, destroy };
}
export type OwnerScene = ReturnType<typeof createOwnerScene>;
