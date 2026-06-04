import { invoke } from '@tauri-apps/api/core'

export type ResponseFormat = 'Bullets' | 'Headline' | 'Code'

export interface Preset {
  id:                string
  name:              string
  system_prompt:     string
  response_format:   ResponseFormat
  default_context_s: number
}

export interface SessionStatus {
  id:         number
  preset_id:  string
  started_at: number
}

export interface CopilotSessionRow {
  id:               number
  preset_id:        string
  started_at:       number
  ended_at:         number | null
  context_window_s: number
  suggestion_count: number
}

export async function copilotGetPresets(): Promise<Preset[]> {
  return invoke<Preset[]>('copilot_get_presets')
}

export async function copilotSessionStatus(): Promise<SessionStatus | null> {
  return invoke<SessionStatus | null>('copilot_session_status')
}

export async function copilotStartSession(args: {
  presetId:        string
  contextWindowS:  number
  save:            boolean
  apiUrl:          string
  apiKey:          string
  model:           string
}): Promise<number> {
  return invoke<number>('copilot_start_session', {
    presetId:       args.presetId,
    contextWindowS: args.contextWindowS,
    save:           args.save,
    apiUrl:         args.apiUrl,
    apiKey:         args.apiKey,
    model:          args.model,
  })
}

export async function copilotStopSession(): Promise<void> {
  return invoke('copilot_stop_session')
}

export async function copilotForceRegenerate(): Promise<void> {
  return invoke('copilot_force_regenerate')
}

export async function copilotListSessions(): Promise<CopilotSessionRow[]> {
  return invoke<CopilotSessionRow[]>('copilot_list_sessions')
}
