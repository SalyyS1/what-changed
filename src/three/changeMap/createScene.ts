import {
  AmbientLight,
  DirectionalLight,
  DoubleSide,
  Mesh,
  MeshBasicMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  Scene,
  Vector3,
} from 'three';
import { createCameraController, type SceneCameraController } from '../shared/camera';
import { disposeRenderer } from '../shared/dispose';
import { readThreePalette, type PaletteToken } from '../shared/palette';
import { chainMembership, createPickingController, type PickingController } from '../shared/picking';
import { createSharedRenderer } from '../shared/renderer';
import { createEdges, type ChangeMapEdges } from './edges';
import { layoutChangeMap, symptomX } from './layout';
import { createNodes, type ChangeMapNodes } from './nodes';
import { FrameTiming, type QualityTier } from './quality';
import type { ChangeMapDataLike, ChangeMapLane, ChangeMapLayout, ChangeMapNodeLike, ProjectedPoint } from './types';

const CHANGE_MAP_PALETTE_TOKENS: Readonly<Record<ChangeMapLane, PaletteToken>> = {
  process: { token: '--map-category-process', fallbackToken: '--accent' },
  port: { token: '--map-category-port', fallbackToken: '--info' },
  config: { token: '--map-category-config', fallbackToken: '--accent' },
  service: { token: '--map-category-service', fallbackToken: '--success' },
  application: { token: '--map-category-application', fallbackToken: '--success' },
  update: { token: '--map-category-update', fallbackToken: '--warning' },
  driver: { token: '--map-category-driver', fallbackToken: '--danger' },
  network: { token: '--map-category-network', fallbackToken: '--info' },
  startup: { token: '--map-category-startup', fallbackToken: '--warning' },
  system: { token: '--map-category-system', fallbackToken: '--text-secondary' },
};

export interface ChangeMapSceneOptions {
  readonly container: HTMLElement;
  readonly data: ChangeMapDataLike;
  readonly reducedMotion?: boolean;
  readonly quality?: QualityTier;
  readonly onHover?: (node: ChangeMapNodeLike | null, clientX: number, clientY: number) => void;
  readonly onSelect?: (eventId: number) => void;
  readonly onContextLost?: () => void;
  readonly onProject?: (x: number, y: number, visible: boolean) => void;
  readonly onMetrics?: (medianMs: number) => void;
}

export interface ChangeMapSceneApi {
  readonly canvas: HTMLCanvasElement;
  update(data: ChangeMapDataLike): void;
  resize(width: number, height: number): void;
  render(): void;
  setVisible(visible: boolean): void;
  setSelected(eventId: number | null): void;
  refreshPalette(): void;
  resetView(): void;
  projectSymptom(): ProjectedPoint | null;
  projectNode(eventId: number): ProjectedPoint | null;
  frameTiming(): { samples: number; median_ms: number };
  destroy(): void;
}

interface RenderContent {
  readonly data: ChangeMapDataLike;
  readonly layout: ChangeMapLayout;
  readonly nodes: ChangeMapNodes;
  readonly edges: ChangeMapEdges;
  readonly marker: Mesh<PlaneGeometry, MeshBasicMaterial> | null;
}

