<script lang="ts">
  import MicButton from './MicButton.svelte'
  import { openSettings, toggleVisibility, injectText } from '$lib/tauri'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  const appWindow = getCurrentWindow()

  type LoopState = 'idle' | 'listening' | 'error'
  let loopState = $state<LoopState>('idle')
  let lastText = $state('')
  let lastTextTimer: ReturnType<typeof setTimeout> | null = null
  let recognition: any = null
  let restartTimer: ReturnType<typeof setTimeout> | null = null

  // Quick language toggle: cycle through JA → EN → ID
  const meetingLangs = [
    { code: 'ja-JP', label: 'JA' },
    { code: 'en-US', label: 'EN' },
    { code: 'id-ID', label: 'ID' },
  ]
  let langIdx = $state(0)

  function cycleLang() {
    langIdx = (langIdx + 1) % meetingLangs.length
    if (loopState === 'listening') {
      if (restartTimer) clearTimeout(restartTimer)
      recognition?.stop()
      recognition = null
      restartTimer = setTimeout(spawnSession, 100)
    }
  }

  async function startDrag(e: MouseEvent) {
    e.preventDefault()
    await appWindow.startDragging()
  }

  async function toggleLoop() {
    if (loopState === 'listening') stopLoop()
    else await startLoop()
  }

  async function startLoop() {
    const SR = (window as any).SpeechRecognition ?? (window as any).webkitSpeechRecognition
    if (!SR) { showText('Speech API not supported'); loopState = 'error'; return }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      stream.getTracks().forEach(t => t.stop())
    } catch {
      showText('Microphone permission denied')
      loopState = 'error'
      return
    }

    loopState = 'listening'
    spawnSession()
  }

  function spawnSession() {
    if (loopState !== 'listening') return

    const SR = (window as any).SpeechRecognition ?? (window as any).webkitSpeechRecognition
    recognition = new SR()
    recognition.continuous = false
    recognition.interimResults = false
    recognition.lang = meetingLangs[langIdx].code   // use active toggle lang

    recognition.onresult = async (event: any) => {
      const text: string = event.results[0][0].transcript.trim()
      if (!text) return
      showText(`[${meetingLangs[langIdx].label}] ${text}`)
      try { await injectText(text) } catch (e) { showText('inject: ' + e) }
    }

    recognition.onend = () => {
      if (loopState === 'listening') {
        restartTimer = setTimeout(spawnSession, 150)
      }
    }

    recognition.onerror = (event: any) => {
      if (event.error === 'no-speech' || event.error === 'aborted') return
      showText('STT error: ' + event.error)
      if (event.error === 'not-allowed') loopState = 'error'
    }

    recognition.start()
  }

  function stopLoop() {
    loopState = 'idle'
    if (restartTimer) { clearTimeout(restartTimer); restartTimer = null }
    recognition?.stop()
    recognition = null
  }

  function showText(text: string) {
    lastText = text
    if (lastTextTimer) clearTimeout(lastTextTimer)
    lastTextTimer = setTimeout(() => { lastText = '' }, 4000)
  }
</script>

<div class="flex flex-col gap-1 items-start">
  {#if lastText}
    <div class="px-3 py-1 bg-gray-800/90 rounded-full border border-gray-600/50 text-xs text-gray-300 max-w-xs truncate">
      {lastText}
    </div>
  {/if}

  <div class="flex items-center gap-2 px-3 h-12 bg-gray-900/90 backdrop-blur-sm rounded-full border border-gray-700/50 select-none" style="width: 300px;">
    <!-- Drag handle -->
    <button onmousedown={startDrag} class="cursor-grab active:cursor-grabbing text-gray-500 hover:text-gray-300 transition-colors p-1 rounded" title="Drag" aria-label="Drag handle">
      <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
        <circle cx="9" cy="7" r="1.5"/><circle cx="15" cy="7" r="1.5"/>
        <circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/>
        <circle cx="9" cy="17" r="1.5"/><circle cx="15" cy="17" r="1.5"/>
      </svg>
    </button>

    <!-- One-shot mic -->
    <MicButton />

    <!-- Language toggle: JA → EN → ID -->
    <button
      onclick={cycleLang}
      title="Switch language ({meetingLangs[langIdx].code})"
      aria-label="Switch STT language"
      class="w-8 h-8 flex items-center justify-center rounded-full text-xs font-bold transition-all
        {loopState === 'listening'
          ? 'bg-blue-600 text-white'
          : 'bg-gray-700 hover:bg-gray-600 text-gray-300'}"
    >
      {meetingLangs[langIdx].label}
    </button>

    <!-- Continuous loop -->
    <button
      onclick={toggleLoop}
      title={loopState === 'listening' ? `Stop (${meetingLangs[langIdx].label})` : 'Start continuous transcription'}
      aria-label={loopState === 'listening' ? 'Stop loop' : 'Start loop'}
      class="w-8 h-8 flex items-center justify-center rounded-full transition-all duration-200
        {loopState === 'listening'
          ? 'bg-red-600 animate-pulse shadow-lg shadow-red-500/40'
          : loopState === 'error'
            ? 'bg-orange-600'
            : 'bg-gray-700 hover:bg-gray-600'}"
    >
      {#if loopState === 'listening'}
        <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 24 24">
          <rect x="5" y="5" width="14" height="14" rx="2"/>
        </svg>
      {:else}
        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-white" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
          <path d="M12 12c-2-2.5-4-4-6-4a4 4 0 0 0 0 8c2 0 4-1.5 6-4z"/>
          <path d="M12 12c2 2.5 4 4 6 4a4 4 0 0 0 0-8c-2 0-4 1.5-6 4z"/>
        </svg>
      {/if}
    </button>

    <div class="flex-1"></div>

    <!-- Settings -->
    <button onclick={() => openSettings()} class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-white hover:bg-gray-700 transition-all" title="Settings" aria-label="Open settings">
      <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
    </button>

    <!-- Hide -->
    <button onclick={() => toggleVisibility()} class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-white hover:bg-red-600 transition-all" title="Hide" aria-label="Hide overlay">
      <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>
</div>
