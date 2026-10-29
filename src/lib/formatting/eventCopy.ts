import type { EventSummary, JsonValue, SystemEvent } from '../tauri/bindings';

export function eventTitle(event: Pick<EventSummary, 'user_friendly_summary'>): string {
  return event.user_friendly_summary;
}

export function technicalEventCopy(event: Pick<SystemEvent, 'technical_summary'>): string {
  return event.technical_summary;
}

export function displayValue(value: JsonValue | null): string {
  if (value === null) return 'Not recorded';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

export function collectorLabel(value: string): string {
  if (value === 'synthetic') return 'Synthetic event generator';
  return value
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}
