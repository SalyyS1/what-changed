import { WebGLRenderer } from 'three';
import { disposeRenderer } from './dispose';

export interface SharedRendererOptions {
  readonly antialias?: boolean;
  readonly maxPixelRatio?: number;
  readonly powerPreference?: WebGLPowerPreference;
}

export function createSharedRenderer(
  container: HTMLElement,
  options: SharedRendererOptions = {},
): WebGLRenderer {
  const renderer = new WebGLRenderer({
    antialias: options.antialias ?? true,
    powerPreference: options.powerPreference ?? 'low-power',
  });
  if (renderer.getContext().isContextLost()) {
    disposeRenderer(renderer);
    throw new Error('WebGL context is unavailable');
  }
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, options.maxPixelRatio ?? 2));
  container.append(renderer.domElement);
  return renderer;
}
