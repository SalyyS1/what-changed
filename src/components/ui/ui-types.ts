/**
 * Types shared between primitives and their callers. They live in a `.ts`
 * module rather than inside the `.svelte` script blocks because plain `tsc`
 * (part of `bun run check`) resolves `*.svelte` to an ambient declaration
 * and cannot see named type exports from a component file.
 */

export type IconName =
  | 'home'
  | 'timeline'
  | 'investigate'
  | 'checkpoint'
  | 'map'
  | 'settings'
  | 'diagnostics'
  | 'dev'
  | 'minimize'
  | 'maximize'
  | 'restore'
  | 'close'
  | 'chevron-down'
  | 'chevron-right'
  | 'check'
  | 'search'
  | 'x'
  | 'alert'
  | 'info'
  | 'refresh'
  | 'sun'
  | 'moon'
  | 'monitor';

export interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  tone?: 'neutral' | 'danger';
  disabled?: boolean;
  /** A visual break before this item. */
  separatorBefore?: boolean;
}

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface TabItem {
  value: string;
  label: string;
}
