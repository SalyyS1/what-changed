/**
 * The route table. Routes are data so the sidebar, the keyboard shortcuts,
 * the router and the dev harness all read one list and cannot disagree.
 *
 * The seven sidebar destinations are §2's product surface. Dev harnesses are
 * added only under `import.meta.env.DEV`, so a release build has neither the
 * harness code nor the route.
 */

export type RouteId =
  | 'home'
  | 'timeline'
  | 'investigations'
  | 'checkpoints'
  | 'system-map'
  | 'settings'
  | 'diagnostics'
  | 'dev-states'
  | 'dev-perf'
  | 'dev-system-map-perf';

export interface RouteDef {
  id: RouteId;
  /** Hash path without the leading `#/`. */
  path: string;
  label: string;
  /** Icon name resolved by `Sidebar`; kept as a string so routes stay data. */
  icon: 'home' | 'timeline' | 'investigate' | 'checkpoint' | 'map' | 'settings' | 'diagnostics' | 'dev';
  /** 1–7 for the sidebar entries; undefined for routes with no shortcut. */
  shortcut?: number;
  /** Shown in the sidebar. Dev routes are reachable by URL only. */
  inNav: boolean;
}

const home: RouteDef = { id: 'home', path: '', label: 'Home', icon: 'home', shortcut: 1, inNav: true };

const all: readonly RouteDef[] = [
  home,
  { id: 'timeline', path: 'timeline', label: 'Timeline', icon: 'timeline', shortcut: 2, inNav: true },
  { id: 'investigations', path: 'investigations', label: 'Investigations', icon: 'investigate', shortcut: 3, inNav: true },
  { id: 'checkpoints', path: 'checkpoints', label: 'Checkpoints', icon: 'checkpoint', shortcut: 4, inNav: true },
  { id: 'system-map', path: 'system-map', label: 'System Map', icon: 'map', shortcut: 5, inNav: true },
  { id: 'settings', path: 'settings', label: 'Settings', icon: 'settings', shortcut: 6, inNav: true },
  { id: 'diagnostics', path: 'diagnostics', label: 'Diagnostics', icon: 'diagnostics', shortcut: 7, inNav: true },
];

// Spread under the DEV guard so a release bundle does not even carry the
// route literal; the minifier drops the dead branch.
export const routes: readonly RouteDef[] = [
  ...all,
  ...(import.meta.env.DEV
    ? [
        { id: 'dev-states', path: 'dev/states', label: 'States harness', icon: 'dev', inNav: false } as const,
        { id: 'dev-perf', path: 'dev/perf', label: 'Performance harness', icon: 'dev', inNav: false } as const,
        { id: 'dev-system-map-perf', path: 'dev/system-map-perf', label: 'System Map performance harness', icon: 'dev', inNav: false } as const,
      ]
    : []),
];

export const navRoutes: readonly RouteDef[] = routes.filter((r) => r.inNav);

export const defaultRoute: RouteDef = home;

export function routeByPath(path: string): RouteDef | undefined {
  return routes.find((r) => r.path === path);
}

export function routeById(id: RouteId): RouteDef | undefined {
  return routes.find((r) => r.id === id);
}

export function routeByShortcut(n: number): RouteDef | undefined {
  return navRoutes.find((r) => r.shortcut === n);
}

export function hrefFor(route: RouteDef): string {
  return `#/${route.path}`;
}
