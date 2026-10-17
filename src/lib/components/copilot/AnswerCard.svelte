<script lang="ts">
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { settings } from '$lib/stores/settings.svelte'
  import { copilotSetCustomInstruction, copilotSetForceIndonesian } from '$lib/copilot/api'
  import { getConfig } from '$lib/tauri'
  import { askCopilot } from '$lib/screen'
  import ScreenCaptureDialog from './ScreenCaptureDialog.svelte'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'

  marked.setOptions({ breaks: true, gfm: true })

  function renderMd(text: string): string {
    return DOMPurify.sanitize(marked.parse(text) as string)
  }

  type Format = 'Bullets' | 'Headline' | 'Code'
  interface Suggestion {
    id: string
    question: string
    text: string
    format: Format
    done: boolean
    timestamp: number
    thumb?: string
  }

  // Chat thread: one question → one answer, oldest first.
  const MAX_THREAD = 50
  let history = $state<Suggestion[]>([])
  let chatEl = $state<HTMLDivElement | null>(null)
  let pinned = $state(false)
  let pulse = $state(false)
  let monitorToast = $state('')
  let toastTimer: ReturnType<typeof setTimeout> | undefined
  let instText = $state('')
  let questionText = $state('')
  let questionError = $state('')
  let questionBusy = $state(false)
  let editingQuestion = $state(false)
  let instructionStatus = $state('')
  let instructionError = $state('')
  let editingInstruction = $state(false)

  const last = $derived(history[history.length - 1] ?? null)

  function scrollChat() {
    queueMicrotask(() => { if (chatEl) chatEl.scrollTop = chatEl.scrollHeight })
  }

  // ── Live transcript ("what the STT heard") ────────────────────────────────
  const TRANSCRIPT_KEEP = 5
  type Source = 'main' | 'remote' | 'self'
  let finals = $state<{ id: number; text: string; source: Source; start_ms: number }[]>([])
  // Indonesian translations (when "always Indonesian" is on), keyed by original text.
  let translations = $state<Record<string, string>>({})
  let forceId = $state(false)
  function tr(original: string): string | null {
    const t = translations[original.trim()]
    return t && t.trim() !== original.trim() ? t : null
  }
  async function toggleForceId() {
    const next = !forceId
    forceId = next
    try { await copilotSetForceIndonesian(next) } catch (e) { forceId = !next; console.error(e) }
  }
  let partials = $state<Partial<Record<Source, string>>>({})
  const partialLines = $derived(
    (Object.entries(partials) as [Source, string][]).filter(([, t]) => t),
  )
  const SOURCE_LABEL: Record<Source, string> = { main: '', remote: 'Lawan', self: 'Saya' }
  let finalSeq = 0
  let showTranscript = $state(readShowTranscript())
  let transcriptEl = $state<HTMLDivElement | null>(null)

  function readShowTranscript(): boolean {
    try { return localStorage.getItem('veilo.showTranscript') !== '0' } catch { return true }
  }
  function toggleTranscript() {
    showTranscript = !showTranscript
    try { localStorage.setItem('veilo.showTranscript', showTranscript ? '1' : '0') } catch {}
  }
  function scrollTranscript() {
    queueMicrotask(() => { if (transcriptEl) transcriptEl.scrollTop = transcriptEl.scrollHeight })
  }

  async function submitInstruction() {
    instructionStatus = ''
    instructionError = ''
    try {
      await copilotSetCustomInstruction(instText.trim())
      instructionStatus = 'Instruksi diterapkan. Untuk membaca layar dengan instruksi ini, tangkap layar lagi.'
    } catch (e) {
      instructionError = `Gagal menerapkan instruksi: ${String(e)}`
      console.error(e)
    }
  }

  async function submitQuestion() {
    const question = questionText.trim()
    if (!question || questionBusy) return
    questionBusy = true
    questionError = ''
    try {
      await askCopilot(question)
      questionText = ''
    } catch (e) {
      questionError = `Gagal mengirim pertanyaan: ${String(e)}`
    } finally {
      questionBusy = false
    }
  }

  function handleQuestionKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      void submitQuestion()
    }
  }

  async function focusInstruction(event: PointerEvent) {
    if (event.button !== 0) return
    const input = event.currentTarget as HTMLElement
    try {
      // The card is shown without activation. Activate only when the user
      // explicitly selects the input so the webview can receive keystrokes.
      await getCurrentWindow().setFocus()
      input.focus({ preventScroll: true })
    } catch (e) {
      console.error('Failed to focus Copilot input:', e)
    }
  }

  function handleInstKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      submitInstruction()
    }
  }

  onMount(() => {
    let unsubs: Array<() => void> = []
    getConfig().then(c => { forceId = !!c.force_indonesian }).catch(() => {})

    const setup = async () => {
      const offDelta = await listen<{id:string, question:string, delta:string, format:Format}>(
        'copilot-suggest-delta', ({payload}) => {
          const existing = history.find(h => h.id === payload.id)
          if (existing) {
            existing.text += payload.delta
          } else {
            history = [...history, {
              id: payload.id, question: payload.question, text: payload.delta,
              format: payload.format, done: false, timestamp: Date.now(),
            }].slice(-MAX_THREAD)
            pulse = true
            setTimeout(() => pulse = false, 600)
          }
          scrollChat()
        }
      )
      const offDone = await listen<{id:string, question:string, full_text:string}>('copilot-suggest-done', ({payload}) => {
        let s = history.find(h => h.id === payload.id)
        if (!s && payload.full_text) {
          // Done arrived without any delta (non-streaming provider).
          history = [...history, {
            id: payload.id, question: payload.question, text: payload.full_text,
            format: 'Bullets' as Format, done: false, timestamp: Date.now(),
          }].slice(-MAX_THREAD)
          s = history[history.length - 1]
        }
        if (s) {
          s.done = true
          scrollChat()
          // Schedule auto-dismiss if not pinned and timeout > 0
          const dismissAfterMs = settings.copilotAutoDismissS * 1000
          if (dismissAfterMs > 0) {
            setTimeout(() => {
              // Only dismiss if: still showing this suggestion as current, not pinned, no newer arrived
              if (!pinned && !editingInstruction && !editingQuestion && !instText.trim() && !questionText.trim()
                  && history[history.length - 1]?.id === payload.id) {
                getCurrentWindow().hide()
              }
            }, dismissAfterMs)
          }
        }
      })
      const offEnd = await listen('copilot-session-ended', async () => {
        await getCurrentWindow().hide()
        history = []
        finals = []
        partials = {}
        translations = {}

      })
      const srcOf = (s?: string): Source => (s === 'remote' || s === 'self' ? s : 'main')
      const offPartial = await listen<{ text: string; source?: string }>('copilot-transcript-partial', ({ payload }) => {
        partials = { ...partials, [srcOf(payload.source)]: payload.text }
        scrollTranscript()
      })
      const offFinal = await listen<{ text: string; source?: string; start_ms?: number }>('copilot-transcript-final', ({ payload }) => {
        const source = srcOf(payload.source)
        partials = { ...partials, [source]: '' }
        finals = [...finals, {
          id: ++finalSeq, text: payload.text, source, start_ms: payload.start_ms ?? 0,
        }].slice(-TRANSCRIPT_KEEP)
        scrollTranscript()
      })
      const offTranslated = await listen<{ original: string; text: string }>(
        'copilot-transcript-translated', ({ payload }) => {
          translations = { ...translations, [payload.original.trim()]: payload.text }
          scrollTranscript()
        },
      )
      const offShot = await listen<{ id: string; question: string; thumbnail: string }>(
        'screen-capture-started', ({ payload }) => {
          history = [...history, {
            id: payload.id, question: payload.question, text: '',
            format: 'Bullets' as Format, done: false, timestamp: Date.now(), thumb: payload.thumbnail,
          }].slice(-MAX_THREAD)
          monitorToast = ''
          pulse = true
          setTimeout(() => pulse = false, 600)
          scrollChat()
        },
      )
      const offMon = await listen<{ index: number; total: number; name: string }>(
        'screen-monitor-selected', ({ payload }) => {
          monitorToast = `Layar ${payload.index + 1}/${payload.total} — ${payload.name}`
          clearTimeout(toastTimer)
          toastTimer = setTimeout(() => (monitorToast = ''), 2500)
        },
      )
      unsubs = [offDelta, offDone, offEnd, offPartial, offFinal, offTranslated, offShot, offMon]
    }
    setup()
    return () => unsubs.forEach(u => u())
  })

  function copy(text: string) {
    navigator.clipboard.writeText(text)
  }
  function clearThread() { history = [] }
  function fmtTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
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
</script>

