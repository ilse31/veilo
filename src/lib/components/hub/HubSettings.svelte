<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { getConfig, saveConfig, setAllContentProtected, setProfileValue, testAiConnection } from '$lib/tauri'
  import { settings } from '$lib/stores/settings.svelte'
  import { setServiceUrl } from './webviewManager'
  import HotkeyRecorder from '$lib/components/HotkeyRecorder.svelte'
  import {
    copilotListCustomPresets, copilotCreatePreset, copilotDeletePreset,
    type CustomPresetRow, type ResponseFormat,
  } from '$lib/copilot/api'

  type Tab = 'connection' | 'privacy' | 'shortcuts' | 'copilot'
  const tabs: { id: Tab; label: string }[] = [
    { id: 'connection', label: 'Koneksi' },
    { id: 'privacy',    label: 'Privasi & Tampilan' },
    { id: 'shortcuts',  label: 'Pintasan' },
    { id: 'copilot',    label: 'Copilot' },
  ]
  let activeTab = $state<Tab>('connection')

  let testing = $state<null | 'pending' | 'ok' | 'fail'>(null)
  let testMsg = $state('')
  let saved = $state(false)
  let saveError = $state('')
  let showKey = $state(false)
  let models = $state<string[]>([])
  let modelsOpen = $state(false)
  let requestId = 0

  function resetConnection() {
    requestId++
    testing = null
    testMsg = ''
    models = []
    modelsOpen = false
    settings.model = ''
  }

  // ── Live: apply opacity change immediately via CSS (hub-root)
  // (No Tauri API needed — Hub.svelte binds settings.opacity to style)

  // ── Live: toggle content protection on all windows
  async function applyContentProtected(val: boolean) {
    settings.contentProtected = val
    try {
      await setAllContentProtected(val)
    } catch (e) {
      console.error('[settings] setAllContentProtected failed:', e)
    }
  }

  // ── Live: apply always-on-top when toggled
  async function applyAlwaysOnTop(val: boolean) {
    settings.alwaysOnTop = val
    try {
      await getCurrentWindow().setAlwaysOnTop(val)
    } catch (e) {
      console.error('[settings] setAlwaysOnTop failed:', e)
    }
  }

  // ── Real API test — runs via Tauri Rust command (bypasses browser CORS)
  async function testConnection() {
    if (!settings.apiUrl.trim()) { testing = 'fail'; testMsg = 'URL tidak boleh kosong'; return }
    const currentRequest = ++requestId
    testing = 'pending'
    testMsg = ''
    models = []
    modelsOpen = false
    try {
      const result = await testAiConnection(settings.apiUrl.trim(), settings.apiKey || '')
      if (currentRequest !== requestId) return
      const { status } = result
      if (status >= 200 && status < 300) {
        testing = 'ok'
        models = result.models
        if (!models.includes(settings.model)) settings.model = ''
        modelsOpen = models.length > 0 && !settings.model
        testMsg = models.length > 0
          ? `${models.length} model tersedia`
          : 'Koneksi berhasil, tetapi tidak ada model tersedia.'
      } else {
        testing = 'fail'
        testMsg = `HTTP ${status}`
      }
    } catch (e) {
      if (currentRequest !== requestId) return
      testing = 'fail'
      testMsg = String(e)
    }
  }

  // ── Save: persist opacity to config, apply webview URL overrides
  async function handleSave() {
    saveError = ''
    try {
      // Apply webview URL overrides to the manager
      setServiceUrl('gpt',       settings.urlGpt)
      setServiceUrl('claude',    settings.urlClaude)
      setServiceUrl('translate', settings.urlTranslate)

      // Save credentials to SQLite
      await setProfileValue('api_key', settings.apiKey)
      await setProfileValue('api_url', settings.apiUrl)
      await setProfileValue('model', settings.model)

      // Persist to disk — merge onto a fresh config so fields this page does
      // NOT edit (site, language, position) aren't clobbered with stale values.
      const fresh = await getConfig()
      await saveConfig({
        ...fresh,
        hotkey:                 settings.hotkey,
        opacity:                settings.opacity / 100,
        autostart:              settings.autostart,
        stt_backend:            settings.sttBackend,
        whisper_model:          settings.whisperModel,
        whisper_local_url:      settings.whisperLocalUrl,
        copilot_context_s:      settings.copilotContextS,
        copilot_auto_dismiss_s: settings.copilotAutoDismissS,
        copilot_min_interval_s: settings.copilotMinIntervalS,
        stt_language:           settings.sttLanguage,
      })

      saved = true
      setTimeout(() => saved = false, 2000)
    } catch (e) {
      saveError = String(e)
    }
  }

  // ── Custom Copilot presets ──────────────────────────────────────────────
  let customPresets = $state<CustomPresetRow[]>([])
  let presetName = $state('')
  let presetPrompt = $state('')
  let presetFormat = $state<ResponseFormat>('Bullets')
  let presetContextS = $state(90)
  let presetSaving = $state(false)
  let presetError = $state('')

  async function loadCustomPresets() {
    try {
      customPresets = await copilotListCustomPresets()
    } catch (e) {
      console.error('[settings] failed to load custom presets', e)
    }
  }

  $effect(() => {
    if (activeTab === 'copilot') loadCustomPresets()
  })

  async function addPreset() {
    if (!presetName.trim() || !presetPrompt.trim()) return
    presetSaving = true
    presetError = ''
    try {
      await copilotCreatePreset({
        name: presetName.trim(),
        systemPrompt: presetPrompt.trim(),
        responseFormat: presetFormat,
        defaultContextS: presetContextS,
      })
      presetName = ''
      presetPrompt = ''
      presetFormat = 'Bullets'
      presetContextS = 90
      await loadCustomPresets()
    } catch (e) {
      presetError = String(e)
    } finally {
      presetSaving = false
    }
  }

  async function removePreset(id: string) {
    try {
      await copilotDeletePreset(id)
      await loadCustomPresets()
    } catch (e) {
      presetError = String(e)
    }
  }
