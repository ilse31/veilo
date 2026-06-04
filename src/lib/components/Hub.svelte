<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import TitleBar from './hub/TitleBar.svelte'
  import Sidebar from './hub/Sidebar.svelte'
  import Dashboard from './hub/Dashboard.svelte'
  import NativeChat from './hub/NativeChat.svelte'
  import HubSettings from './hub/HubSettings.svelte'
  import Onboarding from './hub/Onboarding.svelte'
  import GhostTypingBar from './hub/GhostTypingBar.svelte'
  import { getConfig, getProfileName, injectToService } from '$lib/tauri'
  import { settings } from '$lib/stores/settings.svelte'
  import CopilotPanel from './copilot/CopilotPanel.svelte'
  import StartSessionModal from './copilot/StartSessionModal.svelte'
  import { copilotSessionStatus } from '$lib/copilot/api'
  import {
    showService,
    hideService,
    resizeService,
    hideAllServices,
    type Service,
    SERVICES,
  } from './hub/webviewManager'

  type Page = 'onboarding' | 'dashboard' | Service | 'chat' | 'settings' | 'copilot'

  const pageTitles: Record<Page, string> = {
    onboarding: 'Selamat Datang',
    dashboard:  'Dasbor',
    gpt:        'ChatGPT',
    claude:     'Claude',
    translate:  'Google Terjemahan',
    chat:       'Obrolan AI',
    settings:   'Pengaturan',
    copilot:    'Copilot',
  }

  const webviewUrls: Record<Service, string> = {
    gpt:       'https://chat.openai.com',
    claude:    'https://claude.ai',
    translate: 'https://translate.google.com',
  }

  function isWebviewPage(p: Page): p is Service {
    return SERVICES.includes(p as Service)
  }

  let page = $state<Page>('dashboard')
  let pendingPrompt      = $state<string | null>(null)
  let pendingServiceText = $state<string | null>(null)
  let webviewError = $state<string | null>(null)
  let contentAreaEl = $state<HTMLElement | null>(null)
  let ghostActive = $state(false)
  let showStartModal = $state(false)
  let copilotActiveStatus = $state<{ id: number, preset_id: string, started_at: number } | null>(null)

  const platform = navigator.userAgent.includes('Windows') ? 'win' : 'mac'

  // Sync ghost typing state with Rust (global hotkey Ctrl+Alt+G emits this event)
  onMount(async () => {
    const unlisten = await listen<boolean>('ghost-typing-state', ({ payload }) => {
      ghostActive = payload
    })
    return unlisten
  })

  // Load persisted config and profile on startup
  onMount(async () => {
    try {
      const cfg = await getConfig()
      settings.opacity = Math.round(cfg.opacity * 100)
      settings.hotkey = cfg.hotkey
    } catch {}
    try {
      const name = await getProfileName()
      if (name) {
        settings.profileName = name
      } else {
        page = 'onboarding'
      }
    } catch {}
  })

  // Poll copilot session status every 3s
  onMount(() => {
    const tick = async () => {
      copilotActiveStatus = await copilotSessionStatus()
    }
    tick()
    const i = setInterval(tick, 3000)
    return () => clearInterval(i)
  })

  // Apply dark theme reactively
  $effect(() => {
    document.documentElement.setAttribute('data-theme', settings.dark ? 'dark' : 'light')
  })

  // On window resize: reposition any visible service webview
  onMount(() => {
    const observer = new ResizeObserver(() => {
      if (contentAreaEl && isWebviewPage(page)) {
        resizeService(page, contentAreaEl).catch(console.error)
      }
    })
    requestAnimationFrame(() => {
      if (contentAreaEl) observer.observe(contentAreaEl)
    })
    return () => observer.disconnect()
  })

  // Keyboard shortcuts
  onMount(() => {
    const handler = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      const map: Record<string, Page> = {
        '1': 'dashboard', '2': 'gpt', '3': 'claude',
        '4': 'translate', '5': 'chat', ',': 'settings', '6': 'copilot',
      }
      if (map[e.key]) { e.preventDefault(); setPage(map[e.key] as Page) }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  })

  async function setPage(newPage: Page) {
    const prevPage = page
    webviewError = null

    if (isWebviewPage(prevPage) && prevPage !== newPage) {
      hideService(prevPage).catch(console.error)
    } else if (!isWebviewPage(prevPage) && isWebviewPage(newPage)) {
      hideAllServices().catch(console.error)
    }

    page = newPage
    if (!isWebviewPage(newPage)) return

    await tick()
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()))

    if (!contentAreaEl) {
      webviewError = 'Elemen konten tidak ditemukan'
      return
    }

    const rect = contentAreaEl.getBoundingClientRect()
    console.log(`[hub] showService "${newPage}": x=${rect.left} y=${rect.top} w=${rect.width} h=${rect.height}`)

    if (rect.width === 0 || rect.height === 0) {
      webviewError = `Area konten berukuran nol (${rect.width}×${rect.height})`
      return
    }

    try {
      await showService(newPage, contentAreaEl)
      if (pendingServiceText) {
        const txt = pendingServiceText
        pendingServiceText = null
        injectToService(newPage as Service, txt).catch(console.error)
      }
    } catch (err) {
      webviewError = String(err)
      console.error('[hub] showService gagal:', err)
    }
  }

  function handleGhostSend(text: string) {
    ghostActive = false
    if (isWebviewPage(page)) {
      injectToService(page as Service, text).catch(console.error)
    } else if (page === 'chat') {
      pendingPrompt = text
    }
  }

  function handlePrompt(provider: string, text: string) {
    if (provider === 'native') {
      pendingPrompt = text
      setPage('chat')
    } else {
      pendingServiceText = text
      setPage(provider as Page)
    }
  }
