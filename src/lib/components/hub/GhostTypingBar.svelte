<script lang="ts">
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { onDestroy } from 'svelte'
  import { stopGhostTyping } from '$lib/tauri'

  interface Props {
    onSend: (text: string) => void
    onCancel: () => void
  }
  let { onSend, onCancel }: Props = $props()

  type GhostKey =
    | { type: 'Char'; value: string }
    | { type: 'Backspace' }
    | { type: 'Delete' }
    | { type: 'Enter' }
    | { type: 'Tab' }
    | { type: 'Escape' }
    | { type: 'Left' }
    | { type: 'Right' }
    | { type: 'Home' }
    | { type: 'End' }

  let text   = $state('')
  let cursor = $state(0)   // caret position within text

  let unlisten: UnlistenFn | null = null

  // Start listening as soon as the component mounts
  listen<GhostKey>('ghost-key', ({ payload }) => {
    switch (payload.type) {
      case 'Char':
        text   = text.slice(0, cursor) + payload.value + text.slice(cursor)
        cursor++
        break
      case 'Backspace':
        if (cursor > 0) {
          text = text.slice(0, cursor - 1) + text.slice(cursor)
          cursor--
        }
        break
      case 'Delete':
        if (cursor < text.length) {
          text = text.slice(0, cursor) + text.slice(cursor + 1)
        }
        break
      case 'Left':
        if (cursor > 0) cursor--
        break
      case 'Right':
        if (cursor < text.length) cursor++
        break
      case 'Home':
        cursor = 0
        break
      case 'End':
        cursor = text.length
        break
      case 'Tab':
        text   = text.slice(0, cursor) + '  ' + text.slice(cursor)
        cursor += 2
        break
      case 'Enter':
        handleSend()
        break
      case 'Escape':
        handleCancel()
        break
    }
  }).then(fn => { unlisten = fn })

  async function handleSend() {
    const msg = text.trim()
    text   = ''
    cursor = 0
    await stopGhostTyping()
    if (msg) onSend(msg)
  }

  async function handleCancel() {
    text   = ''
    cursor = 0
    await stopGhostTyping()
    onCancel()
  }

  onDestroy(() => unlisten?.())
</script>

<div class="ghost-bar">
  <div class="ghost-header">
    <span class="ghost-icon">👻</span>
    <span class="ghost-label">Mode Ketik Senyap</span>
    <kbd class="ghost-hint">Ctrl+Alt+G</kbd>
    <span class="ghost-hint-sep">·</span>
    <kbd class="ghost-hint">Enter</kbd>
    <span class="ghost-tip">kirim</span>
    <span class="ghost-hint-sep">·</span>
    <kbd class="ghost-hint">Esc</kbd>
    <span class="ghost-tip">batal</span>
  </div>

  <div class="ghost-input-wrap">
    <!-- Render text with caret position visualised -->
    <div class="ghost-input" aria-label="Teks yang diketik">
      {#if text.length === 0 && cursor === 0}
        <span class="ghost-placeholder">Mulai mengetik...</span>
      {:else}
        <span class="ghost-text-before">{text.slice(0, cursor)}</span><span
          class="ghost-caret"></span><span class="ghost-text-after">{text.slice(cursor)}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .ghost-bar {
    position: relative;
    background: color-mix(in srgb, var(--accent) 8%, var(--bg));
    border-top: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    padding: 10px 14px 12px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ghost-header {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-soft);
  }
  .ghost-icon { font-size: 13px; }
  .ghost-label {
    font-weight: 600;
    color: var(--accent);
    margin-right: 4px;
  }
  .ghost-hint {
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    font-size: 10px;
    font-family: monospace;
    color: var(--text);
  }
  .ghost-hint-sep { color: var(--border-strong); }
  .ghost-tip { color: var(--text-soft); }

  .ghost-input-wrap {
    background: var(--bg);
    border: 1.5px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: 8px;
    padding: 8px 12px;
    min-height: 36px;
    font-size: 13px;
    line-height: 1.5;
    word-break: break-all;
  }

  .ghost-input {
    display: inline;
    color: var(--text);
    white-space: pre-wrap;
  }
  .ghost-placeholder {
    color: var(--text-soft);
    font-style: italic;
  }
  .ghost-caret {
    display: inline-block;
    width: 2px;
    height: 1em;
    background: var(--accent);
    vertical-align: text-bottom;
    animation: blink 1s step-end infinite;
  }
  @keyframes blink {
    50% { opacity: 0; }
  }
  .ghost-text-before,
  .ghost-text-after {
    white-space: pre-wrap;
  }
</style>
