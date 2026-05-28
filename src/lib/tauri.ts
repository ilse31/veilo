import { invoke } from '@tauri-apps/api/core'

export interface Config {
  site: string
  hotkey: string
  language: string
  opacity: number
  autostart: boolean
  position: [number, number] | null
}

export const DEFAULT_CONFIG: Config = {
  site: 'https://claude.ai',
  hotkey: 'Ctrl+Shift+Space',
  language: 'id-ID',
  opacity: 0.9,
  autostart: false,
  position: null,
}

export async function getConfig(): Promise<Config> {
  return invoke<Config>('get_config')
}

export async function saveConfig(config: Config): Promise<void> {
  return invoke('save_config', { config })
}

export async function injectText(text: string): Promise<void> {
  return invoke('inject_text', { text })
}

export async function setSite(site: string): Promise<void> {
  return invoke('set_site', { site })
}

export async function toggleVisibility(): Promise<void> {
  return invoke('toggle_visibility_cmd')
}

export async function openSettings(): Promise<void> {
  return invoke('open_settings_cmd')
}

export async function startStt(): Promise<void> {
  return invoke('start_stt_cmd')
}

export async function stopStt(): Promise<void> {
  return invoke('stop_stt_cmd')
}

export type Service = 'gpt' | 'claude' | 'translate'

export async function openServiceWebview(
  service: Service,
  url: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  return invoke('open_service_webview', { service, url, x, y, width, height })
}

export async function hideServiceWebview(service: Service): Promise<void> {
  return invoke('hide_service_webview', { service })
}

export async function setAllContentProtected(enabled: boolean): Promise<void> {
  return invoke('set_all_content_protected', { protected: enabled })
}

export async function injectToService(service: Service, text: string): Promise<void> {
  return invoke('inject_to_service', { service, text })
}

export async function getProfileName(): Promise<string | null> {
  return invoke<string | null>('get_profile_name')
}

export async function setProfileName(name: string): Promise<void> {
  return invoke('set_profile_name', { name })
}

// ── Chat history ─────────────────────────────────────────────────────────────

export interface ConvRow {
  id: number
  title: string
  msg_count: number
  created_at: number
}

export interface MsgRow {
  id: number
  role: 'user' | 'assistant'
  body: string
}

export async function listConversations(): Promise<ConvRow[]> {
  return invoke<ConvRow[]>('list_conversations')
}

export async function createConversation(title: string): Promise<number> {
  return invoke<number>('create_conversation', { title })
}

export async function updateConversationTitle(convId: number, title: string): Promise<void> {
  return invoke('update_conversation_title', { convId, title })
}

export async function deleteConversation(convId: number): Promise<void> {
  return invoke('delete_conversation', { convId })
}

export async function getMessages(convId: number): Promise<MsgRow[]> {
  return invoke<MsgRow[]>('get_messages', { convId })
}

export async function appendMessage(convId: number, role: 'user' | 'assistant', body: string): Promise<void> {
  return invoke('append_message', { convId, role, body })
}

export async function setClickThrough(enabled: boolean): Promise<void> {
  return invoke('set_click_through', { enabled })
}

export async function startGhostTyping(): Promise<void> {
  return invoke('start_ghost_typing_cmd')
}

export async function stopGhostTyping(): Promise<void> {
  return invoke('stop_ghost_typing_cmd')
}

export async function resizeServiceWebview(
  service: Service,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  return invoke('resize_service_webview', { service, x, y, width, height })
}

