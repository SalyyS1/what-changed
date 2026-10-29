import { formatDuration } from './time';

export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  const deltaMinutes = Math.round(Math.abs(now - timestamp) / 60_000);
  const phrase = formatDuration(deltaMinutes);
  return timestamp <= now ? `${phrase} ago` : `in ${phrase}`;
}

export function formatCorrelation(minutes: number, subject: string): string {
  return `${formatDuration(minutes)} before ${subject}`;
}
