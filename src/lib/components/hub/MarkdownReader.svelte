<script lang="ts">
  import { onMount } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import { marked } from 'marked'
  import DOMPurify from 'dompurify'
  import { readMarkdownFile, getProfileValue, setProfileValue } from '$lib/tauri'
  import { icons } from './icons'

  marked.setOptions({ breaks: true, gfm: true })

  const RECENT_KEY = 'markdown_recent_files'
  const RECENT_LIMIT = 15

  interface RecentFile {
    path: string
    name: string
  }

  let fileName = $state<string | null>(null)
  let filePath = $state<string | null>(null)
  let rawContent = $state('')
  let error = $state<string | null>(null)
  let loading = $state(false)
  let recentFiles = $state<RecentFile[]>([])

  const html = $derived(rawContent ? DOMPurify.sanitize(marked.parse(rawContent) as string) : '')

  onMount(async () => {
    try {
      const stored = await getProfileValue(RECENT_KEY)
      if (stored) recentFiles = JSON.parse(stored)
    } catch {}
  })

  async function saveRecent(list: RecentFile[]) {
    recentFiles = list
    try {
      await setProfileValue(RECENT_KEY, JSON.stringify(list))
    } catch {}
  }

  function rememberFile(path: string, name: string) {
    const withoutCurrent = recentFiles.filter(f => f.path !== path)
    saveRecent([{ path, name }, ...withoutCurrent].slice(0, RECENT_LIMIT))
  }

  function forgetFile(path: string) {
    saveRecent(recentFiles.filter(f => f.path !== path))
  }

  async function loadFile(path: string) {
    error = null
    loading = true
    try {
      rawContent = await readMarkdownFile(path)
      filePath = path
      fileName = path.split(/[\\/]/).pop() ?? path
      rememberFile(path, fileName)
    } catch (e) {
      error = String(e)
      forgetFile(path)
    } finally {
      loading = false
    }
  }

  async function pickFile() {
    error = null
    let selected: string | null
    try {
      selected = await open({
        multiple: false,
        filters: [{ name: 'Markdown', extensions: ['md', 'markdown'] }],
      })
    } catch (e) {
      error = String(e)
      return
    }
    if (!selected) return
    await loadFile(selected)
  }
</script>

<div class="md-reader">
  <div class="md-toolbar">
    <button class="hub-btn accent" onclick={pickFile} disabled={loading}>
      {loading ? 'Membuka...' : 'Buka file Markdown'}
    </button>
    {#if filePath}
      <span class="md-path" title={filePath}>{fileName}</span>
    {/if}
  </div>

  {#if recentFiles.length}
    <div class="md-recent">
      <span class="md-recent-label">Riwayat</span>
      {#each recentFiles as f (f.path)}
        <span class="md-chip" class:active={f.path === filePath}>
          <button class="md-chip-open" onclick={() => loadFile(f.path)} title={f.path}>
            {f.name}
          </button>
          <button class="md-chip-remove" onclick={() => forgetFile(f.path)} aria-label="Hapus dari riwayat">×</button>
        </span>
      {/each}
    </div>
  {/if}

  {#if error}
    <div class="md-error">{error}</div>
  {/if}

  <div class="md-body">
    {#if !filePath && !error}
      <div class="md-empty">
        <span class="ic">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
            {@html icons.fileText}
          </svg>
        </span>
        <p>Pilih file .md atau .markdown untuk dibaca.</p>
      </div>
    {:else if filePath}
      <article class="md-content">{@html html}</article>
    {/if}
  </div>
</div>

<style>
  .md-reader {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 20px 24px;
    gap: 14px;
  }
  .md-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .md-path {
    font-size: 12.5px;
    color: var(--text-soft);
    font-family: var(--mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .md-recent {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .md-recent-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-right: 2px;
  }
  .md-chip {
    display: flex;
    align-items: center;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 999px;
    overflow: hidden;
  }
  .md-chip.active {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: var(--accent-soft);
  }
  .md-chip-open {
    padding: 4px 4px 4px 10px;
    font-size: 12px;
    color: var(--text-mid);
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .md-chip.active .md-chip-open { color: var(--accent); }
  .md-chip-open:hover { color: var(--text); }
  .md-chip-remove {
    padding: 4px 8px;
    font-size: 13px;
    line-height: 1;
    color: var(--text-faint);
  }
  .md-chip-remove:hover { color: var(--red); }
  .md-error {
    font-size: 13px;
    color: var(--red);
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
  }
  .md-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  .md-empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-soft);
    font-size: 13px;
  }
  .md-empty .ic { color: var(--text-faint); }
  .md-content {
    padding: 24px 28px;
    color: var(--text);
    font-size: 14px;
    line-height: 1.65;
  }
  .md-content :global(h1),
  .md-content :global(h2),
  .md-content :global(h3) {
    margin: 1.2em 0 0.5em;
    line-height: 1.3;
  }
  .md-content :global(h1) { font-size: 1.6em; }
  .md-content :global(h2) { font-size: 1.3em; }
  .md-content :global(h3) { font-size: 1.1em; }
  .md-content :global(p) { margin: 0.6em 0; }
  .md-content :global(a) { color: var(--accent); }
  .md-content :global(code) {
    font-family: var(--mono);
    background: var(--bg-hover);
    padding: 0.1em 0.35em;
    border-radius: 4px;
    font-size: 0.9em;
  }
  .md-content :global(pre) {
    background: var(--bg-hover);
    padding: 12px 14px;
    border-radius: 8px;
    overflow-x: auto;
  }
  .md-content :global(pre code) { background: none; padding: 0; }
  .md-content :global(blockquote) {
    margin: 0.8em 0;
    padding: 4px 14px;
    border-left: 3px solid var(--border-strong);
    color: var(--text-mid);
  }
  .md-content :global(ul),
  .md-content :global(ol) { padding-left: 1.4em; margin: 0.6em 0; }
  .md-content :global(table) { border-collapse: collapse; width: 100%; margin: 0.8em 0; }
  .md-content :global(th),
  .md-content :global(td) {
    border: 1px solid var(--border);
    padding: 6px 10px;
    font-size: 0.92em;
  }
  .md-content :global(img) { max-width: 100%; }
</style>
