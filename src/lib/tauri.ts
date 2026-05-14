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
