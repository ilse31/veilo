import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import NativeChat from '../components/hub/NativeChat.svelte'
import { deleteConversation, listConversations } from '../tauri'

vi.mock('../tauri', () => ({
  listConversations: vi.fn(), createConversation: vi.fn(), deleteConversation: vi.fn(),
  getMessages: vi.fn().mockResolvedValue([]), appendMessage: vi.fn(),
  updateConversationTitle: vi.fn(), chatSend: vi.fn(), chatStop: vi.fn(),
}))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(listConversations).mockResolvedValue([
    { id: 1, title: 'Pertama', msg_count: 2, created_at: 0 },
    { id: 2, title: 'Kedua', msg_count: 3, created_at: 0 },
  ])
  vi.mocked(deleteConversation).mockResolvedValue()
})
afterEach(cleanup)

it('deletes only checked conversations', async () => {
  render(NativeChat)
  await fireEvent.click(await screen.findByRole('checkbox', { name: 'Pilih obrolan Pertama' }))
  await fireEvent.click(screen.getByRole('button', { name: 'Hapus pilihan (1)' }))
  await waitFor(() => expect(screen.queryByText('Pertama')).toBeNull())
  expect(screen.getByText('Kedua')).toBeTruthy()
  expect(deleteConversation).toHaveBeenCalledExactlyOnceWith(1)
})

it('selects all and deletes the entire list', async () => {
  render(NativeChat)
  await fireEvent.click(await screen.findByRole('checkbox', { name: 'Pilih semua obrolan' }))
  expect(screen.getByRole('button', { name: 'Hapus pilihan (2)' })).toBeTruthy()
  await fireEvent.click(screen.getByRole('button', { name: 'Hapus semua' }))
  expect(await screen.findByText('Belum ada obrolan')).toBeTruthy()
  expect(deleteConversation).toHaveBeenCalledTimes(2)
})

it('keeps failed deletions visible and reports the error', async () => {
  vi.mocked(deleteConversation).mockRejectedValue(new Error('database error'))
  render(NativeChat)
  await fireEvent.click(await screen.findByRole('button', { name: 'Hapus semua' }))
  expect(await screen.findByRole('alert')).toBeTruthy()
  expect(screen.getByText('Pertama')).toBeTruthy()
  expect(screen.getByText('Kedua')).toBeTruthy()
})
