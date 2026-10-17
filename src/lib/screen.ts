import { invoke } from '@tauri-apps/api/core'

export interface MonitorInfo {
  index: number
  name: string
}

export interface CapturePreview {
  id: string
  image: string
  monitor_name: string
}

export type VisionProbeResult = {
  status: 'passed' | 'failed' | 'inconclusive'
  message: string
}

export const listMonitors = () => invoke<MonitorInfo[]>('screen_list_monitors')
export const capturePreview = (index: number) =>
  invoke<CapturePreview>('screen_capture_preview', { index })
export const discardPreview = (previewId: string) =>
  invoke<void>('screen_discard_preview', { previewId })
export const submitPreview = (previewId: string, question: string) =>
  invoke<void>('screen_submit_preview', { previewId, question })
export const askCopilot = (question: string) =>
  invoke<void>('copilot_ask_text', { question })
export const testAiVision = (apiUrl: string, apiKey: string, model: string) =>
  invoke<VisionProbeResult>('test_ai_vision', { apiUrl, apiKey, model })
