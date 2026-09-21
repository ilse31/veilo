import { cleanup, fireEvent, render, screen } from '@testing-library/svelte'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import Dashboard from '../components/hub/Dashboard.svelte'
import { settings } from '../stores/settings.svelte'
import { getUsageStats, listConversations } from '../tauri'

vi.mock('../tauri', () => ({ listConversations: vi.fn(), getUsageStats: vi.fn() }))

const nowS = () => Math.floor(Date.now() / 1000)

beforeEach(() => {
  vi.clearAllMocks()
  settings.contentProtected = true
  vi.mocked(listConversations).mockResolvedValue(
    Array.from({ length: 8 }, (_, i) => ({
      id: 8 - i, title: `Obrolan ${8 - i}`, msg_count: 2, created_at: nowS() - 120,
    })),
  )
  vi.mocked(getUsageStats).mockResolvedValue({
    messages_today: 142, messages_yesterday: 124, tokens_today: 38_200,
    tokens_estimated: false, avg_latency_ms: 820,
  })
})
afterEach(cleanup)

it('lists the six most recent conversations', async () => {
  render(Dashboard, { apiUrl: 'https://api.openai.com' })
  expect(await screen.findByText('Obrolan 8')).toBeTruthy()
  expect(screen.getByText('Obrolan 3')).toBeTruthy()
  expect(screen.queryByText('Obrolan 2')).toBeNull()
  expect(screen.getAllByText('2 menit')).toHaveLength(6)
})

it('opens the clicked conversation and the full list', async () => {
  const onOpenConversation = vi.fn()
  const onNav = vi.fn()
  render(Dashboard, { onOpenConversation, onNav })
  await fireEvent.click(await screen.findByText('Obrolan 7'))
  expect(onOpenConversation).toHaveBeenCalledExactlyOnceWith(7)
  await fireEvent.click(screen.getByRole('button', { name: /Lihat semua/ }))
  expect(onNav).toHaveBeenCalledExactlyOnceWith('chat')
})

it('shows usage KPIs', async () => {
  render(Dashboard)
  expect(await screen.findByText('142')).toBeTruthy()
  expect(screen.getByText('+18 vs kemarin')).toBeTruthy()
  expect(screen.getByText('38.2K')).toBeTruthy()
  expect(screen.getByText('820ms')).toBeTruthy()
})

it('marks token totals as approximate when they are estimated', async () => {
  vi.mocked(getUsageStats).mockResolvedValue({
    messages_today: 1, messages_yesterday: 0, tokens_today: 1_500,
    tokens_estimated: true, avg_latency_ms: 300,
  })
  render(Dashboard)
  expect(await screen.findByText('≈1.5K')).toBeTruthy()
  expect(screen.getByText('hari ini (perkiraan)')).toBeTruthy()
})

it('reflects connection and stealth state in the service list', async () => {
  settings.contentProtected = false
  render(Dashboard, { apiUrl: 'https://api.openai.com' })
  expect(await screen.findByText('api.openai.com')).toBeTruthy()
  expect(screen.getByText('Aktif')).toBeTruthy()
  expect(screen.getByText('Standby')).toBeTruthy()
})

it('shows an empty state and dashes when the backend has no data', async () => {
  vi.mocked(listConversations).mockResolvedValue([])
  vi.mocked(getUsageStats).mockResolvedValue({
    messages_today: 0, messages_yesterday: 0, tokens_today: 0,
    tokens_estimated: false, avg_latency_ms: null,
  })
  render(Dashboard)
  expect(await screen.findByText('sama seperti kemarin')).toBeTruthy()
  expect(screen.getByText(/Belum ada obrolan/)).toBeTruthy()
  expect(screen.getByText('—')).toBeTruthy()
})

it('still renders when the backend calls fail', async () => {
  vi.mocked(listConversations).mockRejectedValue(new Error('boom'))
  vi.mocked(getUsageStats).mockRejectedValue(new Error('boom'))
  render(Dashboard)
  expect(await screen.findByText(/Belum ada obrolan/)).toBeTruthy()
})
