import { PerspectiveCamera, Spherical, Vector2, Vector3 } from 'three';
import type { SceneBounds } from './types';

export interface SceneCameraController {
  readonly camera: PerspectiveCamera;
  update(): boolean;
  reset(): void;
  resize(width: number, height: number): void;
  dispose(): void;
}

const MIN_POLAR = Math.PI * 0.15;
const MAX_POLAR = Math.PI * 0.85;

export function createCameraController(
  element: HTMLElement,
  bounds: SceneBounds,
  reducedMotion: boolean,
  requestRender: () => void,
): SceneCameraController {
  const camera = new PerspectiveCamera(42, 1, 0.1, 10_000);
  const center = new Vector3(bounds.centerX, bounds.centerY, bounds.centerZ);
  const target = center.clone();
  const fitDistance = Math.max(20, bounds.width * 0.85, bounds.height * 1.2, bounds.depth * 2);
  const spherical = new Spherical(fitDistance, Math.PI * 0.42, Math.PI * 0.18);
  const velocity = new Vector2();
  const previous = new Vector2();
  const offset = new Vector3();
  const right = new Vector3();
  const up = new Vector3();
  let dragging = false;
  let panMode = false;

  function place(): void {
    offset.setFromSpherical(spherical);
    camera.position.copy(target).add(offset);
    camera.lookAt(target);
    camera.updateMatrixWorld();
  }

  function reset(): void {
    target.copy(center);
    spherical.set(fitDistance, Math.PI * 0.42, Math.PI * 0.18);
    velocity.set(0, 0);
    place();
    requestRender();
  }

  function pointerDown(event: PointerEvent): void {
    if (event.button !== 0 && event.button !== 1 && event.button !== 2) return;
    dragging = true;
    panMode = event.button !== 0 || event.shiftKey;
    previous.set(event.clientX, event.clientY);
    element.setPointerCapture?.(event.pointerId);
  }

  function pointerMove(event: PointerEvent): void {
    if (!dragging) return;
    const dx = event.clientX - previous.x;
    const dy = event.clientY - previous.y;
    previous.set(event.clientX, event.clientY);
    if (panMode) {
      const units = spherical.radius / Math.max(1, element.clientHeight) * 1.2;
      camera.getWorldDirection(up);
      right.crossVectors(up, camera.up).normalize();
      up.crossVectors(right, up).normalize();
      target.addScaledVector(right, -dx * units).addScaledVector(up, dy * units);
      clampTarget(target, bounds);
      place();
    } else if (reducedMotion) {
      spherical.theta -= dx * 0.006;
      spherical.phi = clamp(spherical.phi - dy * 0.006, MIN_POLAR, MAX_POLAR);
      place();
    } else {
      velocity.set(-dx * 0.006, -dy * 0.006);
    }
    requestRender();
  }

  function pointerUp(event: PointerEvent): void {
    dragging = false;
    element.releasePointerCapture?.(event.pointerId);
  }

  function wheel(event: WheelEvent): void {
    event.preventDefault();
    spherical.radius = clamp(spherical.radius * Math.exp(event.deltaY * 0.001), fitDistance * 0.4, fitDistance * 4);
    place();
    requestRender();
  }

  function update(): boolean {
    if (reducedMotion || velocity.lengthSq() < 0.000001) return false;
    spherical.theta += velocity.x;
    spherical.phi = clamp(spherical.phi + velocity.y, MIN_POLAR, MAX_POLAR);
    velocity.multiplyScalar(0.82);
    if (velocity.lengthSq() < 0.000001) velocity.set(0, 0);
    place();
    return true;
  }

  function resize(width: number, height: number): void {
    camera.aspect = width / Math.max(1, height);
    camera.updateProjectionMatrix();
    requestRender();
  }

  function dispose(): void {
    element.removeEventListener('pointerdown', pointerDown);
    element.removeEventListener('pointermove', pointerMove);
    element.removeEventListener('pointerup', pointerUp);
    element.removeEventListener('pointercancel', pointerUp);
    element.removeEventListener('wheel', wheel);
  }

  element.addEventListener('pointerdown', pointerDown);
  element.addEventListener('pointermove', pointerMove);
  element.addEventListener('pointerup', pointerUp);
  element.addEventListener('pointercancel', pointerUp);
  element.addEventListener('wheel', wheel, { passive: false });
  reset();
  return { camera, update, reset, resize, dispose };
}

function clampTarget(target: Vector3, bounds: SceneBounds): void {
  target.x = clamp(target.x, bounds.minX, bounds.maxX);
  target.y = clamp(target.y, bounds.minY, bounds.maxY);
  target.z = clamp(target.z, bounds.minZ, bounds.maxZ);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
