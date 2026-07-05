<script lang="ts">
  import { onMount } from 'svelte'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { icons } from './icons'
  import { settings } from '$lib/stores/settings.svelte'
  import {
    listConversations, createConversation, deleteConversation,
    getMessages, appendMessage, updateConversationTitle,
    chatSend, chatStop, type ConvRow, type MsgRow, type ChatMessage, type ChatDeltaEvent,
  } from '$lib/tauri'

  interface Props {
    apiUrl?: string
    apiKey?: string
    model?: string
    initialPrompt?: string | null
    onConsumeInitial?: () => void
  }
  let {
    apiUrl = '',
    apiKey = '',
    model = 'gpt-4o-mini',
    initialPrompt = null,
    onConsumeInitial,
  }: Props = $props()

  type Msg = { role: 'user' | 'assistant'; body: string }

  let convList    = $state<ConvRow[]>([])
  let activeConvId = $state<number | null>(null)
  let msgs        = $state<Msg[]>([])
  let input       = $state('')
  let streaming   = $state(false)
  let streamBuf   = $state('')
  let streamConvId = $state<number | null>(null)  // conversation the in-flight stream belongs to
  let scrollEl    = $state<HTMLElement | null>(null)
  let activeId    = ''               // current stream id (for cancellation)
  let abortUnlisten: UnlistenFn | null = null

  onMount(async () => {
    await loadConvList()
  })

  $effect(() => {
    if (initialPrompt) {
      send(initialPrompt)
      onConsumeInitial?.()
    }
  })

  $effect(() => {
    if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight
  })

  async function loadConvList() {
    convList = await listConversations()
  }

  async function selectConv(id: number) {
    activeConvId = id
    const rows: MsgRow[] = await getMessages(id)
    msgs = rows.map(r => ({ role: r.role, body: r.body }))
    streamBuf = ''
  }

  async function newChat() {
    // Abort any in-flight stream first; its finally-block resets `streaming`.
    await stopStream()
    activeConvId = null
    msgs         = []
    streamBuf    = ''
  }

  async function removeConv(id: number) {
    await deleteConversation(id)
    if (activeConvId === id) await newChat()
    await loadConvList()
  }

  async function stopStream() {
    if (activeId) {
      await chatStop(activeId).catch(() => {})
    }
    if (abortUnlisten) { abortUnlisten(); abortUnlisten = null }
  }

  async function send(override?: string) {
    const text = (override ?? input).trim()
    if (!text || streaming) return
    input = ''

    // Create conversation on first message
    if (activeConvId === null) {
      const title = text.length > 50 ? text.slice(0, 50) + '…' : text
      activeConvId = await createConversation(title)
      await loadConvList()
    }

    const convId = activeConvId!

    // Persist + show user message
    await appendMessage(convId, 'user', text)
    msgs = [...msgs, { role: 'user', body: text }]

    // Auto-rename conv after first real user message if still default
    const conv = convList.find(c => c.id === convId)
    if (conv && conv.msg_count === 0) {
      const title = text.length > 50 ? text.slice(0, 50) + '…' : text
      await updateConversationTitle(convId, title)
      await loadConvList()
    }

    streaming = true
    streamBuf = ''
    streamConvId = convId

    if (!apiUrl) {
      const body = 'URL API belum diatur. Buka Pengaturan → Koneksi AI Lokal.'
      await appendMessage(convId, 'assistant', body)
      msgs = [...msgs, { role: 'assistant', body }]
      streaming = false
      return
    }

    const history: ChatMessage[] = msgs.slice(0, -1).map(m => ({ role: m.role, content: m.body }))
    const id = crypto.randomUUID()
    activeId = id

    // Subscribe to delta events for this stream id only.
    abortUnlisten = await listen<ChatDeltaEvent>('chat-delta', (e) => {
      if (e.payload.id !== id) return
      streamBuf += e.payload.delta
    })

    try {
      const full = await chatSend(id, apiUrl, apiKey || '', model,
        [...history, { role: 'user', content: text }])

      const body = full || '(tidak ada respons)'
      await appendMessage(convId, 'assistant', body)
      // Only touch the UI if the user is still viewing this conversation —
      // they may have switched away mid-stream (DB write above is always correct).
      if (activeConvId === convId) msgs = [...msgs, { role: 'assistant', body }]
    } catch (e) {
      // If something was already streamed before the error, keep it.
      const partial = streamBuf ? streamBuf + '\n\n' : ''
      const body = `${partial}Gagal: ${e}`
      await appendMessage(convId, 'assistant', body)
      if (activeConvId === convId) msgs = [...msgs, { role: 'assistant', body }]
    } finally {
      streaming = false
      streamBuf  = ''
      streamConvId = null
      activeId   = ''
      if (abortUnlisten) { abortUnlisten(); abortUnlisten = null }
      await loadConvList()
    }
  }

  function stop() { stopStream() }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); send() }
  }

  const displayName = $derived(settings.profileName ?? 'Kamu')

  const starters = [
    { t: 'Tulis fungsi debounce di TypeScript', s: 'dengan tipe generic yang benar' },
    { t: 'Ringkas dokumen PRD ini',             s: 'jadi 5 poin eksekutif' },
    { t: 'Generate SQL untuk laporan bulanan',  s: 'gabungkan tabel orders & users' },
    { t: 'Buat outline presentasi internal',    s: '10 slide, 15 menit' },
  ]
