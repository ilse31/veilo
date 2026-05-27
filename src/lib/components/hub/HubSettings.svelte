<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { saveConfig, setAllContentProtected } from '$lib/tauri'
  import { settings } from '$lib/stores/settings.svelte'
  import { setServiceUrl } from './webviewManager'

  let testing = $state<null | 'pending' | 'ok' | 'fail'>(null)
  let testMsg = $state('')
  let saved = $state(false)
  let saveError = $state('')
  let showKey = $state(false)

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

  // ── Real API test — sends a minimal chat completion request
  async function testConnection() {
    if (!settings.apiUrl) { testing = 'fail'; testMsg = 'URL tidak boleh kosong'; return }
    testing = 'pending'
    testMsg = ''
    try {
      const base = settings.apiUrl.replace(/\/$/, '')
      const res = await fetch(`${base}/v1/models`, {
        headers: settings.apiKey ? { 'Authorization': `Bearer ${settings.apiKey}` } : {},
      })
      if (res.ok) {
        testing = 'ok'
        testMsg = `${res.status} OK`
      } else {
        testing = 'fail'
        testMsg = `HTTP ${res.status}`
      }
    } catch (e) {
      testing = 'fail'
      testMsg = e instanceof TypeError ? 'Tidak dapat terhubung (CORS/jaringan)' : String(e)
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

      // Persist to disk (what Config supports)
      await saveConfig({
        site:       settings.urlClaude,
        hotkey:     'Ctrl+Shift+Space',
        language:   'id-ID',
        opacity:    settings.opacity / 100,
        autostart:  false,
        position:   null,
      })

      saved = true
      setTimeout(() => saved = false, 2000)
    } catch (e) {
      saveError = String(e)
    }
  }

  const shortcuts = [
    ['Tampilkan / sembunyikan jendela', 'Ctrl+Shift+Space'],
    ['Pindah ke halaman berikutnya',    'Ctrl+]'],
  ]
</script>

<div class="hub-page-scroll">
  <div class="hub-page-pad">
    <h2 class="hub-greeting" style="font-size:19px;margin-bottom:24px">Pengaturan</h2>

    <div class="hub-settings-grid">

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
            <input class="hub-input" bind:value={settings.apiUrl} placeholder="https://api.openai.com" />
            <div style="display:flex;gap:6px;flex-wrap:wrap">
              {#each [['https://api.openai.com','OpenAI'],['http://localhost:11434','Ollama'],['https://api.groq.com/openai','Groq']] as [u, l]}
                <span class="hub-hint-tag" onclick={() => settings.apiUrl = u} role="button" tabindex="0" onkeydown={() => {}} style="cursor:default">{l}</span>
              {/each}
            </div>
          </div>
        </div>

        <div class="hub-s-row">
          <div class="label-wrap">
            <div class="l-name">API Key</div>
            <div class="l-desc">Disimpan di memori sesi. Tidak dikirim ke server lain.</div>
          </div>
          <div class="field-wrap">
            <div class="hub-key-input">
              <input class="hub-input"
                     type={showKey ? 'text' : 'password'}
                     bind:value={settings.apiKey}
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
            <input class="hub-input" bind:value={settings.model} placeholder="gpt-4o-mini" />
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
            <div class="l-desc">Berlaku langsung. Disimpan saat klik "Simpan".</div>
          </div>
          <div class="field-wrap">
            <div class="hub-range-row">
              <input type="range" min="20" max="100" step="1" bind:value={settings.opacity} />
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
      </div>

      <!-- ── Keyboard Shortcuts -->
      <div class="hub-s-section">
        <div class="head">
          <div class="t">Pintasan Keyboard</div>
          <div class="d">Akses cepat dari mana saja.</div>
        </div>
        {#each shortcuts as [n, k]}
          <div class="hub-s-row">
            <div class="label-wrap"><div class="l-name">{n}</div></div>
            <div class="field-wrap" style="flex-direction:row">
              <span class="hub-hint-tag">{k}</span>
            </div>
          </div>
        {/each}
      </div>

      <!-- ── Save -->
      <div style="display:flex;gap:10px;align-items:center">
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
