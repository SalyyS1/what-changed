import type { BatchNotice } from '../tauri/bindings';
import { onSystemEventAdded, type Unlisten } from '../tauri/events';

export class LiveEventsStore {
  #unlisten: Unlisten | null = null;

  async attach(handler: (notice: BatchNotice) => void): Promise<Unlisten> {
    this.#unlisten?.();
    this.#unlisten = await onSystemEventAdded(handler);
    return () => {
      this.#unlisten?.();
      this.#unlisten = null;
    };
  }
}

export const liveEvents = new LiveEventsStore();
