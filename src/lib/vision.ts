import type { ModelInfo } from '$lib/tauri'

export function parseOverrides(raw: string | null): string[] {
  if (!raw) return []
  try {
    const v = JSON.parse(raw)
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []
  } catch {
    return []
  }
}

export function supportsImages(m: ModelInfo, overrides: string[]): boolean {
  return m.vision || overrides.includes(m.id)
}

export function toggleOverride(overrides: string[], id: string): string[] {
  return overrides.includes(id) ? overrides.filter(x => x !== id) : [...overrides, id]
}
