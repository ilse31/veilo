<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { icons } from './icons'
  import { settings } from '$lib/stores/settings.svelte'

  interface Props {
    current: string
    onNav: (page: string) => void
    apiOk?: boolean
  }
  let { current, onNav, apiOk = false }: Props = $props()

  let appVersion = $state('')
  $effect(() => {
    getVersion().then(v => appVersion = v)
  })

  const navMain = { id: 'dashboard', label: 'Dasbor', icon: 'dashboard' as const, kbd: '⌘1' }
  const navCopilot = { id: 'copilot', label: 'Copilot', icon: 'dashboard' as const, kbd: '⌘6' }
  const navAssistants = [
    { id: 'gpt',       label: 'ChatGPT',    kbd: '⌘2', src: 'gpt' },
    { id: 'claude',    label: 'Claude',     kbd: '⌘3', src: 'claude' },
    { id: 'translate', label: 'Terjemahan', kbd: '⌘4', src: 'translate' },
    { id: 'chat',      label: 'Obrolan AI', kbd: '⌘5', src: 'native' },
  ]
  const navMarkdown = { id: 'markdown', label: 'Baca Markdown', icon: 'fileText' as const, kbd: '⌘7' }
  const navSettings = { id: 'settings', label: 'Pengaturan', icon: 'settings' as const, kbd: '⌘,' }
</script>

<aside class="hub-sidebar">
  <div class="titlebar-spacer" data-tauri-drag-region></div>

  <div class="hub-brand">
    <div class="hub-brand-logo">N</div>
    <div class="hub-brand-name">no‑screen</div>
    <div class="hub-brand-sub">{appVersion ? `v${appVersion}` : '...'}</div>
  </div>

  <!-- Navigasi -->
  <div class="hub-section-label">Navigasi</div>
  <div class="hub-nav-list">
    <div
      class="hub-nav-item {current === navMain.id ? 'active' : ''}"
      onclick={() => onNav(navMain.id)}
      role="button" tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onNav(navMain.id)}
    >
      <span class="ic">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          {@html icons[navMain.icon]}
        </svg>
      </span>
      <span>{navMain.label}</span>
      <span class="kbd">{navMain.kbd}</span>
    </div>
  </div>

  <!-- Copilot -->
  <div class="hub-section-label">Copilot (Beta)</div>
  <div class="hub-nav-list">
    <div
      class="hub-nav-item {current === navCopilot.id ? 'active' : ''}"
      onclick={() => onNav(navCopilot.id)}
      role="button" tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onNav(navCopilot.id)}
    >
      <span class="ic">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M12 1v6m0 10v6m11-11h-6M7 12H1" />
        </svg>
      </span>
      <span>{navCopilot.label}</span>
      <span class="kbd">{navCopilot.kbd}</span>
    </div>
  </div>

  <!-- Alat -->
  <div class="hub-section-label">Alat</div>
  <div class="hub-nav-list">
    <div
      class="hub-nav-item {current === navMarkdown.id ? 'active' : ''}"
      onclick={() => onNav(navMarkdown.id)}
      role="button" tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onNav(navMarkdown.id)}
    >
      <span class="ic">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          {@html icons[navMarkdown.icon]}
        </svg>
      </span>
      <span>{navMarkdown.label}</span>
      <span class="kbd">{navMarkdown.kbd}</span>
    </div>
  </div>

  <!-- Asisten -->
  <div class="hub-section-label">Asisten</div>
  <div class="hub-nav-list">
    {#each navAssistants as item}
      <div
        class="hub-nav-item {current === item.id ? 'active' : ''}"
        onclick={() => onNav(item.id)}
        role="button" tabindex="0"
        onkeydown={(e) => e.key === 'Enter' && onNav(item.id)}
      >
        <span class="src-dot {item.src}"></span>
        <span>{item.label}</span>
        <span class="kbd">{item.kbd}</span>
      </div>
    {/each}
  </div>

  <div class="spacer"></div>

  <!-- Sistem -->
  <div class="hub-section-label">Sistem</div>
  <div class="hub-nav-list">
    <div
      class="hub-nav-item {current === navSettings.id ? 'active' : ''}"
      onclick={() => onNav(navSettings.id)}
      role="button" tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onNav(navSettings.id)}
    >
      <span class="ic">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          {@html icons[navSettings.icon]}
        </svg>
      </span>
      <span>{navSettings.label}</span>
      <span class="kbd">{navSettings.kbd}</span>
    </div>
  </div>

  <!-- User card -->
  <div class="hub-user-card">
    <div class="av">{(settings.profileName ?? 'A')[0].toUpperCase()}</div>
    <div class="info">
      <div class="name">{settings.profileName ?? 'Akun Lokal'}</div>
      <div class="plan">{apiOk ? 'API tersambung' : 'API belum diatur'}</div>
    </div>
  </div>
</aside>

<style>
  .titlebar-spacer {
    height: 36px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: var(--bg-side);
    -webkit-app-region: drag;
  }
  .spacer { flex: 1; }
</style>
