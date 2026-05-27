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
  <div class="ob-card hub-card">
    <div class="ob-logo">N</div>
    <h2 class="ob-title">Halo! Siapa namamu?</h2>
    <p class="ob-sub">Kami akan menyapamu setiap kali membuka noscreen.</p>

    <form class="ob-form" onsubmit={handleSubmit}>
      <input
        class="hub-input ob-input"
        type="text"
        placeholder="Nama kamu…"
        bind:value={name}
        maxlength={60}
        autofocus
      />
      {#if error}
        <p class="ob-error">{error}</p>
      {/if}
      <button class="hub-btn ob-btn" type="submit" disabled={saving || !name.trim()}>
        {saving ? 'Menyimpan…' : 'Mulai →'}
      </button>
    </form>

    <p class="ob-note">Disimpan lokal di perangkat kamu — tidak dikirim ke mana pun.</p>
  </div>
</div>

<style>
  .ob-wrap {
    width: 100vw; height: 100vh;
    display: flex; align-items: center; justify-content: center;
    background: var(--bg);
    font-family: 'Onest', -apple-system, system-ui, sans-serif;
  }

  .ob-card {
    width: 380px;
    display: flex; flex-direction: column; align-items: center;
    gap: 10px;
    padding: 36px 32px 28px;
    text-align: center;
    box-shadow: 0 4px 24px rgba(0,0,0,.06), 0 1px 3px rgba(0,0,0,.05);
  }

  .ob-logo {
    width: 40px; height: 40px; border-radius: 11px;
    background: linear-gradient(135deg, var(--accent) 0%, #8a96f0 100%);
    color: #fff; font-weight: 700; font-size: 18px;
    display: flex; align-items: center; justify-content: center;
    box-shadow: 0 4px 12px rgba(94,106,210,.35);
    margin-bottom: 6px;
  }

  .ob-title {
    font-size: 20px; font-weight: 600; color: var(--text);
    letter-spacing: -0.02em; margin: 0;
  }

  .ob-sub {
    font-size: 13px; color: var(--text-soft);
    margin: 0 0 8px; line-height: 1.5;
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

  .ob-note {
    font-size: 11px; color: var(--text-faint);
    margin: 4px 0 0; line-height: 1.4;
  }
</style>
