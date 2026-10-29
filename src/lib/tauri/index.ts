/**
 * The backend seam. Everything outside this directory imports from here, and
 * a test fails the build if anything outside it imports `@tauri-apps/api`
 * directly — an untyped, unvalidated backend call is exactly what the typed
 * surface exists to prevent.
 */

export * from './bindings';
export * from './client';
export * from './dialog';
export * from './events';
export { windowControls } from './window-controls';
