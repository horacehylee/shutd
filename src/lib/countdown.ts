/** Pure time formatting for the panel and toast; tests cover these. */

export function formatClock(date: Date): string {
  const h = date.getHours().toString().padStart(2, '0');
  const m = date.getMinutes().toString().padStart(2, '0');
  return `${h}:${m}`;
}

export function humanRemaining(remainingMs: number): string {
  const total = Math.max(0, Math.ceil(remainingMs / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) return `${h} h ${m.toString().padStart(2, '0')} m`;
  if (m > 0) return `${m} min ${s.toString().padStart(2, '0')} s`;
  return `${s} s`;
}

export function mmss(remainingMs: number): string {
  const total = Math.max(0, Math.ceil(remainingMs / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export function dayLabel(shutdownAt: Date, now: Date): string {
  const day = (d: Date) => d.getFullYear() * 10000 + d.getMonth() * 100 + d.getDate();
  return day(shutdownAt) === day(now) ? 'today' : 'tomorrow';
}

/** Fraction of the day elapsed toward the shutdown time, 0-100. */
