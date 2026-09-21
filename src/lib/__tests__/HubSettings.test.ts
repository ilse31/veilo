import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import HubSettings from '../components/hub/HubSettings.svelte'
import { settings } from '../stores/settings.svelte'
import { testAiConnection } from '../tauri'

vi.mock('../tauri', () => ({
  getConfig: vi.fn(), saveConfig: vi.fn(), setAllContentProtected: vi.fn(),
  setProfileValue: vi.fn(), testAiConnection: vi.fn(),
}))
vi.mock('../components/hub/webviewManager', () => ({ setServiceUrl: vi.fn() }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: vi.fn() }))

beforeEach(() => {
  vi.resetAllMocks()
  settings.apiUrl = 'https://example.com'
  settings.apiKey = ''
  settings.model = ''
})
afterEach(cleanup)

describe('AI model selection', () => {
  it('opens the returned models without preselecting one, then selects a model', async () => {
    vi.mocked(testAiConnection).mockResolvedValue({ status: 200, models: ['model-a', 'model-b'] })
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    const option = await screen.findByRole('button', { name: 'model-b' })
    expect(option.closest('details')?.open).toBe(true)
    expect(settings.model).toBe('')
    await fireEvent.click(option)
    expect(settings.model).toBe('model-b')
    expect(option.closest('details')?.open).toBe(false)
  })

  it('ignores an in-flight response when the endpoint changes', async () => {
    let resolve!: (value: { status: number, models: string[] }) => void
    vi.mocked(testAiConnection).mockReturnValue(new Promise(done => { resolve = done }))
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    await fireEvent.input(screen.getByPlaceholderText('https://api.openai.com'), {
      target: { value: 'https://other.example.com' },
    })
    resolve({ status: 200, models: ['old-model'] })
    await waitFor(() => expect((screen.getByRole('button', { name: 'Uji koneksi' }) as HTMLButtonElement).disabled).toBe(false))
    expect(screen.queryByText('old-model')).toBeNull()
  })

  it.each([
    [{ status: 200, models: [] }, 'Koneksi berhasil, tetapi tidak ada model tersedia.'],
    [{ status: 401, models: [] }, 'HTTP 401'],
  ])('shows empty and error responses', async (result, message) => {
    vi.mocked(testAiConnection).mockResolvedValue(result)
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    expect(await screen.findByText(message, { exact: false })).toBeTruthy()
    expect(screen.queryByText('Pilih model')).toBeNull()
  })
})
