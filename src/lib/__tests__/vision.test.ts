import { describe, it, expect } from 'vitest'
import { parseOverrides, supportsImages, toggleOverride } from '$lib/vision'

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
})
