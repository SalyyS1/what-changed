import type { CheckpointDto } from "../../lib/tauri/bindings";
import { collectorDisplayName } from "./collector-display";

/** Presentation-only checkpoint shape. Backend DTOs stay snake_case at the IPC seam. */
export interface CheckpointRow {
  id: number;
  name: string;
  createdAt: number;
  origin: "manual" | "automatic";
  reason: string | null;
  itemCount: number;
  durationMs: number;
  partial: boolean;
  missingCollectors: string[];
}

export function checkpointRow(dto: CheckpointDto): CheckpointRow {
  return {
    id: dto.id,
    name: dto.name,
    createdAt: dto.created_at,
    origin: dto.kind === "automatic" ? "automatic" : "manual",
    reason: dto.reason,
    itemCount: dto.item_count,
    durationMs: dto.duration_ms,
    partial: dto.partial,
    missingCollectors: dto.missing_collectors.map(collectorDisplayName),
  };
}

export function formatCheckpointAge(
  timestamp: number,
  now = Date.now(),
): string {
  const elapsed = Math.max(0, now - timestamp);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (elapsed < minute) return "Just now";
  if (elapsed < hour) return `${Math.floor(elapsed / minute)}m ago`;
  if (elapsed < day) return `${Math.floor(elapsed / hour)}h ago`;
  if (elapsed < 7 * day) return `${Math.floor(elapsed / day)}d ago`;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: elapsed > 365 * day ? "numeric" : undefined,
    hour: "numeric",
    minute: "2-digit",
  }).format(timestamp);
}
