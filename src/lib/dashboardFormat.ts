/** Formatting helpers for the Dashboard's history and system-status cards. */

const DAY = 86_400

function startOfLocalDay(ms: number): number {
  const d = new Date(ms)
  d.setHours(0, 0, 0, 0)
  return Math.floor(d.getTime() / 1000)
}

/** Local midnight for `now` as unix seconds (the `today_start` the backend expects). */
export function todayStart(now: number = Date.now()): number {
  return startOfLocalDay(now)
}

/** Indonesian relative time for a unix-seconds timestamp: "2 menit", "1 jam", "Kemarin". */
export function relativeTime(ts: number, now: number = Date.now()): string {
  const nowS = Math.floor(now / 1000)
  const diff = nowS - ts
  if (diff < 60) return 'Baru saja'
  if (diff < 3600) return `${Math.floor(diff / 60)} menit`
  const today = startOfLocalDay(now)
  if (ts >= today) return `${Math.floor(diff / 3600)} jam`
  if (ts >= today - DAY) return 'Kemarin'
  const days = Math.ceil((today - ts) / DAY)
  if (days < 7) return `${days} hari`
  return new Date(ts * 1000).toLocaleDateString('id-ID', { day: 'numeric', month: 'short' })
}

export function formatTokens(n: number): string {
  if (n < 1000) return String(n)
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`
  return `${(n / 1_000_000).toFixed(1)}M`
}

export function formatLatency(ms: number | null): string {
  if (ms === null) return '—'
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`
}

export function formatDelta(today: number, yesterday: number): string {
  const d = today - yesterday
  if (d === 0) return 'sama seperti kemarin'
  return `${d > 0 ? '+' : '−'}${Math.abs(d)} vs kemarin`
}

/** Host part of a URL for display; falls back to the raw string when unparsable. */
export function hostOf(url: string): string {
  try {
    return new URL(url).host
  } catch {
    return url
  }
}
