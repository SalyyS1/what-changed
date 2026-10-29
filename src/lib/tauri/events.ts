/**
 * Typed listeners for events the backend pushes.
 *
 * Same seam as `client.ts` and for the same reason: `listen` dereferences the
 * Tauri internals on call, which do not exist in jsdom or in a plain browser
 * tab. Every subscription therefore returns an unlisten function — a no-op
 * when there is no backend — so a component's teardown is written the same way
 * whether or not it is running inside the app.
 */

import { listen } from '@tauri-apps/api/event';
import type { BatchNotice, MonitoringDto } from './bindings';
import { backendAvailable } from './client';

/** Names must match `app/event_bus.rs` and `app/tray.rs`. */
export const EVENTS = {
  systemEventAdded: 'system-event-added',
  collectorStatusChanged: 'collector-status-changed',
  analysisCompleted: 'analysis-completed',
  monitoringStateChanged: 'monitoring-state-changed',
  trayNavigate: 'tray-navigate',
  checkpointSaved: 'checkpoint-saved',
  notificationEvent: 'notification-event',
} as const;

export interface NotificationEvent {
  event_id: number;
  title: string;
  category: string;
}

/** Removes a subscription. Safe to call more than once. */
export type Unlisten = () => void;

const NOOP: Unlisten = () => {};

async function subscribe<T>(event: string, handler: (payload: T) => void): Promise<Unlisten> {
  if (!backendAvailable()) {
    return NOOP;
  }
  try {
    return await listen<T>(event, (message) => handler(message.payload));
  } catch {
    // A listener that cannot be registered is a feature that does not update,
    // not a reason for the screen that wanted it to fail to render.
    return NOOP;
  }
}

/**
 * Batches of new events, coalesced by the backend to at most one message per
 * 500 ms. The payload carries no event bodies: the frontend decides whether to
 * fetch, which is what makes a burst of a thousand inserts cost nothing here.
 */
export const onSystemEventAdded = (handler: (notice: BatchNotice) => void): Promise<Unlisten> =>
  subscribe(EVENTS.systemEventAdded, handler);

/** A collector changed state. */
export const onCollectorStatusChanged = (handler: (collectorId: string) => void): Promise<Unlisten> =>
  subscribe(EVENTS.collectorStatusChanged, handler);

/** An investigation finished. */
export const onAnalysisCompleted = (handler: (investigationId: number) => void): Promise<Unlisten> =>
  subscribe(EVENTS.analysisCompleted, handler);

/** Monitoring was paused or resumed, from the tray or from the interface. */
export const onMonitoringStateChanged = (
  handler: (state: MonitoringDto) => void,
): Promise<Unlisten> => subscribe(EVENTS.monitoringStateChanged, handler);

/** A tray menu item asked for a screen. The payload is a route id. */
export const onTrayNavigate = (handler: (route: string) => void): Promise<Unlisten> =>
  subscribe(EVENTS.trayNavigate, handler);

/** A checkpoint was saved by the screen, tray, or automatic trigger. */
export const onCheckpointSaved = (handler: (checkpointId: number) => void): Promise<Unlisten> =>
  subscribe(EVENTS.checkpointSaved, handler);

/** A delivered notification carries metadata for a future click bridge. */
export const onNotificationEvent = (
  handler: (event: NotificationEvent) => void,
): Promise<Unlisten> => subscribe(EVENTS.notificationEvent, handler);