export function createChangeMapScene(options: ChangeMapSceneOptions): ChangeMapSceneApi {
  const renderer = createSharedRenderer(options.container);
  const scene = new Scene();
  scene.add(new AmbientLight(0xffffff, 1.35));
  const directional = new DirectionalLight(0xffffff, 1.1);
  directional.position.set(25, 40, 45);
  scene.add(directional);

  const reducedMotion = options.reducedMotion ?? window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const quality = options.quality ?? 'high';
  let ready = false;
  let content = buildContent(options.data);
  let cameraController = buildCamera(content);
  let picking = buildPicking(content, cameraController.camera);
  let selectedEventId: number | null = null;
  let visible = !document.hidden;
  let destroyed = false;
  let metricsReported = false;
  const symptomWorld = new Vector3();
  const projectedPoint = { x: 0, y: 0, visible: false };
  const canvasRect = { left: 0, top: 0, width: 1, height: 1 };
  const timing = new FrameTiming(300);

  function buildContent(data: ChangeMapDataLike): RenderContent {
    const layout = layoutChangeMap(data);
    const nodes = createNodes(data.nodes, layout, readThreePalette(CHANGE_MAP_PALETTE_TOKENS), quality);
    const edges = createEdges(data, layout, quality);
    const markerX = symptomX(data);
    scene.add(nodes.mesh, edges.temporal, edges.structural);
    let marker: Mesh<PlaneGeometry, MeshBasicMaterial> | null = null;
    if (markerX !== null) {
      marker = new Mesh(
        new PlaneGeometry(Math.max(1, layout.bounds.depth + 8), Math.max(1, layout.bounds.height + 8)),
        new MeshBasicMaterial({ color: 0xd9a32b, transparent: true, opacity: 0.11, depthWrite: false, side: DoubleSide }),
      );
      marker.rotation.y = Math.PI / 2;
      marker.position.set(markerX, layout.bounds.centerY, layout.bounds.centerZ);
      scene.add(marker);
    }
    return { data, layout, nodes, edges, marker };
  }

  function disposeContent(current: RenderContent): void {
    scene.remove(current.nodes.mesh, current.edges.temporal, current.edges.structural);
    if (current.marker) scene.remove(current.marker);
    current.nodes.dispose();
    current.edges.dispose();
    current.marker?.geometry.dispose();
    current.marker?.material.dispose();
  }

  function buildCamera(current: RenderContent): SceneCameraController {
    return createCameraController(renderer.domElement, current.layout.bounds, reducedMotion, () => {
      if (ready) requestRender();
    });
  }

  function buildPicking(current: RenderContent, camera: PerspectiveCamera): PickingController {
    return createPickingController(renderer.domElement, camera, current.nodes.mesh, current.data.nodes, {
      hover(index, node, clientX, clientY): void {
        applyEmphasis(index, node);
        options.onHover?.(node, clientX, clientY);
        requestRender();
      },
      select(index, node): void {
        selectedEventId = node.event_id;
        applyEmphasis(index, node);
        options.onSelect?.(node.event_id);
        requestRender();
      },
    });
  }

  function selectedIndex(): number | null {
    if (selectedEventId === null) return null;
    const index = content.data.nodes.findIndex((node) => node.event_id === selectedEventId);
    return index < 0 ? null : index;
  }

  function applyEmphasis(hoverIndex: number | null, hoverNode: ChangeMapNodeLike | null): void {
    const fallbackIndex = selectedIndex();
    const activeIndex = hoverIndex ?? fallbackIndex;
    const activeNode = hoverNode ?? (fallbackIndex === null ? null : content.data.nodes[fallbackIndex] ?? null);
    const chains = chainMembership(activeNode);
    content.nodes.applyEmphasis(chains, activeIndex);
    content.edges.emphasizeChains(chains);
  }

  function render(): void {
    if (destroyed || !visible) return;
    cameraController.update();
    renderer.render(scene, cameraController.camera);
    const point = projectSymptom();
    if (point) options.onProject?.(point.x, point.y, point.visible);
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

  function update(data: ChangeMapDataLike): void {
    ready = false;
    picking.dispose();
    cameraController.dispose();
    disposeContent(content);
    content = buildContent(data);
    cameraController = buildCamera(content);
    picking = buildPicking(content, cameraController.camera);
    ready = true;
    selectedEventId = data.nodes.some((node) => node.event_id === selectedEventId) ? selectedEventId : null;
    applyEmphasis(null, null);
    requestRender();
  }

  function resize(width: number, height: number): void {
    const safeWidth = Math.max(1, Math.floor(width));
    const safeHeight = Math.max(1, Math.floor(height));
    renderer.setSize(safeWidth, safeHeight, false);
    cameraController.resize(safeWidth, safeHeight);
    updateCanvasRect();
  }

  function updateCanvasRect(): void {
    const rect = renderer.domElement.getBoundingClientRect();
    canvasRect.left = rect.left;
    canvasRect.top = rect.top;
    canvasRect.width = Math.max(1, rect.width);
    canvasRect.height = Math.max(1, rect.height);
  }

  function setVisible(nextVisible: boolean): void {
    visible = nextVisible && !document.hidden;
    syncLoop();
  }

  function setSelected(eventId: number | null): void {
    selectedEventId = eventId;
    applyEmphasis(null, null);
    requestRender();
  }

  function refreshPalette(): void {
    content.nodes.refreshColors(readThreePalette(CHANGE_MAP_PALETTE_TOKENS));
    applyEmphasis(null, null);
    requestRender();
  }

  function projectSymptom(): ProjectedPoint | null {
    if (!content.marker) return null;
    symptomWorld.copy(content.marker.position).project(cameraController.camera);
    return projectedScreenPoint(symptomWorld);
  }

  function projectNode(eventId: number): ProjectedPoint | null {
    const index = content.data.nodes.findIndex((node) => node.event_id === eventId);
    if (index < 0) return null;
    const offset = index * 3;
    symptomWorld.set(
      content.layout.positions[offset] ?? 0,
      content.layout.positions[offset + 1] ?? 0,
      content.layout.positions[offset + 2] ?? 0,
    ).project(cameraController.camera);
    return projectedScreenPoint(symptomWorld);
  }

  function projectedScreenPoint(point: Vector3): ProjectedPoint {
    projectedPoint.x = canvasRect.left + (point.x + 1) * canvasRect.width / 2;
    projectedPoint.y = canvasRect.top + (-point.y + 1) * canvasRect.height / 2;
    projectedPoint.visible = point.z >= -1 && point.z <= 1;
    return projectedPoint;
  }

  function destroy(): void {
    if (destroyed) return;
    destroyed = true;
    renderer.setAnimationLoop(null);
    document.removeEventListener('visibilitychange', visibilityChanged);
    window.removeEventListener('scroll', updateCanvasRect, true);
    renderer.domElement.removeEventListener('webglcontextlost', contextLost);
    picking.dispose();
    cameraController.dispose();
    disposeContent(content);
    scene.clear();
    disposeRenderer(renderer);
  }

  document.addEventListener('visibilitychange', visibilityChanged);
  window.addEventListener('scroll', updateCanvasRect, true);
  renderer.domElement.addEventListener('webglcontextlost', contextLost);
  updateCanvasRect();
  ready = true;
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
    projectSymptom,
    projectNode,
    frameTiming: (): { samples: number; median_ms: number } => ({
      samples: timing.sampleCount,
      median_ms: timing.median(),
    }),
    destroy,
  };
}
