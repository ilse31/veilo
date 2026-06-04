<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  type Format = 'Bullets' | 'Headline' | 'Code'
  interface Suggestion {
    id: string
    text: string
    format: Format
    done: boolean
    timestamp: number
  }

  let history = $state<Suggestion[]>([])
  let activeIdx = $state(0)
  let pinned = $state(false)
  let pulse = $state(false)

  const current = $derived(history[activeIdx] ?? null)

  onMount(() => {
    let unsubs: Array<() => void> = []

    const setup = async () => {
      const offDelta = await listen<{id:string, delta:string, format:Format}>(
        'copilot-suggest-delta', ({payload}) => {
          const existing = history.find(h => h.id === payload.id)
          if (existing) {
            existing.text += payload.delta
          } else {
            history = [{
              id: payload.id, text: payload.delta,
              format: payload.format, done: false, timestamp: Date.now(),
            }, ...history].slice(0, 10)
            activeIdx = 0
            pulse = true
            setTimeout(() => pulse = false, 600)
          }
        }
      )
      const offDone = await listen<{id:string}>('copilot-suggest-done', ({payload}) => {
        const s = history.find(h => h.id === payload.id)
        if (s) s.done = true
      })
      const offEnd = await listen('copilot-session-ended', async () => {
        await getCurrentWindow().hide()
        history = []
      })
      unsubs = [offDelta, offDone, offEnd]
    }
    setup()
    return () => unsubs.forEach(u => u())
  })

  function copy() {
    if (current) navigator.clipboard.writeText(current.text)
  }
  function dismiss() {
    getCurrentWindow().hide()
  }
  function regenerate() {
    invoke('copilot_force_regenerate')
  }
  function prev() { if (activeIdx < history.length - 1) activeIdx++ }
  function next() { if (activeIdx > 0) activeIdx-- }
</script>

<div class="card" class:pulse>
  <div class="header" data-tauri-drag-region>
    <span class="status-dot" class:done={current?.done} class:loading={current && !current.done}></span>
    <span class="title">Copilot</span>
    <button class="x" onclick={dismiss} aria-label="Tutup">✕</button>
  </div>

  <div class="body">
    {#if !current}
      <div class="empty">Menunggu suara…</div>
    {:else if current.format === 'Headline'}
      <h2 class="headline">{current.text.split('\n')[0]}</h2>
      <p class="detail">{current.text.split('\n').slice(1).join('\n')}</p>
    {:else if current.format === 'Code'}
      <pre class="code">{current.text}</pre>
    {:else}
      <div class="bullets">{current.text}</div>
    {/if}
  </div>

  <div class="footer">
    {#if history.length > 1}
      <div class="nav">
        <button onclick={prev} disabled={activeIdx >= history.length - 1}>◀</button>
        <span>{activeIdx + 1}/{history.length}</span>
        <button onclick={next} disabled={activeIdx <= 0}>▶</button>
      </div>
    {/if}
    <div class="actions">
      <button onclick={regenerate} title="Regenerate">↻</button>
      <button onclick={copy} title="Copy">📋</button>
      <button class:active={pinned} onclick={() => pinned = !pinned} title="Pin">📌</button>
    </div>
  </div>
</div>

<style>
  .card {
    display: flex; flex-direction: column;
    height: 100vh;
    background: rgba(20, 20, 28, 0.92);
    color: #e5e5ea;
    backdrop-filter: blur(20px);
    border-radius: 12px;
    border: 1px solid rgba(255,255,255,0.08);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    overflow: hidden;
  }
  .card.pulse { animation: pulse 0.6s ease-out; }
  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 rgba(94,106,210,0.5); }
    100% { box-shadow: 0 0 0 12px rgba(94,106,210,0); }
  }
  .header {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 12px; border-bottom: 1px solid rgba(255,255,255,0.06);
    font-size: 12px; user-select: none;
  }
  .status-dot {
    width: 8px; height: 8px; border-radius: 50%; background: #555;
  }
  .status-dot.loading { background: #5e6ad2; animation: blink 1s infinite; }
  .status-dot.done    { background: #19c37d; }
  @keyframes blink { 50% { opacity: 0.4; } }
  .title { flex: 1; font-weight: 600; }
  .x { background: none; border: none; color: #888; cursor: pointer; font-size: 14px; }
  .body { flex: 1; padding: 12px 14px; overflow-y: auto; font-size: 13px; line-height: 1.5; }
  .empty { color: #666; font-style: italic; text-align: center; padding-top: 40px; }
  .headline { font-size: 18px; margin: 0 0 8px; }
  .detail { color: #b8b8c0; font-size: 12px; margin: 0; }
  .code { background: rgba(255,255,255,0.04); padding: 8px; border-radius: 6px;
          font-family: "Consolas", monospace; font-size: 12px;
          white-space: pre-wrap; overflow-x: auto; }
  .bullets { white-space: pre-wrap; }
  .footer { display: flex; justify-content: space-between; align-items: center;
            padding: 6px 12px; border-top: 1px solid rgba(255,255,255,0.06);
            font-size: 11px; }
  .nav button, .actions button {
    background: none; border: none; color: #b8b8c0; cursor: pointer;
    padding: 4px 6px; border-radius: 4px;
  }
  .nav button:hover, .actions button:hover { background: rgba(255,255,255,0.06); }
  .actions button.active { color: #5e6ad2; }
</style>
