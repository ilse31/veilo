import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import HubSettings from '../components/hub/HubSettings.svelte'
import { settings } from '../stores/settings.svelte'
import { testAiConnection } from '../tauri'
import { testAiVision } from '../screen'

vi.mock('../tauri', () => ({
  getConfig: vi.fn(), saveConfig: vi.fn(), setAllContentProtected: vi.fn(),
  setProfileValue: vi.fn(), testAiConnection: vi.fn(),
}))
vi.mock('../screen', () => ({ testAiVision: vi.fn() }))
vi.mock('../components/hub/webviewManager', () => ({ setServiceUrl: vi.fn() }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: vi.fn() }))

beforeEach(() => {
  vi.resetAllMocks()
  settings.apiUrl = 'https://example.com'
  settings.apiKey = ''
  settings.model = ''
  settings.visionOverrides = []
})
afterEach(cleanup)

describe('AI model selection', () => {
  it('tests actual image delivery for the selected model', async () => {
    vi.mocked(testAiConnection).mockResolvedValue({
      status: 200, models: [{ id: 'vision-model', vision: true }],
    })
    vi.mocked(testAiVision).mockResolvedValue({
      status: 'failed', message: 'Model menjawab seolah gambar tidak diterima.',
    })
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    await fireEvent.click(await screen.findByRole('button', { name: /^vision-model/ }))
    await fireEvent.click(screen.getByRole('button', { name: 'Uji gambar' }))
    expect(testAiVision).toHaveBeenCalledWith('https://example.com', '', 'vision-model')
    expect((await screen.findByRole('alert')).textContent).toContain('gambar tidak diterima')
  })
  it('marks a model for screen reading after a successful image test', async () => {
    vi.mocked(testAiConnection).mockResolvedValue({
      status: 200, models: [{ id: 'router-model', vision: false }],
    })
    vi.mocked(testAiVision).mockResolvedValue({ status: 'passed', message: 'Gambar terbaca.' })
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    await fireEvent.click(await screen.findByRole('button', { name: /^router-model/ }))
    await fireEvent.click(screen.getByRole('button', { name: 'Uji gambar' }))
    await waitFor(() => expect(settings.visionOverrides).toContain('router-model'))
    expect(screen.getByRole('status').textContent).toContain('klik Simpan')
  })
  it('opens the returned models without preselecting one, then selects a model', async () => {
    vi.mocked(testAiConnection).mockResolvedValue({
      status: 200,
      models: [{ id: 'model-a', vision: false }, { id: 'model-b', vision: true }],
    })
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    const option = await screen.findByRole('button', { name: /^model-b/ })
    expect(option.closest('details')?.open).toBe(true)
    expect(option.textContent).toContain('👁')
    expect(screen.getByRole('button', { name: /^model-a/ }).textContent).toContain('tanpa gambar')
    expect(settings.model).toBe('')
    await fireEvent.click(option)
    expect(settings.model).toBe('model-b')
    expect(option.closest('details')?.open).toBe(false)
  })

  it('ignores an in-flight response when the endpoint changes', async () => {
    let resolve!: (value: { status: number, models: { id: string, vision: boolean }[] }) => void
    vi.mocked(testAiConnection).mockReturnValue(new Promise(done => { resolve = done }))
    render(HubSettings)
    await fireEvent.click(screen.getByRole('button', { name: 'Uji koneksi' }))
    await fireEvent.input(screen.getByPlaceholderText('https://api.openai.com'), {
      target: { value: 'https://other.example.com' },
    })
    resolve({ status: 200, models: [{ id: 'old-model', vision: false }] })
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
