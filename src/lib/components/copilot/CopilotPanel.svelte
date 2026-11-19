<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'
  import ScreenCaptureDialog from './ScreenCaptureDialog.svelte'
  import {
    copilotSessionStatus, copilotStopSession, copilotListSessions, copilotSummarizeSession,
    copilotGetPresets, copilotListCustomPresets,
    type SessionStatus, type CopilotSessionRow, type Preset,
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
  let presets = $state<Preset[]>([])
  let elapsedS = $state(0)
  let summarizingId = $state<number | null>(null)
  let summarizeError = $state<Record<number, string>>({})

  async function refresh() {
    status = await copilotSessionStatus()
    history = await copilotListSessions()
  }

  onMount(() => {
    refresh()
    copilotGetPresets().then(p => { if (p) presets = p }).catch(() => {})
    copilotListCustomPresets().then(custom => {
      if (custom) presets = [...presets, ...custom]
    }).catch(() => {})

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
  function presetName(id: string) {
    return presets.find(p => p.id === id)?.name ?? id
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
    <h2 class="hub-greeting" style="font-size:19px">Copilot</h2>
    <p class="hub-greeting-sub">Asisten jawaban live saat meeting — jawaban muncul di kartu overlay.</p>

    <!-- Session status card -->
    <div class="status-card {status ? 'live-card' : ''}">
      {#if status}
        <div class="live">
          <span class="live-pill"><span class="dot"></span>LIVE</span>
          <span class="live-info">
            Sesi aktif — preset <strong>{presetName(status.preset_id)}</strong>
            <span class="elapsed">{fmtElapsed(elapsedS)}</span>
          </span>
        </div>
        <button class="hub-btn stop" onclick={stop}>Hentikan Sesi</button>
      {:else}
        <div class="idle">
          <span class="dot off"></span>
          <span>Tidak ada sesi aktif</span>
        </div>
        <button class="hub-btn accent" onclick={onStartClick}>Mulai Sesi</button>
      {/if}
    </div>

    <!-- Preset picker -->
    {#if presets.length > 0}
      <h3 class="section-h">Preset</h3>
      <div class="preset-row">
        {#each presets as p (p.id)}
          <button
            class="preset-chip {status?.preset_id === p.id ? 'active' : ''}"
            onclick={onStartClick}
            title={p.response_format}
          >
            {p.name}
          </button>
        {/each}
      </div>
    {/if}

    <div class="screen-action">
      <ScreenCaptureDialog />
      <span>Pilih monitor, periksa gambar, lalu kirim ke model AI.</span>
    </div>

    {#if history.length > 0}
      <h3 class="section-h">Riwayat sesi</h3>
      <div class="hub-card history">
        {#each history as h (h.id)}
          <div class="hrow">
            <div class="row">
              <span class="t">{fmtTime(h.started_at)}</span>
              <span class="preset-tag">{presetName(h.preset_id)}</span>
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
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .status-card {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 16px; border: 1px solid var(--border); border-radius: 10px;
    background: var(--bg-elev);
    margin-bottom: 20px;
  }
  .status-card.live-card { border-color: color-mix(in srgb, var(--green) 30%, transparent); }
  .live, .idle { display: flex; align-items: center; gap: 12px; font-size: 13px; }
  .live-pill {
    display: inline-flex; align-items: center; gap: 6px;
    font-size: 10.5px; font-weight: 700; letter-spacing: .06em;
    color: var(--green);
    padding: 3px 9px; border-radius: 999px;
    background: color-mix(in srgb, var(--green) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--green) 35%, transparent);
  }
  .live-pill .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--green); animation: blink 1.4s infinite; }
  .live-info { color: var(--text-mid); }
  .live-info strong { color: var(--text); font-weight: 500; }
  .elapsed { font-family: var(--mono); font-size: 12px; color: var(--text-soft); margin-left: 8px; }
  .idle { color: var(--text-soft); }
  .dot.off { width: 8px; height: 8px; border-radius: 50%; background: var(--text-faint); }
  @keyframes blink { 50% { opacity: 0.4; } }

  .hub-btn.stop {
    background: color-mix(in srgb, var(--red) 14%, transparent);
    color: var(--red);
    border: 1px solid color-mix(in srgb, var(--red) 35%, transparent);
  }
  .hub-btn.stop:hover { background: color-mix(in srgb, var(--red) 20%, transparent); opacity: 1; }

  .section-h { font-size: 13px; font-weight: 600; color: var(--text); margin: 20px 0 10px; }

  .preset-row { display: flex; gap: 8px; flex-wrap: wrap; }
  .preset-chip {
    font-family: inherit; font-size: 12.5px; font-weight: 500;
    padding: 7px 14px; border-radius: 999px;
    background: var(--bg-elev); color: var(--text-mid);
    border: 1px solid var(--border); cursor: default;
    transition: all .1s;
  }
  .preset-chip:hover { border-color: var(--border-strong); color: var(--text); }
  .preset-chip.active {
    background: var(--accent-soft); color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  .screen-action { display: flex; align-items: center; gap: 10px; margin: 20px 0 0; font-size: 12px; color: var(--text-soft); }

  .history { padding: 6px 14px; }
  .hrow { padding: 10px 0; border-bottom: 1px solid var(--border); font-size: 12.5px; }
  .hrow:last-child { border-bottom: 0; }
  .hrow .row { display: flex; align-items: center; gap: 12px; }
  .hrow .t { color: var(--text-soft); width: 52px; flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .preset-tag {
    font-size: 11px; font-weight: 500; padding: 2px 8px; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent); flex-shrink: 0;
  }
  .hrow .c { color: var(--text-soft); flex: 1; }
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
    margin: 0 0 8px; white-space: pre-wrap; overflow-wrap: anywhere;
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
