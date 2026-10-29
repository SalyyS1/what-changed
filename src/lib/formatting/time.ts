const clock = new Intl.DateTimeFormat('en-US', {
  hour: 'numeric',
  minute: '2-digit',
});

const full = new Intl.DateTimeFormat('en-US', {
  weekday: 'long',
  month: 'short',
  day: 'numeric',
  year: 'numeric',
  hour: 'numeric',
  minute: '2-digit',
});

const day = new Intl.DateTimeFormat('en-US', {
  weekday: 'long',
  month: 'short',
  day: 'numeric',
});

const hour = new Intl.DateTimeFormat('en-US', { hour: 'numeric' });

function localDateKey(timestamp: number): string {
  const value = new Date(timestamp);
  return `${value.getFullYear()}-${value.getMonth()}-${value.getDate()}`;
}

export function formatClock(timestamp: number): string {
  return clock.format(timestamp);
}

export function formatAbsoluteTime(timestamp: number): string {
  return full.format(timestamp).replace(' at ', ', ');
}

export function formatHour(timestamp: number): string {
  return hour.format(timestamp);
}

export function formatDayHeader(timestamp: number, now = Date.now()): string {
  if (localDateKey(timestamp) === localDateKey(now)) return 'Today';
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (localDateKey(timestamp) === localDateKey(yesterday.getTime())) return 'Yesterday';
  return day.format(timestamp);
}

export function formatDuration(minutes: number): string {
  const value = Math.max(0, Math.round(minutes));
  if (value < 1) return 'less than a minute';
  if (value === 1) return '1 minute';
  if (value < 60) return `${value} minutes`;
  const hours = Math.floor(value / 60);
  const rest = value % 60;
  return rest === 0
    ? `${hours} ${hours === 1 ? 'hour' : 'hours'}`
    : `${hours} ${hours === 1 ? 'hour' : 'hours'} ${rest} minutes`;
}
