/**
 * Shared reactive settings state (Svelte 5 universal reactivity).
 * All components that import `settings` share the same instance.
 */
export const settings = $state({
  // Profile
  profileName: null as string | null,
  // AI API
  apiUrl:  'https://api.openai.com',
  apiKey:  '',
  model:   'gpt-4o-mini',
  // Appearance
  opacity:          90,
  alwaysOnTop:      true,
  stealth:          false,
  dark:             false,
  contentProtected: true,   // hidden from screen capture by default
  // Webview URLs
  urlGpt:       'https://chat.openai.com',
  urlClaude:    'https://claude.ai',
  urlTranslate: 'https://translate.google.com',
  // Copilot
  copilotContextS:      90,
  copilotAutoDismissS:  25,
  copilotMinIntervalS:  4,
  sttBackend:           'whisper-cloud' as const,
  whisperModel:         'whisper-1',
})