</script>

<div class="hub-page-scroll">
  <div class="hub-page-pad">
    <h2 class="hub-greeting" style="font-size:19px;margin-bottom:16px">Pengaturan</h2>

    <div class="hub-s-tabs" role="tablist">
      {#each tabs as t}
        <button
          class="hub-s-tab {activeTab === t.id ? 'active' : ''}"
          role="tab"
          aria-selected={activeTab === t.id}
          onclick={() => activeTab = t.id}
        >
          {t.label}
        </button>
      {/each}
    </div>

    <div class="hub-settings-grid">

    {#if activeTab === 'connection'}
      <!-- ── API Connection -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">Koneksi AI Lokal</div>
          <div class="d">Endpoint & kredensial untuk fitur "Obrolan AI" (kompatibel OpenAI API).</div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">URL Endpoint</div>
            <div class="l-desc">Base URL dari API. Path <code>/v1/chat/completions</code> ditambah otomatis.</div>
          </div>
          <div class="field-wrap">
            <input class="hub-input" bind:value={settings.apiUrl} oninput={resetConnection} placeholder="https://api.openai.com" />
            <div style="display:flex;gap:6px;flex-wrap:wrap">
              {#each [['https://api.openai.com','OpenAI'],['http://localhost:11434','Ollama'],['https://api.groq.com/openai','Groq']] as [u, l]}
                <span class="hub-hint-tag" onclick={() => { settings.apiUrl = u; resetConnection() }} role="button" tabindex="0" onkeydown={() => {}} style="cursor:default">{l}</span>
              {/each}
            </div>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">API Key</div>
            <div class="l-desc">Disimpan secara lokal di perangkat (SQLite). Tidak pernah dikirim ke server lain selain endpoint AI di atas.</div>
          </div>
          <div class="field-wrap">
            <div class="hub-key-input">
              <input class="hub-input"
                     type={showKey ? 'text' : 'password'}
                     bind:value={settings.apiKey}
                     oninput={resetConnection}
                     placeholder="sk-..." />
              <button class="hub-btn secondary" onclick={() => showKey = !showKey} aria-label="Toggle key visibility">
                {#if showKey}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M3 3l18 18M10.5 5.2A10 10 0 0112 5c6.5 0 10 7 10 7a17 17 0 01-3.4 4.4M6.6 6.6A17 17 0 002 12s3.5 7 10 7a10 10 0 005.4-1.6M9.9 9.9A3 3 0 0014 14"/></svg>
                {:else}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="2.8"/></svg>
                {/if}
              </button>
            </div>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Model default</div>
            <div class="l-desc">Dipakai untuk Obrolan AI & Prompt Cepat di Dasbor.</div>
          </div>
          <div class="field-wrap">
            {#if models.length > 0}
              <details class="model-picker" bind:open={modelsOpen}>
                <summary class="hub-input" aria-label="Model default">{settings.model || 'Pilih model'}</summary>
                <div class="model-options">
                  {#each models as model}
                    <button class="hub-input" class:selected={settings.model === model}
                      aria-pressed={settings.model === model}
                      onclick={() => { settings.model = model; modelsOpen = false }}>
                      {model}
                    </button>
                  {/each}
                </div>
              </details>
            {:else}
              <input class="hub-input" aria-label="Model default" value={settings.model}
                readonly placeholder="Uji koneksi untuk memuat daftar model" />
            {/if}
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Uji koneksi</div>
            <div class="l-desc">Kirim GET <code>/v1/models</code> ke endpoint untuk validasi. Untuk Ollama tidak perlu key.</div>
          </div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-btn" onclick={testConnection} disabled={testing === 'pending'}>
              {testing === 'pending' ? 'Menguji…' : 'Uji koneksi'}
            </button>
            {#if testing === 'ok'}
              <span style="display:flex;align-items:center;gap:6px;font-size:12.5px;color:var(--green);font-weight:500">
                <span class="hub-dot g"></span> Berhasil — {testMsg}
              </span>
            {:else if testing === 'fail'}
              <span style="display:flex;align-items:center;gap:6px;font-size:12.5px;color:var(--red);font-weight:500">
                <span class="hub-dot r"></span> {testMsg || 'Gagal — periksa URL/Key'}
              </span>
            {/if}
          </div>
        </div>
      </div>

      <!-- ── Webview URLs -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">URL Webview</div>
          <div class="d">URL yang dibuka saat memilih halaman webview. Berlaku setelah simpan.</div>
        </div>
        {#each [
          { k: 'urlGpt' as const,       n: 'ChatGPT',          ph: 'https://chat.openai.com' },
          { k: 'urlClaude' as const,    n: 'Claude',           ph: 'https://claude.ai' },
          { k: 'urlTranslate' as const, n: 'Google Translate', ph: 'https://translate.google.com' },
        ] as r}
          <div class="hub-s-row">
            <div class="label-wrap"><div class="l-name">{r.n}</div></div>
            <div class="field-wrap">
              <input class="hub-input" bind:value={settings[r.k]} placeholder={r.ph} />
            </div>
          </div>
        {/each}
      </div>
    {/if}

    {#if activeTab === 'privacy'}
      <!-- ── Privacy & Display -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">Privasi & Tampilan</div>
          <div class="d">Transparansi, selalu di atas, dan indikator mode senyap.</div>
        </div>

        <!-- Content protection toggle — hides all windows from screen capture -->
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Sembunyikan dari screen capture</div>
            <div class="l-desc">
              Jika <strong>aktif</strong>: jendela tidak terlihat di Zoom, Teams, OBS, screenshot OS, dll.<br>
              Jika <strong>nonaktif</strong>: jendela terlihat normal di screen sharing dan screenshot.
            </div>
          </div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-switch" data-on={String(settings.contentProtected)}
              onclick={() => applyContentProtected(!settings.contentProtected)}
              aria-label="Toggle sembunyikan dari screen capture"><i></i></button>
            <span style="font-size:12px;color:{settings.contentProtected ? 'var(--green)' : 'var(--amber)'}">
              {settings.contentProtected ? 'Aktif — tersembunyi dari screen capture' : 'Nonaktif — terlihat saat screen sharing'}
            </span>
          </div>
        </div>

        <!-- Stealth badge (visual indicator in title bar) -->
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Indikator Mode Senyap</div>
            <div class="l-desc">Tampilkan badge di title bar sebagai pengingat status proteksi.</div>
          </div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-switch" data-on={String(settings.stealth)}
              onclick={() => settings.stealth = !settings.stealth}
              aria-label="Toggle indikator mode senyap"><i></i></button>
            <span style="font-size:12px;color:var(--text-soft)">
              {settings.stealth ? 'Badge ditampilkan' : 'Badge disembunyikan'}
            </span>
          </div>
        </div>

        <!-- Opacity — live via CSS on hub-root -->
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Opasitas jendela</div>
            <div class="l-desc">Mengatur transparansi background — teks tetap tajam. Berlaku langsung. Disimpan saat klik "Simpan".</div>
          </div>
          <div class="field-wrap">
            <div class="hub-range-row">
              <input type="range" min="5" max="100" step="1" bind:value={settings.opacity} />
              <span class="v-val">{settings.opacity}%</span>
            </div>
          </div>
        </div>

        <!-- Always on top — live via Tauri JS API -->
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Selalu di atas</div>
            <div class="l-desc">Jendela tetap di depan aplikasi lain. Berlaku langsung.</div>
          </div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-switch" data-on={String(settings.alwaysOnTop)}
              onclick={() => applyAlwaysOnTop(!settings.alwaysOnTop)}
              aria-label="Toggle selalu di atas"><i></i></button>
            <span style="font-size:12px;color:var(--text-soft)">
              {settings.alwaysOnTop ? 'Aktif' : 'Nonaktif'}
            </span>
          </div>
        </div>

        <!-- Dark mode -->
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Tema gelap</div>
            <div class="l-desc">Berlaku langsung.</div>
          </div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-switch" data-on={String(settings.dark)}
              onclick={() => settings.dark = !settings.dark}
              aria-label="Toggle tema gelap"><i></i></button>
            <span style="font-size:12px;color:var(--text-soft)">
              {settings.dark ? 'Gelap' : 'Terang'}
            </span>
          </div>
        </div>

        <!-- Buka otomatis saat boot -->
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Buka otomatis saat boot</div>
            <div class="l-desc">Jalankan noscreen secara otomatis ketika komputer menyala.</div>
          </div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-switch" data-on={String(settings.autostart)}
              onclick={() => settings.autostart = !settings.autostart}
              aria-label="Toggle buka otomatis saat boot"><i></i></button>
            <span style="font-size:12px;color:var(--text-soft)">
              {settings.autostart ? 'Aktif' : 'Nonaktif'}
            </span>
          </div>
        </div>
      </div>
    {/if}

    {#if activeTab === 'shortcuts'}
      <!-- ── Keyboard Shortcuts -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">Pintasan Keyboard</div>
          <div class="d">Akses cepat dari mana saja.</div>
        </div>
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Tampilkan / sembunyikan jendela</div>
          </div>
          <div class="field-wrap" style="flex-direction:row">
            <HotkeyRecorder bind:value={settings.hotkey} />
          </div>
        </div>
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Ghost typing toggle</div>
          </div>
          <div class="field-wrap" style="flex-direction:row">
            <span class="hub-hint-tag">Ctrl+Alt+G</span>
          </div>
        </div>
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Copilot session toggle</div>
          </div>
          <div class="field-wrap" style="flex-direction:row">
            <span class="hub-hint-tag">Ctrl+Alt+L</span>
          </div>
        </div>
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Hide/show copilot card</div>
          </div>
          <div class="field-wrap" style="flex-direction:row">
            <span class="hub-hint-tag">Ctrl+Alt+H</span>
          </div>
        </div>
        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Force regenerate</div>
          </div>
          <div class="field-wrap" style="flex-direction:row">
            <span class="hub-hint-tag">Ctrl+Alt+R</span>
          </div>
        </div>
      </div>
    {/if}

    {#if activeTab === 'copilot'}
      <!-- ── Copilot -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">Copilot</div>
          <div class="d">Stealth assistant — capture audio system, transcribe, dan saran AI proaktif.</div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">STT Backend</div>
            <div class="l-desc">Engine speech-to-text.</div>
          </div>
          <div class="field-wrap">
            <select class="hub-input" bind:value={settings.sttBackend}>
              <option value="whisper-cloud">Whisper API (OpenAI)</option>
              <option value="whisper-local">Whisper Local (OpenAI-compatible)</option>
              <option value="windows-sr">Windows Speech Recognition</option>
            </select>
          </div>
        </div>

        {#if settings.sttBackend !== 'windows-sr'}
          <div class="hub-s-row">
            <div class="label-wrap">
              <div class="l-name">Bahasa STT</div>
              <div class="l-desc">Bahasa yang dikenali Whisper. <strong>Auto-detect</strong> cocok untuk percakapan campuran (misal Japan-English).</div>
            </div>
            <div class="field-wrap">
              <select class="hub-input" bind:value={settings.sttLanguage}>
                <option value="">Auto-detect</option>
                <option value="en">English</option>
                <option value="ja">Japanese (日本語)</option>
                <option value="id">Indonesian (Bahasa Indonesia)</option>
                <option value="zh">Chinese (中文)</option>
                <option value="ko">Korean (한국어)</option>
                <option value="fr">French (Français)</option>
                <option value="de">German (Deutsch)</option>
                <option value="es">Spanish (Español)</option>
              </select>
            </div>
          </div>
        {/if}

        {#if settings.sttBackend === 'whisper-local'}
          <div class="hub-s-row">
            <div class="label-wrap">
              <div class="l-name">Local STT URL</div>
              <div class="l-desc">Endpoint server Whisper lokal Anda (misal whisper.cpp, Ollama, dll.).</div>
            </div>
            <div class="field-wrap">
              <input class="hub-input" bind:value={settings.whisperLocalUrl} placeholder="http://localhost:8080" />
            </div>
          </div>
        {/if}

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Whisper model</div>
            <div class="l-desc">Nama model yang dipakai (Whisper API field "model").</div>
          </div>
          <div class="field-wrap">
            <input class="hub-input" bind:value={settings.whisperModel} placeholder="whisper-1" />
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Context window (default)</div>
            <div class="l-desc">Berapa detik transkrip yang dikirim ke AI tiap inference.</div>
          </div>
          <div class="field-wrap">
            <div class="hub-range-row">
              <input type="range" min="30" max="300" step="10" bind:value={settings.copilotContextS} />
              <span class="v-val">{settings.copilotContextS}s</span>
            </div>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Auto-dismiss card</div>
            <div class="l-desc">Card otomatis tertutup setelah N detik (0 = tidak pernah).</div>
          </div>
          <div class="field-wrap">
            <div class="hub-range-row">
              <input type="range" min="0" max="60" step="1" bind:value={settings.copilotAutoDismissS} />
              <span class="v-val">{settings.copilotAutoDismissS}s</span>
            </div>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Min interval antar saran</div>
            <div class="l-desc">Rate-limit: minimum detik antar LLM call. Hindari bill meledak.</div>
          </div>
          <div class="field-wrap">
            <div class="hub-range-row">
              <input type="range" min="2" max="15" step="1" bind:value={settings.copilotMinIntervalS} />
              <span class="v-val">{settings.copilotMinIntervalS}s</span>
            </div>
          </div>
        </div>
      </div>

      <!-- ── Custom presets -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">Preset Kustom</div>
          <div class="d">Buat preset Copilot sendiri untuk skenario yang tidak tercakup preset bawaan.</div>
        </div>

        {#if customPresets.length > 0}
          <div class="hub-s-row">
            <div class="label-wrap"><div class="l-name">Preset tersimpan</div></div>
            <div class="field-wrap">
              <div class="preset-list">
                {#each customPresets as p (p.id)}
                  <div class="preset-item">
                    <div class="preset-item-info">
                      <span class="preset-item-name">{p.name}</span>
                      <span class="preset-item-meta">{p.response_format} · {p.default_context_s}s</span>
                    </div>
                    <button class="hub-btn secondary sm" onclick={() => removePreset(p.id)}>Hapus</button>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">Nama preset</div>
          </div>
          <div class="field-wrap">
            <input class="hub-input" bind:value={presetName} placeholder="mis. Negosiasi Kontrak" />
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">System prompt</div>
            <div class="l-desc">Instruksi ke AI tentang cara merespons transkrip live.</div>
          </div>
          <div class="field-wrap">
            <textarea class="hub-input" style="min-height:80px;resize:vertical;font-family:inherit"
              bind:value={presetPrompt} placeholder="You are a live assistant for..."></textarea>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap"><div class="l-name">Format respons</div></div>
          <div class="field-wrap">
            <select class="hub-input" bind:value={presetFormat}>
              <option value="Bullets">Bullets</option>
              <option value="Headline">Headline</option>
              <option value="Code">Code</option>
            </select>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap"><div class="l-name">Context window default</div></div>
          <div class="field-wrap">
            <div class="hub-range-row">
              <input type="range" min="30" max="300" step="10" bind:value={presetContextS} />
              <span class="v-val">{presetContextS}s</span>
            </div>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap"></div>
          <div class="field-wrap" style="flex-direction:row;align-items:center;gap:10px">
            <button class="hub-btn" onclick={addPreset} disabled={presetSaving || !presetName.trim() || !presetPrompt.trim()}>
              {presetSaving ? 'Menyimpan…' : 'Tambah preset'}
            </button>
            {#if presetError}
              <span style="font-size:12px;color:var(--red)">{presetError}</span>
            {/if}
          </div>
        </div>
      </div>
    {/if}

      <!-- ── Save (always visible, independent of active tab) -->
      <div class="hub-s-save" style="display:flex;gap:10px;align-items:center">
        <button class="hub-btn accent" onclick={handleSave}>Simpan pengaturan</button>
        {#if saved}
          <span style="font-size:12.5px;color:var(--green);font-weight:500">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" style="display:inline;vertical-align:-2px"><path d="M5 13l4 4L19 7"/></svg>
            Tersimpan
          </span>
        {/if}
        {#if saveError}
          <span style="font-size:12px;color:var(--red)">{saveError}</span>
        {/if}
      </div>

    </div>
  </div>
</div>

<style>
  .preset-list { display: flex; flex-direction: column; gap: 6px; }
  .preset-item {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px;
  }
  .preset-item-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .preset-item-name { font-size: 12.5px; font-weight: 500; }
  .preset-item-meta { font-size: 11px; color: var(--text-soft); }
  .hub-btn.sm { padding: 4px 10px; font-size: 11.5px; flex-shrink: 0; }

  .hub-s-tabs {
    display: flex; gap: 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 20px;
  }
  .hub-s-tab {
    padding: 8px 12px;
    font-size: 13px; font-weight: 500;
    color: var(--text-soft);
    border: 0; background: transparent;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    cursor: pointer;
    transition: color .12s, border-color .12s;
  }
  .hub-s-tab:hover { color: var(--text); }
  .hub-s-tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  .hub-s-save {
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .model-picker { width: 100%; }
  .model-picker summary { cursor: pointer; }
  .model-options {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 240px;
    overflow-y: auto;
    margin-top: 6px;
  }
  .model-options button { text-align: left; cursor: pointer; overflow-wrap: anywhere; }
  .model-options button:hover, .model-options button.selected {
    background: var(--bg-hover, #eeeeee);
  }
</style>
