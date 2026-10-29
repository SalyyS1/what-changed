import type { Material, Object3D, Texture, WebGLRenderer } from 'three';

type Disposable = { dispose(): void };

export function disposeObjectTree(root: Object3D): void {
  const disposed = new Set<object>();
  root.traverse((object) => {
    const candidate = object as Object3D & { geometry?: Disposable; material?: Material | Material[] };
    disposeOnce(candidate.geometry, disposed);
    const materials = Array.isArray(candidate.material) ? candidate.material : [candidate.material];
    for (const material of materials) {
      if (!material || disposed.has(material)) continue;
      disposeMaterialTextures(material, disposed);
      disposeOnce(material, disposed);
    }
  });
  root.clear();
}

export function disposeRenderer(renderer: WebGLRenderer): void {
  renderer.setAnimationLoop(null);
  renderer.renderLists.dispose();
  renderer.dispose();
  renderer.forceContextLoss();
  renderer.domElement.remove();
}

function disposeMaterialTextures(material: Material, disposed: Set<object>): void {
  for (const value of Object.values(material)) {
    if (isTexture(value)) disposeOnce(value, disposed);
  }
}

function isTexture(value: unknown): value is Texture {
  return typeof value === 'object' && value !== null && 'isTexture' in value && value.isTexture === true;
}

function disposeOnce(value: Disposable | undefined, disposed: Set<object>): void {
  if (!value || disposed.has(value)) return;
  disposed.add(value);
  value.dispose();
}
