export type ChangeMapRenderer = '3d' | '2d';
export type VisualizationPreference = 'auto' | '3d' | '2d';

interface RendererConditions {
  preference: VisualizationPreference;
  reducedMotion: boolean;
  webglAvailable: boolean;
  contextLost: boolean;
}

export function resolveChangeMapRenderer({
  preference,
  reducedMotion,
  webglAvailable,
  contextLost,
}: RendererConditions): ChangeMapRenderer {
  if (contextLost || !webglAvailable || preference === '2d') return '2d';
  if (preference === 'auto' && reducedMotion) return '2d';
  return '3d';
}
