import { describe, it, expect } from 'vitest'

function buildHotkey(ctrl: boolean, meta: boolean, alt: boolean, shift: boolean, key: string): string {
  const parts: string[] = []
  if (ctrl) parts.push('Ctrl')
  if (meta) parts.push('Meta')
  if (alt) parts.push('Alt')
  if (shift) parts.push('Shift')
  parts.push(key === ' ' ? 'Space' : key.length === 1 ? key.toUpperCase() : key)
  return parts.join('+')
}

describe('buildHotkey', () => {
  it('builds Ctrl+Shift+Space', () => { expect(buildHotkey(true, false, false, true, ' ')).toBe('Ctrl+Shift+Space') })
  it('builds Ctrl+Shift+A', () => { expect(buildHotkey(true, false, false, true, 'a')).toBe('Ctrl+Shift+A') })
  it('builds Alt+F4', () => { expect(buildHotkey(false, false, true, false, 'F4')).toBe('Alt+F4') })
  it('builds Meta+Space', () => { expect(buildHotkey(false, true, false, false, ' ')).toBe('Meta+Space') })
  it('single key F9', () => { expect(buildHotkey(false, false, false, false, 'F9')).toBe('F9') })
})
