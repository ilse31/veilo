import { writable, derived } from 'svelte/store'
import type { Config } from './tauri'
import { DEFAULT_CONFIG } from './tauri'

export type SpeechState = 'idle' | 'listening' | 'processing' | 'error'

export const speechState = writable<SpeechState>('idle')
export const transcript = writable<string>('')
export const config = writable<Config>({ ...DEFAULT_CONFIG })

export const isRecording = derived(
  speechState,
  ($state) => $state === 'listening'
)
