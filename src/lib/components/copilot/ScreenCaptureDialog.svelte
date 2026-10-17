<script lang="ts">
  import { onDestroy, tick } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import {
    listMonitors, capturePreview, discardPreview, submitPreview,
    type MonitorInfo, type CapturePreview,
  } from '$lib/screen'

  let open = $state(false)
  let monitors = $state<MonitorInfo[]>([])
  let selected = $state(0)
  let preview = $state<CapturePreview | null>(null)
  let question = $state('')
  let busy = $state(false)
  let error = $state('')
  let generation = 0
  let dialogEl = $state<HTMLDivElement | undefined>(undefined)

  onDestroy(() => {
    generation++
    void clearPreview()
  })

  async function openDialog() {
    try { await getCurrentWindow().setFocus() } catch {}
    open = true
    await tick()
    dialogEl?.focus()
    error = ''
    busy = true
    const token = ++generation
    try {
      const found = await listMonitors()
      if (token !== generation) return
      monitors = found
      selected = found[0]?.index ?? 0
      if (!found.length) error = 'Tidak ada monitor yang terdeteksi.'
    } catch (e) {
      if (token === generation) error = `Gagal membaca monitor: ${String(e)}`
    } finally {
      if (token === generation) busy = false
    }
  }

  async function clearPreview() {
    const old = preview
    preview = null
    if (old) await discardPreview(old.id).catch(() => {})
  }

  function closeDialog() {
    generation++
    open = false
    busy = false
    question = ''
    error = ''
    void clearPreview()
  }

  async function chooseMonitor(index: number) {
    selected = index
    generation++
    busy = false
    await clearPreview()
  }

  async function takePreview() {
    error = ''
    await clearPreview()
    busy = true
    const token = ++generation
    try {
      const shot = await capturePreview(selected)
      if (token !== generation || !open) {
        await discardPreview(shot.id)
        return
      }
      preview = shot
    } catch (e) {
      if (token === generation) error = `Gagal menangkap layar: ${String(e)}`
    } finally {
      if (token === generation) busy = false
    }
  }

  async function send() {
    if (!preview || busy) return
    busy = true
    error = ''
    try {
      await submitPreview(preview.id, question.trim())
      preview = null
      closeDialog()
    } catch (e) {
      error = `Gagal mengirim gambar: ${String(e)}`
      busy = false
    }
  }
</script>

<svelte:window onkeydown={(event) => { if (open && event.key === 'Escape') closeDialog() }} />

<button class="screen-open" onclick={openDialog} type="button">Baca layar</button>

{#if open}
  <div class="screen-backdrop" role="presentation" onclick={closeDialog}></div>
  <div class="screen-dialog" role="dialog" aria-modal="true" aria-label="Baca layar" tabindex="-1" bind:this={dialogEl}>
    <header>
      <h3>Baca layar</h3>
      <button class="close" type="button" aria-label="Tutup pratinjau" onclick={closeDialog}>✕</button>
    </header>
    <label for="screen-monitor">Monitor</label>
    <select id="screen-monitor" aria-label="Pilih monitor" value={selected}
      onchange={(e) => chooseMonitor(Number(e.currentTarget.value))} disabled={busy}>
      {#each monitors as monitor}
        <option value={monitor.index}>{monitor.name}</option>
      {/each}
    </select>
    <button class="secondary" type="button" onclick={takePreview} disabled={busy || !monitors.length}>
      {busy ? 'Memproses…' : 'Ambil pratinjau'}
    </button>
    {#if preview}
      <p class="caption">Pratinjau {preview.monitor_name}. Pastikan tidak ada informasi yang tidak ingin Anda kirim.</p>
      <img class="preview" src={preview.image} alt={`Pratinjau ${preview.monitor_name}`} />
      <label for="screen-question">Pertanyaan tentang layar (opsional)</label>
      <textarea id="screen-question" bind:value={question} rows="2"
        placeholder="Misalnya: jawab soal ini dengan TypeScript"></textarea>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <footer>
      <button class="secondary" type="button" onclick={closeDialog}>Batal</button>
      <button class="primary" type="button" onclick={send} disabled={!preview || busy}>Kirim ke AI</button>
    </footer>
  </div>
{/if}

<style>
  .screen-open, .primary, .secondary { border: 1px solid var(--border, #666); border-radius: 7px; padding: 7px 11px; cursor: pointer; font: inherit; }
  .screen-open, .primary { background: #5e6ad2; color: white; border-color: #5e6ad2; }
  .secondary { background: var(--bg, #292935); color: var(--text, #eee); }
  button:disabled { opacity: .5; cursor: default; }
  .screen-backdrop { position: fixed; inset: 0; z-index: 200; background: rgba(0,0,0,.5); }
  .screen-dialog { position: fixed; z-index: 201; left: 50%; top: 50%; transform: translate(-50%, -50%);
    width: min(460px, calc(100vw - 24px)); max-height: calc(100vh - 24px); overflow-y: auto;
    box-sizing: border-box; padding: 16px; display: flex; flex-direction: column; gap: 10px;
    background: var(--bg, #20202b); color: var(--text, #eee); border: 1px solid var(--border, #555); border-radius: 12px;
    box-shadow: 0 12px 36px rgba(0,0,0,.35); }
  header, footer { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  h3, p { margin: 0; } h3 { font-size: 16px; } label, .caption { font-size: 12px; }
  select, textarea { box-sizing: border-box; width: 100%; border: 1px solid var(--border, #666); border-radius: 7px;
    padding: 7px 8px; background: var(--bg, #292935); color: var(--text, #eee); font: inherit; }
  .preview { display: block; width: 100%; max-height: 45vh; object-fit: contain; background: #101016; border-radius: 6px; }
  .close { background: transparent; border: 0; color: inherit; cursor: pointer; }
  .error { color: #f79d9d; font-size: 12px; }
  footer { justify-content: flex-end; margin-top: 3px; }
</style>
