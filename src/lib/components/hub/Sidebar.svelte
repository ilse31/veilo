<script lang="ts">
  import { icons } from './icons'
  import { settings } from '$lib/stores/settings.svelte'

  interface Props {
    current: string
    onNav: (page: string) => void
    apiOk?: boolean
  }
  let { current, onNav, apiOk = false }: Props = $props()

  const navMain = { id: 'dashboard', label: 'Dasbor', icon: 'dashboard' as const, kbd: '⌘1' }
  const navAssistants = [
    { id: 'gpt',       label: 'ChatGPT',    kbd: '⌘2', src: 'gpt' },
    { id: 'claude',    label: 'Claude',     kbd: '⌘3', src: 'claude' },
    { id: 'translate', label: 'Terjemahan', kbd: '⌘4', src: 'translate' },
    { id: 'chat',      label: 'Obrolan AI', kbd: '⌘5', src: 'native' },
  ]
  const navSettings = { id: 'settings', label: 'Pengaturan', icon: 'settings' as const, kbd: '⌘,' }
</script>

<aside class="hub-sidebar">
  <div class="titlebar-spacer" data-tauri-drag-region></div>

  <div class="hub-brand">
    <div class="hub-brand-logo">N</div>
    <div class="hub-brand-name">no‑screen</div>
    <div class="hub-brand-sub">v0.4</div>
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
