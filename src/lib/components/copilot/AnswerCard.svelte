<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { settings } from '$lib/stores/settings.svelte'
  import { copilotSetCustomInstruction } from '$lib/copilot/api'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'

  marked.setOptions({ breaks: true, gfm: true })

  function renderMd(text: string): string {
    return DOMPurify.sanitize(marked.parse(text) as string)
  }

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
  let instText = $state('')

  const current = $derived(history[activeIdx] ?? null)

  async function submitInstruction() {
    try {
      await copilotSetCustomInstruction(instText.trim())
      regenerate()
    } catch (e) {
      console.error(e)
    }
  }

  function handleInstKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      submitInstruction()
    }
  }

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
        if (s) {
          s.done = true
          // Schedule auto-dismiss if not pinned and timeout > 0
          const dismissAfterMs = settings.copilotAutoDismissS * 1000
          if (dismissAfterMs > 0) {
            setTimeout(() => {
              // Only dismiss if: still showing this suggestion as current, not pinned, no newer arrived
              if (!pinned && history[0]?.id === payload.id) {
                getCurrentWindow().hide()
              }
            }, dismissAfterMs)
          }
        }
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
  async function dismiss() {
    try {
      await invoke('copilot_stop_session')
    } catch (e) {
      console.error('Failed to stop copilot session on close:', e)
    }
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
    {:else if current.format === 'Code'}
      <pre class="code">{current.text}</pre>
    {:else}
      <div class="md">{@html renderMd(current.text)}</div>
    {/if}
  </div>

  <!-- Real-time instructions -->
  <div class="inst-box">
    <input 
      type="text" 
      bind:value={instText} 
      onkeydown={handleInstKeydown}
      placeholder="Ketik instruksi khusus (misal: 'buat code saja')..." 
      class="inst-input"
    />
    <button onclick={submitInstruction} class="inst-send" title="Kirim instruksi">➔</button>
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
  .code { background: rgba(255,255,255,0.04); padding: 8px; border-radius: 6px;
          font-family: "Consolas", monospace; font-size: 12px;
          white-space: pre-wrap; overflow-x: auto; }
  .md :global(p)          { margin: 0 0 8px; }
  .md :global(p:last-child){ margin-bottom: 0; }
  .md :global(ul), .md :global(ol) { margin: 0 0 8px; padding-left: 18px; }
  .md :global(li)         { margin-bottom: 3px; }
  .md :global(strong)     { color: #fff; font-weight: 600; }
  .md :global(em)         { color: #c8c8d4; }
  .md :global(code)       { background: rgba(255,255,255,0.08); padding: 1px 5px;
                             border-radius: 3px; font-family: "Consolas", monospace;
                             font-size: 11.5px; color: #a8daff; }
  .md :global(pre)        { background: rgba(255,255,255,0.04); padding: 8px;
                             border-radius: 6px; overflow-x: auto; margin: 0 0 8px; }
  .md :global(pre code)   { background: none; padding: 0; color: #e5e5ea;
                             font-size: 12px; }
  .md :global(h1), .md :global(h2), .md :global(h3) {
                             margin: 0 0 6px; font-weight: 600; color: #fff; }
  .md :global(h1)         { font-size: 16px; }
  .md :global(h2)         { font-size: 14px; }
  .md :global(h3)         { font-size: 13px; }
  .md :global(blockquote) { border-left: 3px solid rgba(94,106,210,0.6);
                             margin: 0 0 8px; padding-left: 10px; color: #b8b8c0; }
  .footer { display: flex; justify-content: space-between; align-items: center;
            padding: 6px 12px; border-top: 1px solid rgba(255,255,255,0.06);
            font-size: 11px; }
  .nav button, .actions button {
    background: none; border: none; color: #b8b8c0; cursor: pointer;
    padding: 4px 6px; border-radius: 4px;
  }
  .nav button:hover, .actions button:hover { background: rgba(255,255,255,0.06); }
  .actions button.active { color: #5e6ad2; }

  /* Real-time instructions style */
  .inst-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid rgba(255,255,255,0.06);
    background: rgba(0,0,0,0.15);
  }
  .inst-input {
    flex: 1;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 6px;
    padding: 6px 10px;
    color: #e5e5ea;
    font-size: 11.5px;
    outline: none;
    transition: all 0.15s ease;
    box-sizing: border-box;
  }
  .inst-input:focus {
    border-color: rgba(94,106,210,0.5);
    background: rgba(255,255,255,0.07);
  }
  .inst-send {
    background: none;
    border: none;
    color: #b8b8c0;
    font-size: 12px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }
  .inst-send:hover {
    color: #e5e5ea;
    background: rgba(255,255,255,0.06);
  }
</style>
