<script lang="ts">
  import { setProfileName } from '$lib/tauri'

  interface Props {
    onComplete: (name: string) => void
  }
  let { onComplete }: Props = $props()

  let name    = $state('')
  let error   = $state('')
  let saving  = $state(false)

  async function handleSubmit(e: Event) {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed) { error = 'Nama tidak boleh kosong'; return }
    saving = true
    error  = ''
    try {
      await setProfileName(trimmed)
      onComplete(trimmed)
    } catch (err) {
      error  = String(err)
      saving = false
    }
  }
</script>

<div class="ob-wrap">
  <div class="ob-card">
    <img class="ob-logo" src="/veilo.svg" alt="Veilo" width="56" height="56" />
    <h2 class="ob-title">Selamat datang di Veilo</h2>
    <p class="ob-sub">Teman AI yang tenang — terlihat olehmu, tersembunyi dari screen capture. Siapa namamu?</p>

    <form class="ob-form" onsubmit={handleSubmit}>
      <input
        class="hub-input ob-input"
        type="text"
        placeholder="Nama kamu…"
        bind:value={name}
        maxlength={60}
        autofocus
      />
      <p class="ob-hint">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2l8 3.5v5.2c0 5-3.4 9.6-8 11.3-4.6-1.7-8-6.3-8-11.3V5.5L12 2z"/>
        </svg>
        Disimpan lokal di perangkat kamu
      </p>
      {#if error}
        <p class="ob-error">{error}</p>
      {/if}
      <button class="hub-btn ob-btn" type="submit" disabled={saving || !name.trim()}>
        {saving ? 'Menyimpan…' : 'Mulai →'}
      </button>
    </form>
  </div>

  <p class="ob-foot">Tekan <kbd>Ctrl+Shift+Space</kbd> kapan saja untuk membuka Veilo</p>
</div>

<style>
  .ob-wrap {
    width: 100vw; height: 100vh;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 18px;
    background: var(--bg);
    font-family: 'Onest', -apple-system, system-ui, sans-serif;
  }

  .ob-card {
    width: 400px;
    display: flex; flex-direction: column; align-items: center;
    gap: 10px;
    padding: 40px 36px 32px;
    text-align: center;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 8px 32px rgba(0,0,0,.18);
  }

  .ob-logo {
    width: 56px; height: 56px; border-radius: 14px;
    box-shadow: 0 6px 18px rgba(91,100,214,.35);
    margin-bottom: 8px;
  }

  .ob-title {
    font-size: 20px; font-weight: 600; color: var(--text);
    letter-spacing: -0.02em; margin: 0;
  }

  .ob-sub {
    font-size: 13px; color: var(--text-soft);
    margin: 0 0 10px; line-height: 1.55;
  }

  .ob-form {
    width: 100%; display: flex; flex-direction: column; gap: 10px;
    margin-top: 4px;
  }

  .ob-input {
    text-align: center;
    font-size: 15px;
    padding: 10px 14px;
  }

  .ob-hint {
    display: flex; align-items: center; justify-content: center; gap: 5px;
    font-size: 11.5px; color: var(--text-faint); margin: 0;
  }

  .ob-error {
    font-size: 12px; color: var(--red); margin: 0;
  }

  .ob-btn {
    width: 100%; justify-content: center;
    padding: 9px 14px; font-size: 14px;
    background: var(--accent); color: #fff; border: 0;
    border-radius: 8px;
    transition: opacity .1s;
  }
  .ob-btn:disabled { opacity: 0.5; cursor: default; }

  .ob-foot {
    font-size: 11.5px; color: var(--text-faint); margin: 0;
  }
  .ob-foot kbd {
    font-family: var(--mono); font-size: 10.5px;
    padding: 2px 6px; border-radius: 4px;
    background: var(--bg-elev); border: 1px solid var(--border);
    color: var(--text-soft);
  }
</style>
