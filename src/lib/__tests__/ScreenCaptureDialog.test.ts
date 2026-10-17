import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ScreenCaptureDialog from '$lib/components/copilot/ScreenCaptureDialog.svelte'
import { listMonitors, capturePreview, discardPreview, submitPreview } from '$lib/screen'

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setFocus: vi.fn(async () => {}) }),
}))
vi.mock('$lib/screen', () => ({
  listMonitors: vi.fn(), capturePreview: vi.fn(), discardPreview: vi.fn(), submitPreview: vi.fn(),
}))

beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(listMonitors).mockResolvedValue([{ index: 0, name: 'Monitor utama' }])
  vi.mocked(capturePreview).mockResolvedValue({
    id: 'shot-1', image: 'data:image/jpeg;base64,AAAA', monitor_name: 'Monitor utama',
  })
  vi.mocked(discardPreview).mockResolvedValue()
  vi.mocked(submitPreview).mockResolvedValue()
})
afterEach(cleanup)

describe('screen capture preview', () => {
  it('shows the captured image and sends it only after confirmation', async () => {
    render(ScreenCaptureDialog)
    await fireEvent.click(screen.getByRole('button', { name: 'Baca layar' }))
    expect(await screen.findByRole('option', { name: 'Monitor utama' })).toBeTruthy()
    await fireEvent.click(screen.getByRole('button', { name: 'Ambil pratinjau' }))
    expect(await screen.findByRole('img', { name: 'Pratinjau Monitor utama' })).toBeTruthy()
    expect(submitPreview).not.toHaveBeenCalled()
    await fireEvent.input(screen.getByRole('textbox', { name: 'Pertanyaan tentang layar (opsional)' }), {
      target: { value: 'Jawab dengan TypeScript' },
    })
    await fireEvent.click(screen.getByRole('button', { name: 'Kirim ke AI' }))
    await waitFor(() => expect(submitPreview).toHaveBeenCalledWith('shot-1', 'Jawab dengan TypeScript'))
  })

  it('discards the screenshot when cancelled', async () => {
    render(ScreenCaptureDialog)
    await fireEvent.click(screen.getByRole('button', { name: 'Baca layar' }))
    await screen.findByRole('option', { name: 'Monitor utama' })
    await fireEvent.click(screen.getByRole('button', { name: 'Ambil pratinjau' }))
    await screen.findByRole('img', { name: 'Pratinjau Monitor utama' })
    await fireEvent.click(screen.getByRole('button', { name: 'Batal' }))
    await waitFor(() => expect(discardPreview).toHaveBeenCalledWith('shot-1'))
    expect(submitPreview).not.toHaveBeenCalled()
  })
})
