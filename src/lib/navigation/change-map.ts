/** Contextual Change Map URL helpers. The sidebar route never changes. */

export type ChangeMapOrigin = 'timeline' | 'investigation';

const MAP_HISTORY_ENTRY = '__whatChangedChangeMap';

function parsed(hash = globalThis.location?.hash ?? '#/'): { path: string; params: URLSearchParams } {
  const [path = '#/', query = ''] = hash.split('?', 2);
  return { path, params: new URLSearchParams(query) };
}

export function changeMapOpen(hash = globalThis.location?.hash ?? ''): boolean {
  return parsed(hash).params.get('view') === 'map';
}

export function mapInvestigationId(hash = globalThis.location?.hash ?? ''): number | null {
  const value = Number(parsed(hash).params.get('id'));
  return Number.isSafeInteger(value) && value > 0 ? value : null;
}

export function setChangeMap(open: boolean, extras: Record<string, string | null> = {}, win: Window = window): void {
  const { path, params } = parsed(win.location.hash);
  const isMapEntry = Boolean((win.history.state as Record<string, unknown> | null)?.[MAP_HISTORY_ENTRY]);
  if (!open && isMapEntry) {
    win.history.back();
    return;
  }
  if (open) params.set('view', 'map');
  else params.delete('view');
  for (const [key, value] of Object.entries(extras)) {
    if (value === null) params.delete(key);
    else params.set(key, value);
  }
  const query = params.toString();
  const next = `${path}${query ? `?${query}` : ''}`;
  if (open) win.history.pushState({ ...(win.history.state ?? {}), [MAP_HISTORY_ENTRY]: true }, '', next);
  else win.history.replaceState(win.history.state, '', next);
  win.dispatchEvent(new HashChangeEvent('hashchange'));
}

export function restoreMapTrigger(selector: string): void {
  let attempts = 0;
  const focusWhenReady = () => {
    const target = document.querySelector<HTMLElement>(selector) ?? document.querySelector<HTMLElement>('#main h1') ?? document.querySelector<HTMLElement>('#main');
    if (target) target.focus();
    else if (attempts++ < 10) requestAnimationFrame(focusWhenReady);
  };
  requestAnimationFrame(focusWhenReady);
}
