/**
 * Hash router. Deliberately small: the app is a static SPA inside a webview,
 * so there is no server, no history API worth the complexity, and no need
 * for a framework. The URL is `#/<path>`; anything unknown resolves to Home
 * rather than a 404, because a desktop app has no "page not found".
 */

import { defaultRoute, routeByPath, routeByShortcut, type RouteDef } from '../../app/routes';

export type RouteParams = Readonly<Record<string, string>>;

function parseHash(hash: string): { route: RouteDef; params: RouteParams } {
  const path = hash.replace(/^#\/?/, '').split('?')[0]?.replace(/\/+$/, '') ?? '';
  const route = routeByPath(path) ?? defaultRoute;
  if (route.id === 'owner-app') {
    const appId = decodeSegment(path.split('/')[1]);
    return appId ? { route, params: { appId } } : { route: defaultRoute, params: {} };
  }
  if (route.id === 'owner-change') {
    const parts = path.split('/');
    const appId = decodeSegment(parts[1]);
    const changeId = parts[3] ? decodeSegment(parts[3]) : undefined;
    if (!appId || (parts[3] && !changeId)) return { route: defaultRoute, params: {} };
    return { route, params: { appId, ...(changeId ? { changeId } : {}) } };
  }
  return { route, params: {} };
}

function decodeSegment(value: string | undefined): string {
  if (!value || value.length > 200) return '';
  try {
    const decoded = decodeURIComponent(value);
    return /^[A-Za-z0-9][A-Za-z0-9._:-]{0,199}$/.test(decoded) ? decoded : '';
  } catch { return ''; }
}

export class Router {
  current = $state<RouteDef>(parseHash(globalThis.location?.hash ?? '').route);
  params = $state<RouteParams>(parseHash(globalThis.location?.hash ?? '').params);
  #teardown: (() => void) | null = null;

  /** Subscribe to the window; returns the teardown. Idempotent per instance. */
  attach(win: Window = window): () => void {
    this.#teardown?.();
    const onHash = () => {
      const parsed = parseHash(win.location.hash);
      if (this.#guard && !this.#guard(parsed.route, parsed.params)) {
        win.location.hash = this.href(this.current, this.params).slice(1);
        this.#blocked?.(parsed.route, parsed.params);
        return;
      }
      this.current = parsed.route;
      this.params = parsed.params;
    };
    const onKey = (e: KeyboardEvent) => {
      // Ctrl+1..7 jump to the sidebar destinations. AltGr on some layouts
      // reports as Ctrl+Alt, so Alt is excluded; Ctrl+digit has no meaning
      // inside a text field, so the shortcut still fires there.
      if (!e.ctrlKey || e.altKey || e.metaKey || e.repeat) {
        return;
      }
      const n = Number(e.key);
      if (!Number.isInteger(n) || n < 1 || n > 9) {
        return;
      }
      const r = routeByShortcut(n);
      if (r) {
        e.preventDefault();
        this.navigate(r);
      }
    };
    win.addEventListener('hashchange', onHash);
    win.addEventListener('keydown', onKey);
    onHash();
    this.#teardown = () => {
      win.removeEventListener('hashchange', onHash);
      win.removeEventListener('keydown', onKey);
      this.#teardown = null;
    };
    return this.#teardown;
  }

  #guard: ((route: RouteDef, params: RouteParams) => boolean) | null = null;
  #blocked: ((route: RouteDef, params: RouteParams) => void) | null = null;

  setNavigationGuard(guard: ((route: RouteDef, params: RouteParams) => boolean) | null, blocked?: (route: RouteDef, params: RouteParams) => void): () => void {
    this.#guard = guard;
    this.#blocked = blocked ?? null;
    return () => { this.#guard = null; this.#blocked = null; };
  }

  href(route: RouteDef, params: RouteParams = {}): string {
    if (route.id === 'owner-app' && params.appId) return `#/apps/${encodeURIComponent(params.appId)}`;
    if (route.id === 'owner-change' && params.appId) return `#/apps/${encodeURIComponent(params.appId)}/change${params.changeId ? `/${encodeURIComponent(params.changeId)}` : ''}`;
    return `#/${route.path}`;
  }

  navigate(route: RouteDef, win: Window = window, params: RouteParams = {}): void {
    if (this.#guard && !this.#guard(route, params)) { this.#blocked?.(route, params); return; }
    const next = this.href(route, params);
    if (win.location.hash === next) {
      this.current = route;
      this.params = params;
      return;
    }
    win.location.hash = next;
  }

  /** Canonicalize a just-saved draft without adding a second history entry. */
  replace(route: RouteDef, params: RouteParams, win: Window = window): void {
    if (this.#guard && !this.#guard(route, params)) { this.#blocked?.(route, params); return; }
    win.history.replaceState(null, '', this.href(route, params));
    this.current = route;
    this.params = params;
  }

  isActive(route: RouteDef): boolean {
    if (route.id === 'owner-apps' && (this.current.id === 'owner-app' || this.current.id === 'owner-change')) return true;
    return this.current.id === route.id;
  }
}

export const router = new Router();
