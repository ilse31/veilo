import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

const { setFocus, hide } = vi.hoisted(() => ({
  setFocus: vi.fn(async () => {}),
  hide: vi.fn(async () => {}),
}))

// Mock Tauri APIs before importing the component
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}))
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => {}),
}))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ hide, setFocus }),
}))

import AnswerCard from '$lib/components/copilot/AnswerCard.svelte'
import { settings } from '$lib/stores/settings.svelte'

afterEach(() => {
  cleanup()
  vi.useRealTimers()
})

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

  it('listens for screen capture and monitor selection events', async () => {
    const { listen } = await import('@tauri-apps/api/event')
    render(AnswerCard)
    await vi.waitFor(() => {
      expect(listen).toHaveBeenCalledWith('screen-capture-started', expect.any(Function))
      expect(listen).toHaveBeenCalledWith('screen-monitor-selected', expect.any(Function))
    })
  })

  it('activates the window only when the instruction input is selected', async () => {
    render(AnswerCard)
    const input = screen.getByRole('textbox', { name: 'Instruksi Copilot' })
    expect(setFocus).not.toHaveBeenCalled()
    await fireEvent.pointerDown(input, { button: 0 })
    await waitFor(() => expect(setFocus).toHaveBeenCalledOnce())
    expect(document.activeElement).toBe(input)
    await fireEvent.input(input, { target: { value: 'Jelaskan lebih rinci' } })
    expect((input as HTMLInputElement).value).toBe('Jelaskan lebih rinci')
  })

  it('applies the instruction once and explains how to use it with a screenshot', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(AnswerCard)
    const input = screen.getByRole('textbox', { name: 'Instruksi Copilot' })
    await fireEvent.input(input, { target: { value: '  Gunakan TypeScript.  ' } })
    await fireEvent.keyDown(input, { key: 'Enter' })
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('copilot_set_custom_instruction', {
      instruction: 'Gunakan TypeScript.',
    }))
    expect(invoke).not.toHaveBeenCalledWith('copilot_force_regenerate')
    expect(screen.getByRole('status').textContent).toContain('tangkap layar lagi')
  })

  it('keeps the card open while the user is editing an instruction', async () => {
    const { listen } = await import('@tauri-apps/api/event')
    const previousDismiss = settings.copilotAutoDismissS
    settings.copilotAutoDismissS = 1
    try {
      render(AnswerCard)
      await waitFor(() => expect(listen).toHaveBeenCalledWith('screen-monitor-selected', expect.any(Function)))
      const done = vi.mocked(listen).mock.calls.find(([name]) => name === 'copilot-suggest-done')![1]
      const input = screen.getByRole('textbox', { name: 'Instruksi Copilot' })
      await fireEvent.focus(input)
      vi.useFakeTimers()
      done({ event: 'copilot-suggest-done', id: 1, payload: { id: 'answer', question: 'Pertanyaan', full_text: 'Jawaban' } })
      await vi.advanceTimersByTimeAsync(1100)
      expect(hide).not.toHaveBeenCalled()
    } finally {
      settings.copilotAutoDismissS = previousDismiss
    }
  })
})
