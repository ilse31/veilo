<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'
  import ScreenCaptureDialog from './ScreenCaptureDialog.svelte'
  import {
    copilotSessionStatus, copilotStopSession, copilotListSessions, copilotSummarizeSession,
    type SessionStatus, type CopilotSessionRow,
  } from '$lib/copilot/api'

  marked.setOptions({ breaks: true, gfm: true })

  function renderMd(text: string): string {
    return DOMPurify.sanitize(marked.parse(text) as string)
  }

  interface Props {
    onStartClick: () => void
    apiUrl: string
    apiKey: string
    model: string
  }
  let { onStartClick, apiUrl, apiKey, model }: Props = $props()

  let status = $state<SessionStatus | null>(null)
  let history = $state<CopilotSessionRow[]>([])
  let elapsedS = $state(0)
  let summarizingId = $state<number | null>(null)
  let summarizeError = $state<Record<number, string>>({})

  async function refresh() {
    status = await copilotSessionStatus()
    history = await copilotListSessions()
  }

  onMount(() => {
    refresh()
    const interval = setInterval(() => {
      if (status) elapsedS = Math.floor(Date.now() / 1000 - status.started_at)
    }, 1000)

    let unsubs: Array<() => void> = []
    const setup = async () => {
      unsubs.push(await listen('copilot-session-started', refresh))
      unsubs.push(await listen('copilot-session-ended', refresh))
    }
    setup()

    return () => {
      clearInterval(interval)
      unsubs.forEach(u => u())
    }
  })

  async function stop() {
    await copilotStopSession()
    elapsedS = 0
    await refresh()
  }

  function fmtElapsed(s: number) {
    const m = Math.floor(s / 60), r = s % 60
    return `${m.toString().padStart(2,'0')}:${r.toString().padStart(2,'0')}`
  }
  function fmtTime(ts: number) {
    return new Date(ts * 1000).toLocaleTimeString('id-ID', { hour:'2-digit', minute:'2-digit' })
  }

  async function summarize(sessionId: number) {
    summarizingId = sessionId
    summarizeError = { ...summarizeError, [sessionId]: '' }
    try {
      await copilotSummarizeSession({ sessionId, apiUrl, apiKey, model })
      await refresh()
    } catch (e) {
      summarizeError = { ...summarizeError, [sessionId]: String(e) }
    } finally {
      summarizingId = null
    }
  }
</script>

<div class="hub-page-scroll">
  <div class="hub-page-pad">
    <h2 class="hub-greeting" style="font-size:19px;margin-bottom:24px">Copilot</h2>

    <div class="status-card">
      {#if status}
        <div class="live">
          <span class="dot"></span>
          <span>Sesi aktif · {fmtElapsed(elapsedS)} · preset <code>{status.preset_id}</code></span>
        </div>
        <button class="hub-btn" onclick={stop}>Hentikan Sesi</button>
      {:else}
        <div class="idle">
          <span class="dot off"></span>
          <span>Tidak ada sesi aktif</span>
        </div>
        <button class="hub-btn accent" onclick={onStartClick}>Mulai Sesi</button>
      {/if}
    </div>

    <div class="screen-action">
      <ScreenCaptureDialog />
      <span>Pilih monitor, periksa gambar, lalu kirim ke model AI.</span>
    </div>

    {#if history.length > 0}
      <h3 class="section-h">Riwayat sesi</h3>
      <ul class="history">
        {#each history as h}
          <li>
            <div class="row">
              <span class="t">{fmtTime(h.started_at)}</span>
              <span class="p">{h.preset_id}</span>
              <span class="c">{h.suggestion_count} saran</span>
              {#if !h.summary}
                <button
                  class="hub-btn secondary sm"
                  onclick={() => summarize(h.id)}
                  disabled={summarizingId === h.id}
                >
                  {summarizingId === h.id ? 'Meringkas…' : 'Ringkas'}
                </button>
              {/if}
            </div>
            {#if h.summary}
              <div class="summary md">{@html renderMd(h.summary)}</div>
            {:else if summarizeError[h.id]}
              <p class="summary-error">{summarizeError[h.id]}</p>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .screen-action { display: flex; align-items: center; gap: 10px; margin: -12px 0 24px; font-size: 12px; color: var(--text-soft); }
  .status-card {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px; border: 1px solid var(--border); border-radius: 10px;
    margin-bottom: 24px;
  }
  .live, .idle { display: flex; align-items: center; gap: 10px; font-size: 13px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #19c37d; animation: blink 1.4s infinite; }
  .dot.off { background: #666; animation: none; }
  @keyframes blink { 50% { opacity: 0.4; } }
  .section-h { font-size: 13px; color: var(--text-soft); margin: 24px 0 10px; }
  .history { list-style: none; padding: 0; margin: 0; }
  .history li { padding: 10px 0; border-bottom: 1px solid var(--border); font-size: 12.5px; }
  .history .row { display: flex; align-items: center; gap: 12px; }
  .history .t { color: var(--text-soft); width: 60px; flex-shrink: 0; }
  .history .p { flex: 1; }
  .history .c { color: var(--text-soft); }
  .hub-btn.sm { padding: 4px 10px; font-size: 11.5px; }
  .summary {
    margin: 8px 0 0; padding: 10px 12px;
    background: var(--bg-side); border-radius: 8px;
    font-size: 12.5px; line-height: 1.55; color: var(--text);
    user-select: text;
  }
  .md > :global(:first-child) { margin-top: 0; }
  .md > :global(:last-child)  { margin-bottom: 0; }
  .md :global(h1), .md :global(h2), .md :global(h3), .md :global(h4) {
    margin: 14px 0 6px; font-weight: 600; line-height: 1.3;
  }
  .md :global(h1) { font-size: 15px; }
  .md :global(h2) { font-size: 14px; }
  .md :global(h3) { font-size: 13px; color: var(--text-soft); }
  .md :global(p)  { margin: 0 0 8px; }
  .md :global(ul), .md :global(ol) { margin: 0 0 8px; padding-left: 20px; }
  .md :global(li) { margin-bottom: 4px; }
  .md :global(strong) { font-weight: 600; }
  .md :global(hr) { border: none; border-top: 1px solid var(--border); margin: 12px 0; }
  .md :global(code) {
    background: rgba(127,127,127,0.18); padding: 1px 5px; border-radius: 4px;
    font-family: ui-monospace, "SF Mono", Consolas, monospace; font-size: 11.5px;
  }
  .md :global(pre) {
    background: rgba(127,127,127,0.12); padding: 8px 10px; border-radius: 6px;
    overflow-x: auto; margin: 0 0 8px;
  }
  .md :global(pre code) { background: none; padding: 0; }
  .md :global(blockquote) {
    margin: 0 0 8px; padding-left: 10px; border-left: 3px solid var(--border);
    color: var(--text-soft);
  }
  .md :global(table) { border-collapse: collapse; margin: 0 0 8px; font-size: 12px; }
  .md :global(th), .md :global(td) { border: 1px solid var(--border); padding: 4px 8px; }
  .summary-error { margin: 6px 0 0; font-size: 11.5px; color: var(--red, #e74c3c); }
</style>