</script>

{#if page === 'onboarding'}
  <Onboarding onComplete={(name) => {
    settings.profileName = name
    page = 'dashboard'
  }} />
{:else}
<!-- Opacity is controlled via CSS, sourced from settings store -->
<div class="hub-root" style="opacity:{settings.opacity / 100}">
  <Sidebar current={page} onNav={(p) => setPage(p as Page)} apiOk={!!settings.apiKey} />

  <main class="hub-main">
    <TitleBar
      {platform}
      title={pageTitles[page]}
      stealth={settings.stealth}
      copilotElapsedS={copilotActiveStatus
        ? Math.floor(Date.now()/1000 - copilotActiveStatus.started_at)
        : null}
    />

    {#if isWebviewPage(page)}
      <div class="hub-wv-strip">
        <span class="src-dot {page}"></span>
        <span class="wv-host">{webviewUrls[page]}</span>
        <span class="wv-tag">webview</span>
      </div>
    {/if}

    {#if ghostActive}
      <GhostTypingBar
        onSend={handleGhostSend}
        onCancel={() => { ghostActive = false }}
      />
    {/if}

    <div
      class="hub-content-area"
      class:is-webview={isWebviewPage(page)}
      bind:this={contentAreaEl}
    >
      {#if page === 'dashboard'}
        <Dashboard
          apiUrl={settings.apiUrl}
          apiKey={settings.apiKey}
          onNav={(p) => setPage(p as Page)}
          onPrompt={handlePrompt}
        />

      {:else if page === 'chat'}
        <NativeChat
          apiUrl={settings.apiUrl}
          apiKey={settings.apiKey}
          model={settings.model}
          initialPrompt={pendingPrompt}
          onConsumeInitial={() => pendingPrompt = null}
        />

      {:else if page === 'settings'}
        <HubSettings />

      {:else if page === 'copilot'}
        <CopilotPanel onStartClick={() => showStartModal = true} />

      {:else if isWebviewPage(page)}
        {#if webviewError}
          <div class="hub-wv-error">
            <span class="err-icon">⚠</span>
            <span>{webviewError}</span>
            <button class="hub-btn secondary" onclick={() => setPage(page)}>Coba lagi</button>
          </div>
        {/if}
      {/if}
    </div>

    <StartSessionModal
      open={showStartModal}
      apiUrl={settings.apiUrl}
      apiKey={settings.apiKey}
      model={settings.model}
      onClose={() => showStartModal = false}
      onStarted={(_sid) => { showStartModal = false }}
    />
  </main>
</div>
{/if}

<style>
  .hub-content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
  }
  .hub-content-area.is-webview {
    background: transparent;
  }
  .hub-wv-error {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-soft);
    font-size: 13px;
    padding: 24px;
    text-align: center;
  }
  .err-icon {
    font-size: 24px;
    color: var(--amber);
  }
</style>
