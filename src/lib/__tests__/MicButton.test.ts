import { describe, it, expect, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import { speechState } from '../stores'

beforeEach(() => { speechState.set('idle') })

describe('speech state machine transitions', () => {
  it('starts idle', () => { expect(get(speechState)).toBe('idle') })
  it('idle -> listening', () => {
    speechState.set('listening')
    expect(get(speechState)).toBe('listening')
  })
  it('listening -> processing', () => {
    speechState.set('listening')
    speechState.set('processing')
    expect(get(speechState)).toBe('processing')
  })
  it('processing -> idle', () => {
    speechState.set('processing')
    speechState.set('idle')
    expect(get(speechState)).toBe('idle')
  })
  it('any state -> error', () => {
    speechState.set('listening')
    speechState.set('error')
    expect(get(speechState)).toBe('error')
  })
  it('error -> idle recovery', () => {
    speechState.set('error')
    speechState.set('idle')
    expect(get(speechState)).toBe('idle')
  })
})