<div class="card" class:pulse>
  <div class="header" data-tauri-drag-region>
    <span class="status-dot" class:done={last?.done} class:loading={last && !last.done}></span>
    <span class="title">Copilot</span>
    <button class="x tr-toggle id-toggle" class:active={forceId} onclick={toggleForceId}
            title="Selalu Bahasa Indonesia: terjemahkan transkrip dan jawaban walau lawan bicara memakai bahasa lain"
            aria-label="Selalu Bahasa Indonesia">ID</button>
    <button class="x tr-toggle" class:active={showTranscript} onclick={toggleTranscript}
            title="Tampilkan/sembunyikan transkrip" aria-label="Transkrip">🎙</button>
    <button class="x" onclick={dismiss} aria-label="Tutup">✕</button>
  </div>

  <div class="screen-toolbar"><ScreenCaptureDialog /></div>

  {#if monitorToast}<div class="toast">{monitorToast}</div>{/if}
  <div class="body chat" bind:this={chatEl}>
    {#if history.length === 0}
      <div class="empty">Tanya AI atau baca layar untuk mulai…</div>
    {/if}
    {#each history as item, i (item.id)}
      <div class="qa">
        <div class="q">
          <div class="q-meta">Q{i + 1} · {fmtTime(item.timestamp)}</div>
          <div class="q-text">{tr(item.question) ?? item.question}</div>
          {#if item.thumb}<img class="thumb" src={item.thumb} alt="Tangkapan layar" />{/if}
          {#if tr(item.question)}<div class="q-orig">{item.question}</div>{/if}
        </div>
        <div class="a" class:streaming={!item.done}>
          {#if item.format === 'Code'}
            <pre class="code">{item.text}</pre>
          {:else}
            <div class="md">{@html renderMd(item.text)}</div>
          {/if}
          <button class="a-copy" onclick={() => copy(item.text)} title="Copy jawaban">📋</button>
        </div>
      </div>
    {/each}
  </div>

  {#if showTranscript}
    <div class="transcript" bind:this={transcriptEl} aria-label="Transkrip suara">
      {#if finals.length === 0 && partialLines.length === 0}
        <div class="tr-empty">Transkrip ucapan akan muncul di sini…</div>
      {/if}
      {#each finals as f (f.id)}
        <div class="tr-line" class:tr-self={f.source === 'self'}>
          {#if SOURCE_LABEL[f.source]}<span class="tr-tag">{SOURCE_LABEL[f.source]}</span>{/if}{tr(f.text) ?? f.text}
          {#if tr(f.text)}<div class="tr-orig">{f.text}</div>{/if}
        </div>
      {/each}
      {#each partialLines as [src, text] (src)}
        <div class="tr-line tr-partial" class:tr-self={src === 'self'}>
          {#if SOURCE_LABEL[src]}<span class="tr-tag">{SOURCE_LABEL[src]}</span>{/if}{text}
        </div>
      {/each}
    </div>
  {/if}

  <div class="ask-box">
    <textarea aria-label="Tanya AI" bind:value={questionText} rows="2"
      onpointerdown={focusInstruction}
      onfocus={() => editingQuestion = true} onblur={() => editingQuestion = false}
      onkeydown={handleQuestionKeydown}
      placeholder="Tanya AI tentang percakapan (tanpa gambar)…"></textarea>
    <button onclick={submitQuestion} disabled={!questionText.trim() || questionBusy} title="Kirim pertanyaan">Kirim</button>
  </div>
  {#if questionError}<div class="inst-feedback error" role="alert">{questionError}</div>{/if}

  <details class="instruction-settings">
    <summary>Instruksi cara menjawab</summary>
    <div class="inst-box">
    <input 
      type="text" 
      bind:value={instText} 
      oninput={() => { instructionStatus = ''; instructionError = '' }}
      aria-label="Instruksi Copilot"
      onpointerdown={focusInstruction}
      onfocus={() => editingInstruction = true}
      onblur={() => editingInstruction = false}
      onkeydown={handleInstKeydown}
      placeholder="Misalnya: gunakan TypeScript untuk soal kode"
      class="inst-input"
    />
    <button onclick={submitInstruction} class="inst-send" title="Simpan instruksi">Simpan</button>
    </div>
  </details>

  {#if instructionStatus}<div class="inst-feedback" role="status">{instructionStatus}</div>{/if}
  {#if instructionError}<div class="inst-feedback" role="alert">{instructionError}</div>{/if}

  <div class="footer">
    <span class="count">{history.length > 0 ? `${history.length} tanya-jawab` : ''}</span>
    <div class="actions">
      <button onclick={regenerate} title="Jawab ulang pertanyaan terakhir">↻</button>
      <button onclick={clearThread} title="Bersihkan percakapan">🗑</button>
      <button class:active={pinned} onclick={() => pinned = !pinned} title="Pin">📌</button>
    </div>
  </div>
</div>

<style>
  .screen-toolbar { padding: 6px 12px; border-bottom: 1px solid rgba(255,255,255,.06); }
  .ask-box { display: flex; align-items: end; gap: 6px; padding: 8px 12px; border-top: 1px solid rgba(255,255,255,.08); }
  .ask-box textarea { flex: 1; min-width: 0; max-height: 90px; resize: vertical; border-radius: 7px;
    border: 1px solid rgba(255,255,255,.14); background: rgba(255,255,255,.05); color: inherit; padding: 7px; font: inherit; }
  .ask-box button { border: 0; border-radius: 7px; padding: 8px 10px; background: #5e6ad2; color: white; cursor: pointer; }
  .ask-box button:disabled { opacity: .5; cursor: default; }
  .instruction-settings { border-top: 1px solid rgba(255,255,255,.06); }
  .instruction-settings summary { cursor: pointer; padding: 6px 12px; font-size: 11.5px; color: #b9c0ff; }
  .inst-feedback.error { color: #f79d9d; }
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
  .actions button {
    background: none; border: none; color: #b8b8c0; cursor: pointer;
    padding: 4px 6px; border-radius: 4px;
  }
  .actions button:hover { background: rgba(255,255,255,0.06); }
  .actions button.active { color: #5e6ad2; }

  /* Chat thread */
  .chat { display: flex; flex-direction: column; gap: 14px; }
  .qa { display: flex; flex-direction: column; gap: 6px; }
  .q {
    align-self: flex-end; max-width: 88%;
    background: rgba(94,106,210,0.22);
    border: 1px solid rgba(94,106,210,0.35);
    border-radius: 12px 12px 3px 12px;
    padding: 6px 10px;
  }
  .q-meta { font-size: 10px; color: #9a9ab0; margin-bottom: 2px; }
  .thumb { max-width: 100%; max-height: 80px; border-radius: 6px; margin-top: 6px; opacity: .9; }
  .toast { margin: 6px 12px; padding: 6px 10px; border-radius: 8px; background: rgba(94,106,210,.25); font-size: 12px; }
  .q-text { font-size: 12.5px; color: #f0f0f5; user-select: text; }
  .a {
    position: relative; align-self: flex-start; max-width: 96%;
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 12px 12px 12px 3px;
    padding: 8px 12px; user-select: text;
  }
  .a.streaming { border-color: rgba(94,106,210,0.5); }
  .a-copy {
    position: absolute; top: 2px; right: 2px;
    background: none; border: none; cursor: pointer;
    opacity: 0; font-size: 11px; padding: 2px 4px; border-radius: 4px;
  }
  .a:hover .a-copy { opacity: 0.7; }
  .a-copy:hover { background: rgba(255,255,255,0.08); opacity: 1; }
  .count { color: #8a8a96; }

  /* Live transcript */
  .tr-toggle { opacity: 0.5; }
  .tr-toggle.active { opacity: 1; }
  .transcript {
    max-height: 84px; overflow-y: auto;
    padding: 6px 12px;
    border-top: 1px solid rgba(255,255,255,0.06);
    background: rgba(0,0,0,0.12);
    font-size: 11.5px; line-height: 1.45;
    user-select: text;
  }
  .tr-line { color: #d4d4dc; margin-bottom: 2px; }
  .tr-partial { color: #8e8e9a; font-style: italic; }
  .id-toggle { font-size: 10px; font-weight: 700; letter-spacing: 0.3px; }
  .id-toggle.active { color: #7ee2b0; }
  .q-orig { margin-top: 2px; font-size: 10.5px; color: #9a9ab0; font-style: italic; }
  .tr-orig { font-size: 10.5px; color: #77778a; font-style: italic; }
  .tr-tag {
    display: inline-block; margin-right: 6px; padding: 0 5px;
    font-size: 9.5px; font-style: normal; border-radius: 4px;
    background: rgba(94,106,210,0.25); color: #b9c0ff;
  }
  .tr-self { color: #a8e6c8; }
  .tr-self .tr-tag { background: rgba(25,195,125,0.2); color: #7ee2b0; }
  .tr-empty { color: #666; font-style: italic; }

  /* Real-time instructions style */
  .inst-feedback { padding: 4px 12px 8px; font-size: 11px; color: #b9c0ff; }
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
