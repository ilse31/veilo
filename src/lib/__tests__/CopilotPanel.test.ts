import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'copilot_session_status') return null
    if (cmd === 'copilot_list_sessions') return []
    return undefined
  }),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}))

import CopilotPanel from '$lib/components/copilot/CopilotPanel.svelte'

const aiProps = { apiUrl: 'https://api.openai.com', apiKey: '', model: 'gpt-4o-mini' }

describe('CopilotPanel', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  it('renders "Mulai Sesi" button when idle', async () => {
    render(CopilotPanel, { props: { onStartClick: () => {}, ...aiProps } })
    await waitFor(() => {
      expect(screen.getByText(/Mulai Sesi/i)).toBeTruthy()
    })
  })

  it('calls onStartClick when start button clicked', async () => {
    const onStartClick = vi.fn()
    render(CopilotPanel, { props: { onStartClick, ...aiProps } })
    const btn = await waitFor(() => screen.getByText(/Mulai Sesi/i))
    await fireEvent.click(btn)
    expect(onStartClick).toHaveBeenCalled()
  })
})
