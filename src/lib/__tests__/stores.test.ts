import { describe, it, expect, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import { speechState, transcript, isRecording, config } from '../stores'
import { DEFAULT_CONFIG } from '../tauri'

beforeEach(() => {
  speechState.set('idle')
  transcript.set('')
  config.set(DEFAULT_CONFIG)
})

describe('speechState', () => {
  it('starts as idle', () => {
    expect(get(speechState)).toBe('idle')
  })
  it('can be set to listening', () => {
    speechState.set('listening')
    expect(get(speechState)).toBe('listening')
  })
  it('can be set to error', () => {
    speechState.set('error')
    expect(get(speechState)).toBe('error')
  })
})

describe('isRecording derived store', () => {
  it('is false when idle', () => {
    speechState.set('idle')
    expect(get(isRecording)).toBe(false)
  })
  it('is true only when listening', () => {
    speechState.set('listening')
    expect(get(isRecording)).toBe(true)
  })
  it('is false when processing', () => {
    speechState.set('processing')
    expect(get(isRecording)).toBe(false)
  })
})

describe('config store defaults', () => {
  it('has claude.ai as default site', () => {
    expect(get(config).site).toBe('https://claude.ai')
  })
  it('has id-ID as default language', () => {
    expect(get(config).language).toBe('id-ID')
  })
})
