import { describe, it, expect } from 'vitest'
import { parseOverrides, supportsImages, toggleOverride, resolveModelVision } from '$lib/vision'

describe('vision helpers', () => {
  it('parses overrides defensively', () => {
    expect(parseOverrides(null)).toEqual([])
    expect(parseOverrides('not json')).toEqual([])
    expect(parseOverrides('{"a":1}')).toEqual([])
    expect(parseOverrides('["a",3,"b"]')).toEqual(['a', 'b'])
  })
  it('override forces support on', () => {
    expect(supportsImages({ id: 'x', vision: false }, [])).toBe(false)
    expect(supportsImages({ id: 'x', vision: false }, ['x'])).toBe(true)
    expect(supportsImages({ id: 'y', vision: true }, [])).toBe(true)
  })
  it('toggles without mutating', () => {
    const a = ['x']
    expect(toggleOverride(a, 'y')).toEqual(['x', 'y'])
    expect(toggleOverride(a, 'x')).toEqual([])
    expect(a).toEqual(['x'])
  })

  it('resolveModelVision uses provider flag when model is listed', () => {
    const models = [{ id: 'mistral-small', vision: true }, { id: 'txt', vision: false }]
    const prev = { flag: false, forModel: '' }
    expect(resolveModelVision(models, 'mistral-small', [], prev)).toEqual({ flag: true, forModel: 'mistral-small' })
    expect(resolveModelVision(models, 'txt', [], prev)).toEqual({ flag: false, forModel: 'txt' })
    expect(resolveModelVision(models, 'txt', ['txt'], prev).flag).toBe(true)
  })
  it('resolveModelVision keeps prev flag only for the same unlisted model', () => {
    expect(resolveModelVision([], 'm', [], { flag: true, forModel: 'm' })).toEqual({ flag: true, forModel: 'm' })
    expect(resolveModelVision([], 'm', [], { flag: true, forModel: 'old' })).toEqual({ flag: false, forModel: 'm' })
  })
})
