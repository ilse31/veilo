import { describe, expect, it } from 'vitest'
import {
  formatDelta, formatLatency, formatTokens, hostOf, relativeTime, todayStart,
} from '../dashboardFormat'

// Wednesday 2026-09-16 15:00 local time.
const NOW = new Date(2026, 8, 16, 15, 0, 0).getTime()
const at = (y: number, m: number, d: number, h = 12) => new Date(y, m, d, h).getTime() / 1000

describe('relativeTime', () => {
  it('handles sub-minute, minutes and hours today', () => {
    const s = NOW / 1000
    expect(relativeTime(s - 20, NOW)).toBe('Baru saja')
    expect(relativeTime(s - 120, NOW)).toBe('2 menit')
    expect(relativeTime(s - 3 * 3600, NOW)).toBe('3 jam')
  })

  it('says Kemarin for any time yesterday, even under 24h ago', () => {
    expect(relativeTime(at(2026, 8, 15, 23), NOW)).toBe('Kemarin')
    expect(relativeTime(at(2026, 8, 15, 1), NOW)).toBe('Kemarin')
  })

  it('counts days for the past week, then falls back to a date', () => {
    expect(relativeTime(at(2026, 8, 13), NOW)).toBe('3 hari')
    expect(relativeTime(at(2026, 8, 1), NOW)).toMatch(/1/)
    expect(relativeTime(at(2026, 8, 1), NOW)).not.toContain('hari')
  })
})

it('todayStart is local midnight', () => {
  expect(todayStart(NOW)).toBe(new Date(2026, 8, 16).getTime() / 1000)
})

it('formats token counts', () => {
  expect(formatTokens(842)).toBe('842')
  expect(formatTokens(38_200)).toBe('38.2K')
  expect(formatTokens(1_250_000)).toBe('1.3M')
})

it('formats latency, with a dash when unknown', () => {
  expect(formatLatency(null)).toBe('—')
  expect(formatLatency(820)).toBe('820ms')
  expect(formatLatency(1250)).toBe('1.3s')
})

it('formats the day-over-day delta', () => {
  expect(formatDelta(142, 124)).toBe('+18 vs kemarin')
  expect(formatDelta(3, 10)).toBe('−7 vs kemarin')
  expect(formatDelta(5, 5)).toBe('sama seperti kemarin')
})

it('extracts the host from URLs', () => {
  expect(hostOf('https://api.openai.com/v1')).toBe('api.openai.com')
  expect(hostOf('not a url')).toBe('not a url')
})
