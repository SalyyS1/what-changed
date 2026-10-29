import { collectorCopy } from "../settings/collector-settings";

/** Product-facing label for a backend collector id. */
export function collectorDisplayName(id: string): string {
  return collectorCopy(id).label;
}
