<script lang="ts">
  import { onMount } from 'svelte'
  import LanguageSelect from './LanguageSelect.svelte'
  import HotkeyRecorder from './HotkeyRecorder.svelte'
  import { getConfig, saveConfig, DEFAULT_CONFIG } from '$lib/tauri'
  import type { Config } from '$lib/tauri'
  import { config } from '$lib/stores'
  import { get } from 'svelte/store'

  let form: Config = $state({ ...DEFAULT_CONFIG })
  let saving = $state(false)
  let saveError = $state('')
  let saved = $state(false)

  onMount(async () => {
    try {
      const loaded = await getConfig()
      form = { ...loaded }
      config.set(loaded)
    } catch { /* use defaults */ }
  })

  async function handleSave() {
    saving = true; saveError = ''; saved = false
    try {
      await saveConfig(form)
      config.set({ ...form })
      saved = true
      setTimeout(() => (saved = false), 2000)
    } catch (e) {
      saveError = String(e)
    } finally {
      saving = false
    }
  }

  function handleCancel() { form = { ...get(config) } }
</script>

<div class="min-h-screen bg-gray-900 text-white p-6 font-sans">
  <h1 class="text-base font-semibold mb-6 text-gray-100">noscreen — Settings</h1>

  <div class="mb-5">
    <p class="text-xs text-gray-400 mb-2 uppercase tracking-wide">AI Site</p>
    <div class="flex gap-4">
      <label class="flex items-center gap-2 cursor-pointer text-sm">
        <input type="radio" bind:group={form.site} value="https://claude.ai" class="accent-blue-500" /> Claude.ai
      </label>
      <label class="flex items-center gap-2 cursor-pointer text-sm">
        <input type="radio" bind:group={form.site} value="https://chatgpt.com" class="accent-blue-500" /> ChatGPT
      </label>
    </div>
  </div>

  <div class="mb-5">
    <p class="text-xs text-gray-400 mb-2 uppercase tracking-wide">Speech Language</p>
    <LanguageSelect bind:value={form.language} />
  </div>

  <div class="mb-5">
    <p class="text-xs text-gray-400 mb-2 uppercase tracking-wide">Global Hotkey</p>
    <HotkeyRecorder bind:value={form.hotkey} />
    <p class="text-xs text-gray-500 mt-1">Hotkey change takes effect on next app restart.</p>
  </div>

  <div class="mb-5">
    <p class="text-xs text-gray-400 mb-2 uppercase tracking-wide">Overlay Opacity — {Math.round(form.opacity * 100)}%</p>
    <input type="range" min="0.3" max="1" step="0.05" bind:value={form.opacity} class="w-full accent-blue-500" />
  </div>

  <div class="mb-6 flex items-center justify-between">
    <span class="text-sm text-gray-300">Auto-start on boot</span>
    <button onclick={() => (form.autostart = !form.autostart)}
      class="relative inline-flex h-6 w-11 rounded-full transition-colors duration-200 {form.autostart ? 'bg-blue-600' : 'bg-gray-600'}"
      aria-checked={form.autostart} role="switch" aria-label="Auto-start on boot">
      <span class="absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white transition-transform duration-200 {form.autostart ? 'translate-x-5' : 'translate-x-0'}"></span>
    </button>
  </div>

  {#if saveError}<p class="text-red-400 text-xs mb-3">{saveError}</p>{/if}
  {#if saved}<p class="text-green-400 text-xs mb-3">Settings saved.</p>{/if}

  <div class="flex gap-3">
    <button onclick={handleSave} disabled={saving} aria-label="Save settings"
      class="flex-1 py-2 rounded-md bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-sm font-medium transition-colors">
      {saving ? 'Saving...' : 'Save'}
    </button>
    <button onclick={handleCancel}
      class="flex-1 py-2 rounded-md bg-gray-700 hover:bg-gray-600 text-sm font-medium transition-colors">
      Cancel
    </button>
  </div>
</div>
