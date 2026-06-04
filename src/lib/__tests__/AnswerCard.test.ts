import { render, screen, waitFor } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock Tauri APIs before importing the component
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}))
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => {}),
}))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ hide: vi.fn() }),
}))

import AnswerCard from '$lib/components/copilot/AnswerCard.svelte'

describe('AnswerCard', () => {
  beforeEach(() => vi.clearAllMocks())

  it('renders empty placeholder when no suggestion', () => {
    render(AnswerCard)
    expect(screen.getByText(/menunggu/i)).toBeTruthy()
  })

  it('subscribes to copilot-suggest-delta on mount', async () => {
    const { listen } = await import('@tauri-apps/api/event')
    render(AnswerCard)
    // setup() is async — wait for all three listen() calls to complete
    await waitFor(() => {
      expect(listen).toHaveBeenCalledWith('copilot-suggest-delta', expect.any(Function))
      expect(listen).toHaveBeenCalledWith('copilot-suggest-done', expect.any(Function))
      expect(listen).toHaveBeenCalledWith('copilot-session-ended', expect.any(Function))
    })
  })
})
