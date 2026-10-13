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

export interface ModelVision { flag: boolean; forModel: string }

/**
 * Vision flag to persist for the selected model. Uses the provider's
 * modalities when the model is in the fetched list; otherwise keeps the
 * previous flag if it was computed for this same model, else falls back to
 * the manual override.
 */
export function resolveModelVision(
  models: ModelInfo[], model: string, overrides: string[], prev: ModelVision,
): ModelVision {
  const info = models.find(m => m.id === model)
  if (info) return { flag: supportsImages(info, overrides), forModel: model }
  if (prev.forModel === model) return { flag: prev.flag || overrides.includes(model), forModel: model }
  return { flag: overrides.includes(model), forModel: model }
}
