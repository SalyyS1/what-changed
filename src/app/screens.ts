/**
 * Route id → screen component. Kept apart from `routes.ts` so the route table
 * stays plain data (importable by tests and the router without pulling in
 * every screen), while this file is the one place that pays the import cost.
 */

import type { Component } from 'svelte';
import type { AsyncState } from '../lib/types/async-state';
import CheckpointsScreen from '../features/checkpoints/CheckpointsScreen.svelte';
import DiagnosticsScreen from '../features/diagnostics/DiagnosticsScreen.svelte';
import HomeScreen from '../features/home/HomeScreen.svelte';
import InvestigationsScreen from '../features/investigation/InvestigationsScreen.svelte';
import SettingsScreen from '../features/settings/SettingsScreen.svelte';
import SystemMapScreen from '../features/system-map/SystemMapScreen.svelte';
import TimelineScreen from '../features/timeline/TimelineScreen.svelte';
import type { RouteId } from './routes';

// Every screen accepts an optional `data` so the dev harness can drive all
// four faces; the payload type differs per screen, hence the loose `any`.
export type ScreenComponent = Component<{ data?: AsyncState<any> }>;

export const navScreens: Record<Exclude<RouteId, 'dev-states' | 'dev-perf' | 'dev-system-map-perf' | 'owner-apps' | 'owner-app' | 'owner-change'>, ScreenComponent> = {
  home: HomeScreen,
  timeline: TimelineScreen,
  investigations: InvestigationsScreen,
  checkpoints: CheckpointsScreen,
  'system-map': SystemMapScreen,
  settings: SettingsScreen,
  diagnostics: DiagnosticsScreen,
};
