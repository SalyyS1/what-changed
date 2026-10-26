/**
 * Hash router. Deliberately small: the app is a static SPA inside a webview,
 * so there is no server, no history API worth the complexity, and no need
 * for a framework. The URL is `#/<path>`; anything unknown resolves to Home
 * rather than a 404, because a desktop app has no "page not found".
 */

import { defaultRoute, routeByPath, routeByShortcut, type RouteDef } from '../../app/routes';

function parseHash(hash: string): RouteDef {
  const path = hash.replace(/^#\/?/, '').split('?')[0]?.replace(/\/+$/, '') ?? '';
  return routeByPath(path) ?? defaultRoute;
}

export class Router {
  current = $state<RouteDef>(parseHash(globalThis.location?.hash ?? ''));
  #teardown: (() => void) | null = null;

  /** Subscribe to the window; returns the teardown. Idempotent per instance. */
  attach(win: Window = window): () => void {
    this.#teardown?.();
    const onHash = () => {
      this.current = parseHash(win.location.hash);
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

  navigate(route: RouteDef, win: Window = window): void {
    const next = `#/${route.path}`;
    if (win.location.hash === next) {
      this.current = route;
      return;
    }
    win.location.hash = next;
  }

  isActive(route: RouteDef): boolean {
    return this.current.id === route.id;
  }
}

export const router = new Router();
