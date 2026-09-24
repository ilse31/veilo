<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import {
    copilotSessionStatus, copilotStopSession, copilotListSessions, copilotSummarizeSession,
    type SessionStatus, type CopilotSessionRow,
  } from '$lib/copilot/api'

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
              <p class="summary">{h.summary}</p>
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
    font-size: 12.5px; line-height: 1.5; color: var(--text);
    white-space: pre-wrap;
  }
  .summary-error { margin: 6px 0 0; font-size: 11.5px; color: var(--red, #e74c3c); }
</style>
