import { describe, expect, it, vi } from 'vitest';
import { BufferGeometry, DataTexture, Mesh, MeshBasicMaterial, Object3D } from 'three';
import { createCameraController } from '../shared/camera';
import { disposeObjectTree, disposeRenderer } from '../shared/dispose';

describe('Change Map disposal', () => {
  it('disposes geometry, material, and textures exhaustively over 20 lifecycles', () => {
    for (let cycle = 0; cycle < 20; cycle += 1) {
      const root = new Object3D();
      const geometry = new BufferGeometry();
      const texture = new DataTexture();
      const material = new MeshBasicMaterial({ map: texture });
      const geometryDispose = vi.spyOn(geometry, 'dispose');
      const textureDispose = vi.spyOn(texture, 'dispose');
      const materialDispose = vi.spyOn(material, 'dispose');
      root.add(new Mesh(geometry, material));
      disposeObjectTree(root);
      expect(geometryDispose).toHaveBeenCalledOnce();
      expect(textureDispose).toHaveBeenCalledOnce();
      expect(materialDispose).toHaveBeenCalledOnce();
      expect(root.children).toHaveLength(0);
    }
  });

  it('stops animation, clears render lists, loses context, and removes canvas', () => {
    const canvas = document.createElement('canvas');
    document.body.append(canvas);
    const renderer = {
      setAnimationLoop: vi.fn(),
      renderLists: { dispose: vi.fn() },
      dispose: vi.fn(),
      forceContextLoss: vi.fn(),
      domElement: canvas,
    };
    disposeRenderer(renderer as never);
    expect(renderer.setAnimationLoop).toHaveBeenCalledWith(null);
    expect(renderer.renderLists.dispose).toHaveBeenCalledOnce();
    expect(renderer.dispose).toHaveBeenCalledOnce();
    expect(renderer.forceContextLoss).toHaveBeenCalledOnce();
    expect(canvas.isConnected).toBe(false);
  });

  it('removes every direct-interaction listener over 20 controller lifecycles', () => {
    for (let cycle = 0; cycle < 20; cycle += 1) {
      const element = document.createElement('canvas');
      const add = vi.spyOn(element, 'addEventListener');
      const remove = vi.spyOn(element, 'removeEventListener');
      const controller = createCameraController(element, {
        minX: -10, maxX: 10, minY: -5, maxY: 5, minZ: 0, maxZ: 8,
        centerX: 0, centerY: 0, centerZ: 4, width: 20, height: 10, depth: 8,
      }, true, vi.fn());
      controller.dispose();
      expect(add).toHaveBeenCalledTimes(5);
      expect(remove).toHaveBeenCalledTimes(5);
    }
  });
});
