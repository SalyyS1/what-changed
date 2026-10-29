import { failed, loading, ready, type AsyncState } from '../types/async-state';
import type { ChangeMapNode, EventDetailsDto } from '../tauri/bindings';
import { getEventDetails, getRelatedEvents, toAppError } from '../tauri/client';

const TECHNICAL_KEY = 'what-changed:technical-details-open';

interface EventDetailsDependencies {
  fetchDetails: (eventId: number) => Promise<EventDetailsDto>;
}

export class EventDetailsStore {
  open = $state(false);
  state = $state<AsyncState<EventDetailsDto>>(loading());
  technicalOpen = $state(globalThis.localStorage?.getItem(TECHNICAL_KEY) === 'true');
  snapshotOnly = $state(false);
  returnFocus: HTMLElement | null = null;
  #deps: EventDetailsDependencies;
  #eventId: number | null = null;
  #request = 0;

  constructor(dependencies: EventDetailsDependencies = {
    fetchDetails: async (eventId) => {
      const details = await getEventDetails(eventId);
      const related = await getRelatedEvents(eventId).catch(() => details.related);
      return { ...details, related };
    },
  }) {
    this.#deps = dependencies;
  }

  async show(eventId: number, opener?: HTMLElement | null): Promise<void> {
    const request = ++this.#request;
    const wasOpen = this.open;
    this.#eventId = eventId;
    this.snapshotOnly = false;
    if (!wasOpen) this.returnFocus = opener ?? (document.activeElement as HTMLElement | null);
    this.open = true;
    this.state = loading();
    try {
      const details = await this.#deps.fetchDetails(eventId);
      if (request !== this.#request) return;
      this.state = ready(details);
    } catch (reason) {
      if (request !== this.#request) return;
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.reload() : undefined);
    }
  }

  /** Opens the immutable investigation summary when the source event has expired. */
  showSnapshot(node: ChangeMapNode, opener?: HTMLElement | null): void {
    this.#request += 1;
    this.#eventId = null;
    if (!this.open) this.returnFocus = opener ?? (document.activeElement as HTMLElement | null);
    this.snapshotOnly = true;
    this.open = true;
    this.state = ready({
      event: {
        id: node.event_id,
        timestamp: node.timestamp_ms,
        category: node.category,
        event_type: 'modified',
        entity_type: 'system',
        entity_id: node.id,
        entity_display_name: node.label,
        severity: node.importance === 'critical' || node.importance === 'important' ? 'warning' : 'info',
        importance: node.importance,
        source_collector: 'saved investigation',
        before: null,
        after: null,
        field: null,
        metadata: null,
        related_entity_ids: [],
        user_friendly_summary: node.label,
        technical_summary: 'Summary retained by the saved investigation after source-event expiry.',
        confidence: 'low',
        sensitive: false,
        occurred_at_is_uncertain: false,
      },
      related: [],
      prior_occurrences: 0,
    });
  }

  close(): void {
    this.#request += 1;
    this.open = false;
  }

  toggleTechnical(): void {
    this.technicalOpen = !this.technicalOpen;
    globalThis.localStorage?.setItem(TECHNICAL_KEY, String(this.technicalOpen));
  }

  private async reload(): Promise<void> {
    if (this.#eventId !== null) await this.show(this.#eventId, this.returnFocus);
  }
}

export const eventDetails = new EventDetailsStore();
