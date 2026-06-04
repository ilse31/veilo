<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { setClickThrough } from '$lib/tauri'

  interface Props {
    platform?: 'mac' | 'win'
    title?: string
    stealth?: boolean
    copilotElapsedS?: number | null
  }
  let { platform = 'mac', title = '', stealth = false, copilotElapsedS = null }: Props = $props()

  const win = getCurrentWindow()

  let maximized    = $state(false)
  let clickThrough = $state(false)

  // Keep maximize icon in sync
  win.onResized(async () => {
    maximized = await win.isMaximized()
  })

  // Reset click-through indicator when window becomes visible again
  win.onFocusChanged(({ payload: focused }) => {
    if (focused) clickThrough = false
  })

  // For a skip-taskbar overlay, minimize = hide so the user can always
  // restore via the tray icon or global hotkey.
  async function minimize()       { await win.hide() }
  async function toggleMaximize() { await win.toggleMaximize(); maximized = await win.isMaximized() }
  async function close()          { await win.close() }

  async function toggleClickThrough() {
    clickThrough = !clickThrough
    await setClickThrough(clickThrough)
  }
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

  <!-- Click-through toggle: overlay floats above browser without stealing focus -->
  <button
    class="ct-btn {clickThrough ? 'ct-on' : ''}"
    onclick={toggleClickThrough}
    aria-label={clickThrough ? 'Nonaktifkan click-through' : 'Aktifkan click-through'}
    title={clickThrough
      ? 'Click-through ON — klik menembus ke browser di bawah. Klik untuk nonaktifkan.'
      : 'Click-through OFF — aktifkan agar browser tidak kehilangan fokus'}
  >
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none"
         stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <!-- cursor arrow -->
      <path d="M5 3l14 9-7 1-4 7-3-17z"/>
      {#if clickThrough}
        <!-- slash through cursor when active -->
        <line x1="3" y1="21" x2="21" y2="3"/>
      {/if}
    </svg>
    {clickThrough ? 'Tembus ✓' : 'Tembus'}
  </button>

  <div class="title-center" data-tauri-drag-region>
    {title}
    {#if copilotElapsedS != null}
      <span class="copilot-badge" title="Sesi copilot aktif">
        ● Live {Math.floor(copilotElapsedS/60).toString().padStart(2,'0')}:{(copilotElapsedS%60).toString().padStart(2,'0')}
      </span>
    {/if}
  </div>

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
  .ct-btn {
    display: flex; align-items: center; gap: 5px;
    padding: 0 8px; height: 22px; border-radius: 6px;
    font-size: 11px; font-weight: 500; letter-spacing: 0.01em;
    border: 1px solid var(--border);
    background: transparent; color: var(--text-soft);
    cursor: pointer; transition: all 0.15s;
    pointer-events: auto; -webkit-app-region: no-drag;
    flex-shrink: 0;
  }
  .ct-btn:hover { background: var(--bg-hover); color: var(--text); }
  .ct-btn.ct-on {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    color: var(--accent);
  }

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
  .copilot-badge {
    display: inline-flex; align-items: center; gap: 4px;
    margin-left: 8px; padding: 2px 8px;
    background: rgba(94,106,210,0.15); color: #5e6ad2;
    border-radius: 10px; font-size: 10.5px; font-weight: 500;
  }
</style>
