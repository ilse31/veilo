<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'

  interface Props {
    platform?: 'mac' | 'win'
    title?: string
    stealth?: boolean
  }
  let { platform = 'mac', title = '', stealth = false }: Props = $props()

  const win = getCurrentWindow()

  let maximized = $state(false)

  // Keep maximize icon in sync
  win.onResized(async () => {
    maximized = await win.isMaximized()
  })

  // For a skip-taskbar overlay, minimize = hide so the user can always
  // restore via the tray icon or global hotkey.
  async function minimize()       { await win.hide() }
  async function toggleMaximize() { await win.toggleMaximize(); maximized = await win.isMaximized() }
  async function close()          { await win.close() }
</script>

<div class="hub-titlebar" data-tauri-drag-region>
  {#if platform === 'mac'}
    <!-- macOS: traffic lights (close / minimise / maximise) -->
    <div class="lights">
      <button class="hub-tl r" aria-label="Close"    onclick={close}></button>
      <button class="hub-tl y" aria-label="Minimize"  onclick={minimize}></button>
      <button class="hub-tl g" aria-label="Maximize"  onclick={toggleMaximize}></button>
    </div>
  {:else}
    <div style="width:1px"></div>
  {/if}

  {#if stealth}
    <div class="stealth-badge" title="Tersembunyi dari screenshot & share screen">
      <span class="pulse"></span>
      Mode Senyap
    </div>
  {/if}

  <div class="title-center" data-tauri-drag-region>{title}</div>

  {#if platform === 'win'}
    <!-- Windows: controls on the right -->
    <div class="win-ctrls">
      <button class="wc" aria-label="Minimize" onclick={minimize}>
        <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
      </button>
      <button class="wc" aria-label={maximized ? 'Restore' : 'Maximize'} onclick={toggleMaximize}>
        {#if maximized}
          <!-- Restore icon (two overlapping squares) -->
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
            <rect x="2" y="0" width="8" height="8"/>
            <path d="M0 2v8h8"/>
          </svg>
        {:else}
          <!-- Maximize icon (single square) -->
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
            <rect x="0.5" y="0.5" width="9" height="9"/>
          </svg>
        {/if}
      </button>
      <button class="wc x" aria-label="Close" onclick={close}>
        <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
          <line x1="1" y1="1" x2="9" y2="9"/>
          <line x1="9" y1="1" x2="1" y2="9"/>
        </svg>
      </button>
    </div>
  {:else}
    <div style="width:60px" data-tauri-drag-region></div>
  {/if}
</div>

<style>
  .lights {
    display: flex; gap: 8px; align-items: center;
    pointer-events: auto; -webkit-app-region: no-drag;
  }
  .hub-tl {
    cursor: default;
    border: none;
    padding: 0;
  }
  .title-center {
    flex: 1; text-align: center;
    font-size: 12px; font-weight: 500; color: var(--text-soft);
    letter-spacing: 0.01em;
    -webkit-app-region: drag;
  }
</style>
