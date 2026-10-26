import { AmbientLight, DirectionalLight, PerspectiveCamera, Scene } from 'three';
import { FrameTiming, type QualityTier } from '../changeMap/quality';
import { createCameraController, type SceneCameraController } from '../shared/camera';
import { disposeRenderer } from '../shared/dispose';
import { createPickingController, type PickingController } from '../shared/picking';
import { createSharedRenderer } from '../shared/renderer';
import { layoutSystemMap } from './layout';
import { createSystemMapContent, type SystemMapRenderContent } from './scene-content';
import type { EntityRefLike, SystemMapDataLike, SystemMapNodeLike } from './types';

export interface SystemMapSceneOptions {
  readonly container: HTMLElement;
  readonly data: SystemMapDataLike;
  readonly reducedMotion?: boolean;
  readonly quality?: QualityTier;
  readonly selectedEntityId?: string | null;
  readonly onHover?: ((node: SystemMapNodeLike | null, clientX: number, clientY: number) => void) | undefined;
  readonly onFocus?: ((entity: EntityRefLike) => void) | undefined;
  readonly onContextLost?: (() => void) | undefined;
  readonly onMetrics?: ((medianMs: number) => void) | undefined;
}

export interface SystemMapSceneApi {
  readonly canvas: HTMLCanvasElement;
  update(data: SystemMapDataLike): void;
  resize(width?: number, height?: number): void;
  render(): void;
  setVisible(visible: boolean): void;
  setSelected(entityId: string | null): void;
  refreshPalette(): void;
  resetView(): void;
  frameTiming(): { samples: number; median_ms: number };
  destroy(): void;
}

interface SceneContent {
  readonly data: SystemMapDataLike;
  readonly render: SystemMapRenderContent;
  readonly bounds: ReturnType<typeof layoutSystemMap>['bounds'];
}

export function createSystemMapScene(options: SystemMapSceneOptions): SystemMapSceneApi {
  const renderer = createSharedRenderer(options.container);
  const scene = new Scene();
  scene.add(new AmbientLight(0xffffff, 1.35));
  const directional = new DirectionalLight(0xffffff, 1.1);
  directional.position.set(25, 40, 45);
  scene.add(directional);
  const reducedMotion = options.reducedMotion ?? window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const quality = options.quality ?? 'high';
  const timing = new FrameTiming(300);
  let ready = false;
  let content = buildContent(options.data);
  let cameraController = buildCamera(content);
  let picking = buildPicking(content, cameraController.camera);
  let selectedEntityId = options.selectedEntityId ?? options.data.focus?.entity_id ?? null;
  let visible = !document.hidden;
  let destroyed = false;
  let metricsReported = false;

  function buildContent(data: SystemMapDataLike): SceneContent {
    const layout = layoutSystemMap(data);
    const render = createSystemMapContent(data, layout, quality);
    scene.add(render.edges, render.nodes);
    return { data, render, bounds: layout.bounds };
  }

  function disposeContent(current: SceneContent): void {
    scene.remove(current.render.edges, current.render.nodes);
    current.render.dispose();
  }

  function buildCamera(current: SceneContent): SceneCameraController {
    return createCameraController(renderer.domElement, current.bounds, reducedMotion, () => {
      if (ready) requestRender();
    });
  }

  function buildPicking(current: SceneContent, camera: PerspectiveCamera): PickingController {
    return createPickingController(renderer.domElement, camera, current.render.nodes, current.data.nodes, {
      hover(_index, node, clientX, clientY): void {
        current.render.emphasize(node?.entity_id ?? selectedEntityId);
        options.onHover?.(node, clientX, clientY);
        requestRender();
      },
      select(_index, node): void {
        selectedEntityId = node.entity_id;
        current.render.emphasize(node.entity_id);
        options.onFocus?.({ entity_id: node.entity_id, entity_type: node.entity_type });
        requestRender();
      },
    });
  }

  function render(): void {
    if (destroyed || !visible) return;
    cameraController.update();
    renderer.render(scene, cameraController.camera);
  }

  function animationFrame(timestamp: number): void {
    const sampled = timing.frame(timestamp);
    render();
    if (sampled && !metricsReported && timing.sampleCount === 300) {
      metricsReported = true;
      options.onMetrics?.(timing.median());
    }
  }

  function syncLoop(): void {
    const continuous = visible && !reducedMotion;
    if (!continuous) timing.pause();
    renderer.setAnimationLoop(continuous ? animationFrame : null);
    if (visible && reducedMotion) render();
  }

  function requestRender(): void {
    if (reducedMotion) render();
  }

  function visibilityChanged(): void {
    visible = !document.hidden;
    syncLoop();
  }

  function contextLost(event: Event): void {
    event.preventDefault();
    setVisible(false);
    options.onContextLost?.();
  }

  function update(data: SystemMapDataLike): void {
    ready = false;
    picking.dispose();
    cameraController.dispose();
    disposeContent(content);
    content = buildContent(data);
    cameraController = buildCamera(content);
    picking = buildPicking(content, cameraController.camera);
    if (!data.nodes.some((node) => node.entity_id === selectedEntityId)) {
      selectedEntityId = data.focus?.entity_id ?? null;
    }
    content.render.emphasize(selectedEntityId);
    ready = true;
    requestRender();
  }

  function resize(width = options.container.clientWidth, height = options.container.clientHeight): void {
    const safeWidth = Math.max(1, Math.floor(width));
    const safeHeight = Math.max(1, Math.floor(height));
    renderer.setSize(safeWidth, safeHeight, false);
    cameraController.resize(safeWidth, safeHeight);
  }

  function setVisible(nextVisible: boolean): void {
    visible = nextVisible && !document.hidden;
    syncLoop();
  }

  function setSelected(entityId: string | null): void {
    selectedEntityId = entityId;
    content.render.emphasize(entityId);
    requestRender();
  }

  function refreshPalette(): void {
    content.render.refreshPalette();
    content.render.emphasize(selectedEntityId);
    requestRender();
  }

  function destroy(): void {
    if (destroyed) return;
    destroyed = true;
    renderer.setAnimationLoop(null);
    document.removeEventListener('visibilitychange', visibilityChanged);
    renderer.domElement.removeEventListener('webglcontextlost', contextLost);
    picking.dispose();
    cameraController.dispose();
    disposeContent(content);
    scene.clear();
    disposeRenderer(renderer);
  }

  document.addEventListener('visibilitychange', visibilityChanged);
  renderer.domElement.addEventListener('webglcontextlost', contextLost);
  ready = true;
  content.render.emphasize(selectedEntityId);
  syncLoop();
  return {
    canvas: renderer.domElement,
    update,
    resize,
    render,
    setVisible,
    setSelected,
    refreshPalette,
    resetView: (): void => cameraController.reset(),
    frameTiming: (): { samples: number; median_ms: number } => ({
      samples: timing.sampleCount,
      median_ms: timing.median(),
    }),
    destroy,
  };
}