</script>

<div class="hub-chat-shell">
  <!-- Conversation sidebar -->
  <aside class="hub-chat-side">
    <div class="top">
      <button class="hub-btn secondary" style="width:100%;justify-content:center" onclick={newChat}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="{icons.plus}"/>
        </svg>
        Obrolan baru
      </button>
    </div>

    <div class="list">
      {#if convList.length === 0}
        <div class="empty-hist">Belum ada obrolan</div>
      {:else}
        {#each convList as c}
          <div
            class="conv {activeConvId === c.id ? 'active' : ''}"
            onclick={() => selectConv(c.id)}
            role="button" tabindex="0"
            onkeydown={(e) => e.key === 'Enter' && selectConv(c.id)}
          >
            <div class="t">{c.title}</div>
            <div class="conv-meta">
              <span class="s">{c.msg_count} pesan</span>
              <button
                class="del-btn"
                onclick={(e) => { e.stopPropagation(); removeConv(c.id) }}
                aria-label="Hapus obrolan"
              >×</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <div class="api-footer">
      <div class="api-row">
        <span class="hub-dot {apiKey ? 'g' : 'a'}"></span>
        <span>{apiKey ? 'API tersambung' : 'API belum dikonfigurasi'}</span>
      </div>
      <div class="api-url">{(apiUrl || '—').replace(/^https?:\/\//, '')}</div>
    </div>
  </aside>

  <!-- Chat main -->
  <div class="hub-chat-main">
    {#if msgs.length === 0 && !streaming}
      <div class="hub-chat-empty">
        <div class="glyph">N</div>
        <div class="ttl">Mulai obrolan baru</div>
        <div class="sub">Pakai model {model} via endpoint kamu sendiri. Streaming aktif.</div>
        <div class="starter-row">
          {#each starters as s}
            <div class="starter" onclick={() => send(s.t + ' — ' + s.s)} role="button" tabindex="0" onkeydown={() => {}}>
              <div class="stt">{s.t}</div>
              <div class="ssb">{s.s}</div>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="hub-chat-msgs" bind:this={scrollEl}>
        {#each msgs as m}
          <div class="hub-msg {m.role}">
            <div class="av">{m.role === 'user' ? displayName[0].toUpperCase() : 'N'}</div>
            <div>
              <div class="role">{m.role === 'user' ? displayName : `AI Lokal · ${model}`}</div>
              <div class="body">{m.body}</div>
            </div>
          </div>
        {/each}
        {#if streaming && activeConvId === streamConvId}
          <div class="hub-msg assistant">
            <div class="av">N</div>
            <div>
              <div class="role">AI Lokal · {model}</div>
              <div class="body">{streamBuf}<span class="hub-cursor-blink"></span></div>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <div class="hub-chat-input-wrap">
      <div class="hub-chat-input-box">
        <textarea
          placeholder="Tulis pesan… (⌘↵ untuk kirim)"
          bind:value={input}
          onkeydown={handleKeydown}
          rows={2}
        ></textarea>
        <div class="row-b">
          <div class="model-pill">
            <span class="hub-dot g"></span>
            {model}
          </div>
          <div class="model-pill" title="Streaming aktif">
            <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor" stroke="none">
              <path d="{icons.zap}"/>
            </svg>
            stream
          </div>
          {#if streaming}
            <button class="stop-btn" onclick={stop}>
              <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor" stroke="none">
                <path d="{icons.stop}"/>
              </svg>
              Berhenti
            </button>
          {:else}
            <button class="send-btn" disabled={!input.trim()} onclick={() => send()} aria-label="Kirim pesan">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <path d="{icons.arrowUp}"/>
              </svg>
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .empty-hist {
    padding: 16px 12px;
    font-size: 12px;
    color: var(--text-faint);
    text-align: center;
  }

  .conv-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .del-btn {
    display: none;
    background: none;
    border: none;
    color: var(--text-faint);
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;
    cursor: default;
    border-radius: 3px;
  }
  .del-btn:hover { color: var(--red); background: rgba(210,74,74,.1); }

  .conv:hover .del-btn { display: block; }

  .api-footer {
    padding: 10px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-soft);
  }
  .api-row { display: flex; align-items: center; gap: 6px; }
  .api-url {
    margin-top: 4px;
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-faint);
    word-break: break-all;
  }
</style>
