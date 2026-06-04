import { render, screen, fireEvent, waitFor } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach } from 'vitest'

const fakePresets = [
  { id: 'generic', name: 'Generic Assistant', system_prompt: '...', response_format: 'Bullets', default_context_s: 90 },
  { id: 'interview-backend', name: 'Interview – Backend', system_prompt: '...', response_format: 'Code', default_context_s: 120 },
]

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'copilot_get_presets') return fakePresets
    if (cmd === 'copilot_start_session') return 42
    return undefined
  }),
}))

import StartSessionModal from '$lib/components/copilot/StartSessionModal.svelte'

describe('StartSessionModal', () => {
  beforeEach(() => vi.clearAllMocks())

  it('renders preset options after loading', async () => {
    render(StartSessionModal, { props: {
      open: true, apiUrl: 'x', apiKey: 'k', model: 'm',
      onClose: () => {}, onStarted: () => {},
    } })
    await waitFor(() => {
      expect(screen.getByText('Generic Assistant')).toBeTruthy()
      expect(screen.getByText('Interview – Backend')).toBeTruthy()
    })
  })

  it('calls copilot_start_session and onStarted on submit', async () => {
    const onStarted = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')
    render(StartSessionModal, { props: {
      open: true, apiUrl: 'x', apiKey: 'k', model: 'm',
      onClose: () => {}, onStarted,
    } })
    await waitFor(() => screen.getByText('Generic Assistant'))
    const startBtn = screen.getByText(/Mulai →/i)
    await fireEvent.click(startBtn)
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('copilot_start_session', expect.objectContaining({
        presetId: 'generic',
        apiUrl: 'x', apiKey: 'k', model: 'm',
      }))
      expect(onStarted).toHaveBeenCalledWith(42)
    })
  })
})
