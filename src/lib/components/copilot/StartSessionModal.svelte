<script lang="ts">
  import { onMount } from 'svelte'
  import { copilotGetPresets, copilotStartSession, type Preset } from '$lib/copilot/api'

  interface Props {
    open: boolean
    apiUrl: string
    apiKey: string
    model: string
    onClose: () => void
    onStarted: (sid: number) => void
  }
  let { open, apiUrl, apiKey, model, onClose, onStarted }: Props = $props()

  let presets = $state<Preset[]>([])
  let selectedId = $state('generic')
  let contextS = $state(90)
  let save = $state(true)
  let error = $state('')
  let busy = $state(false)
  let customContext = $state('')

  function handleFileUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      const reader = new FileReader();
      reader.onload = (event) => {
        if (event.target && typeof event.target.result === 'string') {
          customContext = event.target.result;
        }
      };
      reader.readAsText(file);
    }
  }

  onMount(async () => {
    presets = await copilotGetPresets()
    if (presets.length > 0) {
      selectedId = presets[0].id
      contextS = presets[0].default_context_s
    }
  })

  function selectPreset(id: string) {
    selectedId = id
    const p = presets.find(p => p.id === id)
    if (p) contextS = p.default_context_s
  }

  async function start() {
    if (busy) return
    busy = true
    error = ''
    try {
      const sid = await copilotStartSession({
        presetId: selectedId, contextWindowS: contextS, save,
        apiUrl, apiKey, model, customContext,
      })
      onStarted(sid)
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }
</script>

{#if open}
  <div class="overlay" onclick={onClose} role="presentation"></div>
  <div class="modal">
    <h3>Mulai Sesi Copilot</h3>

    <div class="field">
      <span class="field-title" style="display: block; font-size: 12.5px; color: var(--text-soft); margin-bottom: 6px;">Preset</span>
      <div class="presets">
        {#each presets as p}
          <label class="preset-opt">
            <input type="radio" name="preset"
                   checked={selectedId === p.id}
                   onchange={() => selectPreset(p.id)} />
            <span>{p.name}</span>
          </label>
        {/each}
      </div>
    </div>

    <div class="field">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;">
        <span class="field-title" style="font-size: 12.5px; color: var(--text-soft);">Context Latar Belakang (CV, Deskripsi Kerja, dll.)</span>
        <label style="margin: 0; cursor: pointer; color: var(--accent, #5e6ad2); font-size: 11.5px; font-weight: 500;">
          Upload .txt
          <input type="file" accept=".txt" onchange={handleFileUpload} style="display: none;" />
        </label>
      </div>
      <textarea 
        class="hub-input" 
        style="width: 100%; height: 75px; resize: none; font-size: 12px; font-family: inherit; box-sizing: border-box; padding: 8px; border: 1px solid var(--border); border-radius: 6px; background: rgba(255,255,255,0.02); color: inherit;"
        bind:value={customContext} 
        placeholder="Tempel CV, pertanyaan kisi-kisi wawancara, deskripsi pekerjaan, atau topik diskusi di sini untuk memandu asisten..."
      ></textarea>
    </div>

    <div class="field">
      <span class="field-title" style="display: block; font-size: 12.5px; color: var(--text-soft); margin-bottom: 6px;">Context window: {contextS}s</span>
      <input type="range" min="30" max="300" step="10" bind:value={contextS} />
    </div>

    <div class="field">
      <label class="check">
        <input type="checkbox" bind:checked={save} />
        <span>Simpan transkrip & saran ke history</span>
      </label>
      <div class="hint">Uncheck = ephemeral, terhapus saat sesi berakhir</div>
    </div>

    {#if error}<div class="error">{error}</div>{/if}

    <div class="actions">
      <button class="hub-btn secondary" onclick={onClose} disabled={busy}>Batal</button>
      <button class="hub-btn accent" onclick={start} disabled={busy}>
        {busy ? 'Memulai…' : 'Mulai →'}
      </button>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 100; }
  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    background: var(--bg); border: 1px solid var(--border); border-radius: 12px;
    padding: 24px; width: 420px; z-index: 101;
  }
  .modal h3 { margin: 0 0 18px; font-size: 16px; }
  .field { margin-bottom: 16px; }
  .field label { display: block; font-size: 12.5px; color: var(--text-soft); margin-bottom: 6px; }
  .presets { display: flex; flex-direction: column; gap: 6px; }
  .preset-opt { display: flex; align-items: center; gap: 8px; font-size: 13px; cursor: pointer; }
  .check { display: flex; align-items: center; gap: 8px; cursor: pointer; }
  .hint { font-size: 11.5px; color: var(--text-soft); margin-top: 4px; margin-left: 20px; }
  .error { color: var(--red, #e74c3c); font-size: 12px; margin-bottom: 12px; }
  .actions { display: flex; justify-content: flex-end; gap: 10px; }
</style>
