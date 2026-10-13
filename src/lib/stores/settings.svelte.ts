/**
 * Shared reactive settings state (Svelte 5 universal reactivity).
 * All components that import `settings` share the same instance.
 */
export const settings = $state({
  // Profile
  profileName: null as string | null,
  // Hotkey
  hotkey: 'Ctrl+Shift+Space',
  // AI API
  apiUrl:  'https://api.openai.com',
  apiKey:  '',
  model:   '',
  visionOverrides:          [] as string[],  // model ids the user marked "supports images"
  screenIncludeTranscript:  true,
  modelVision:              false,  // provider-reported image support of `modelVisionFor`
  modelVisionFor:           '',
  // Appearance
  opacity:          90,
  alwaysOnTop:      true,
  stealth:          false,
  dark:             false,
  contentProtected: true,   // hidden from screen capture by default
  autostart:        false,
  // Webview URLs
  urlGpt:       'https://chat.openai.com',
  urlClaude:    'https://claude.ai',
  urlTranslate: 'https://translate.google.com',
  // Copilot
  copilotContextS:      90,
  copilotAutoDismissS:  25,
  copilotMinIntervalS:  4,
  sttBackend:           'whisper-cloud' as 'whisper-cloud' | 'whisper-local' | 'deepgram' | 'windows-sr',
  whisperModel:         'whisper-1',
  whisperLocalUrl:      'http://localhost:8080',
  sttApiKey:            '',   // Deepgram key (separate from AI chat apiKey)
  audioSource:          'system' as 'system' | 'microphone' | 'both',
  micDevice:            '',   // '' = system default input
  sttLanguage:          '',   // '' = auto-detect; 'en', 'ja', 'id', etc.
})
