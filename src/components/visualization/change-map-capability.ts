/** Checks WebGL without retaining the throwaway context used for the probe. */
export function probeWebGl(canvas: HTMLCanvasElement, preference: 'auto' | '3d' | '2d'): boolean {
  if (preference === '2d') return true;
  try {
    const context = canvas.getContext('webgl2') || canvas.getContext('webgl');
    if (!context) return false;
    context.getExtension('WEBGL_lose_context')?.loseContext();
    return true;
  } catch {
    return false;
  }
}
