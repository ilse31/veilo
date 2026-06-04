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
        apiUrl, apiKey, model,
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
      <label>Preset</label>
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
      <label>Context window: {contextS}s</label>
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
