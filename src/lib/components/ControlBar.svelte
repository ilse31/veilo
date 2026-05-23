<script lang="ts">
  import MicButton from './MicButton.svelte'
  import { openSettings, toggleVisibility, injectText, setSite, getConfig } from '$lib/tauri'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { onMount } from 'svelte'

  const appWindow = getCurrentWindow()

  type LoopState = 'idle' | 'listening' | 'error'
  let loopState = $state<LoopState>('idle')
  let lastText   = $state('')
  let lastTextTimer: ReturnType<typeof setTimeout> | null = null
  let recognition: any = null
  let restartTimer: ReturnType<typeof setTimeout> | null = null

  const langs = [
    { code: 'ja-JP', label: 'JA' },
    { code: 'en-US', label: 'EN' },
    { code: 'id-ID', label: 'ID' },
  ]
  let langIdx = $state(0)

  const sites = [
    { label: 'Claude', short: 'CL', url: 'https://claude.ai', color: 'text-orange-400' },
    { label: 'ChatGPT', short: 'GP', url: 'https://chatgpt.com', color: 'text-green-400' },
    { label: 'Translate', short: 'GT', url: 'https://translate.google.com', color: 'text-blue-400' },
  ]
  let activeSiteUrl = $state('https://claude.ai')
  let showSiteMenu  = $state(false)

  onMount(async () => {
    try {
      const cfg = await getConfig()
      if (cfg.site) activeSiteUrl = cfg.site
    } catch {}
  })

  async function switchSite(url: string) {
    activeSiteUrl = url
    showSiteMenu  = false
    try { await setSite(url) } catch (e) { showText('Site error: ' + e) }
  }

  function activeSiteShort() {
    return sites.find(s => s.url === activeSiteUrl)?.short ?? '??'
  }

  function setLang(i: number) {
    langIdx = i
    if (loopState === 'listening') {
      if (restartTimer) clearTimeout(restartTimer)
      recognition?.stop()
      recognition = null
      restartTimer = setTimeout(spawnSession, 80)
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
      const s = await navigator.mediaDevices.getUserMedia({ audio: true })
      s.getTracks().forEach(t => t.stop())
    } catch {
      showText('Mic permission denied')
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
    recognition.continuous    = false
    recognition.interimResults = false
    recognition.lang           = langs[langIdx].code

    recognition.onresult = async (event: any) => {
      const text = event.results[0][0].transcript.trim()
      if (!text) return
      showText(text)
      try { await injectText(text) } catch (e) { showText('inject error: ' + e) }
    }

    recognition.onend = () => {
      if (loopState === 'listening') restartTimer = setTimeout(spawnSession, 150)
    }

    recognition.onerror = (event: any) => {
      if (event.error === 'no-speech' || event.error === 'aborted') return
      showText('Error: ' + event.error)
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

<div class="flex flex-col gap-1.5 items-start">

  <!-- Site picker panel (appears above bar) -->
  {#if showSiteMenu}
    <div class="flex items-center gap-1 px-2 py-1.5 bg-gray-900/98 border border-gray-700/60 rounded-2xl shadow-xl backdrop-blur-md"
         style="width:340px">
      <span class="text-[10px] text-gray-500 font-medium tracking-wide pl-1 mr-1">SITE</span>
      {#each sites as site}
        <button
          onclick={() => switchSite(site.url)}
          title={site.label}
          class="flex-1 h-7 rounded-xl text-[11px] font-semibold tracking-wide transition-all duration-150
            {activeSiteUrl === site.url
              ? 'bg-gray-700 text-white shadow-sm'
              : 'text-gray-500 hover:text-gray-200 hover:bg-gray-800'}"
        >
          {site.label}
        </button>
      {/each}
      <button
        onclick={() => { showSiteMenu = false }}
        aria-label="Close site menu"
        class="ml-1 w-6 h-6 flex items-center justify-center rounded-lg text-gray-600 hover:text-gray-400 transition-colors"
      >
        <svg class="w-3 h-3" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  {/if}

  <!-- Transcript toast -->
  {#if lastText}
    <div class="px-3 py-1.5 bg-gray-800 border border-gray-600/60 rounded-2xl text-xs text-gray-200 max-w-[340px] truncate shadow-lg">
      {lastText}
    </div>
  {/if}

  <!-- Main bar -->
  <div class="flex items-center gap-1.5 px-2.5 h-11 bg-gray-950/95 backdrop-blur-md rounded-2xl border border-gray-700/60 shadow-xl select-none"
       style="width:340px">

    <!-- Drag handle -->
    <button
      onmousedown={startDrag}
      aria-label="Drag"
      class="flex-none w-6 h-6 flex items-center justify-center text-gray-600 hover:text-gray-400 transition-colors cursor-grab active:cursor-grabbing rounded-lg"
    >
      <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
        <circle cx="9"  cy="6"  r="1.5"/><circle cx="15" cy="6"  r="1.5"/>
        <circle cx="9"  cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/>
        <circle cx="9"  cy="18" r="1.5"/><circle cx="15" cy="18" r="1.5"/>
      </svg>
    </button>

    <!-- Site switcher toggle -->
    <button
      onclick={() => { showSiteMenu = !showSiteMenu }}
      aria-label="Switch site"
      title="Switch website"
      class="flex-none h-7 px-2 rounded-xl flex items-center gap-1 text-[10px] font-bold tracking-wider transition-all duration-150
        {showSiteMenu ? 'bg-gray-700 text-white' : 'bg-gray-800/80 text-gray-400 hover:text-gray-200 hover:bg-gray-700'}"
    >
      <!-- Globe icon -->
      <svg class="w-3 h-3" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10"/>
        <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
      </svg>
      {activeSiteShort()}
    </button>

    <!-- Divider -->
    <div class="w-px h-5 bg-gray-700/60"></div>

    <!-- One-shot mic -->
    <MicButton />

    <!-- Loop button -->
    <button
      onclick={toggleLoop}
      aria-label={loopState === 'listening' ? 'Stop loop' : 'Start loop'}
      title={loopState === 'listening' ? `Recording ${langs[langIdx].label} — click to stop` : 'Continuous transcription'}
      class="relative flex-none w-8 h-8 rounded-xl flex items-center justify-center transition-all duration-200
        {loopState === 'listening'
          ? 'bg-red-500 shadow-[0_0_12px_rgba(239,68,68,0.5)]'
          : loopState === 'error'
            ? 'bg-orange-500/80'
            : 'bg-gray-800 hover:bg-gray-700'}"
    >
      {#if loopState === 'listening'}
        <!-- Pulse ring -->
        <span class="absolute inset-0 rounded-xl bg-red-500 animate-ping opacity-30"></span>
        <!-- Stop icon -->
        <svg class="relative w-3 h-3 text-white" fill="currentColor" viewBox="0 0 24 24">
          <rect x="5" y="5" width="14" height="14" rx="3"/>
        </svg>
      {:else}
        <!-- Loop / infinity icon -->
        <svg class="w-3.5 h-3.5 text-gray-300" fill="none" stroke="currentColor" stroke-width="2.2" viewBox="0 0 24 24">
          <path d="M12 12c-2-2.5-4-4-6-4a4 4 0 0 0 0 8c2 0 4-1.5 6-4z"/>
          <path d="M12 12c2 2.5 4 4 6 4a4 4 0 0 0 0-8c-2 0-4 1.5-6 4z"/>
        </svg>
      {/if}
    </button>

    <!-- Language segmented control -->
    <div class="flex items-center bg-gray-800/80 rounded-xl p-0.5 gap-0.5">
      {#each langs as lang, i}
        <button
          onclick={() => setLang(i)}
          aria-label="Use {lang.label}"
          class="w-8 h-6 rounded-lg text-[10px] font-semibold tracking-wide transition-all duration-150
            {langIdx === i
              ? loopState === 'listening'
                ? 'bg-red-500 text-white shadow-sm'
                : 'bg-gray-600 text-white shadow-sm'
              : 'text-gray-500 hover:text-gray-300'}"
        >
          {lang.label}
        </button>
      {/each}
    </div>

    <div class="flex-1"></div>

    <!-- Settings -->
    <button
      onclick={() => openSettings()}
      aria-label="Settings"
      title="Settings"
      class="flex-none w-7 h-7 flex items-center justify-center rounded-xl text-gray-500 hover:text-gray-300 hover:bg-gray-800 transition-all"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
    </button>

    <!-- Hide / Close -->
    <button
      onclick={() => toggleVisibility()}
      aria-label="Hide overlay"
      title="Hide (use hotkey or tray to show again)"
      class="flex-none w-7 h-7 flex items-center justify-center rounded-xl text-white hover:bg-red-600/80 transition-all"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>

  </div>
</div>
