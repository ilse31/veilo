<script lang="ts">
  import { icons } from "./icons";

  import { settings } from '$lib/stores/settings.svelte'

  interface Props {
    apiUrl?: string;
    apiKey?: string;
    onNav?: (page: string) => void;
    onPrompt?: (provider: string, text: string) => void;
  }
  let { apiUrl = "", apiKey = "", onNav, onPrompt }: Props = $props();

  let draft = $state("");
  let activeProvider = $state("native");

  const providers = [
    { id: "native", label: "AI Lokal", color: "#5e6ad2" },
    { id: "gpt", label: "ChatGPT", color: "#19c37d" },
    { id: "claude", label: "Claude", color: "#d97757" },
    { id: "translate", label: "Terjemah", color: "#4285f4" },
  ];


  const hour = new Date().getHours();
  const greet =
    hour < 11
      ? "Selamat pagi"
      : hour < 15
        ? "Selamat siang"
        : hour < 19
          ? "Selamat sore"
          : "Selamat malam";

  function send() {
    if (!draft.trim()) return;
    onPrompt?.(activeProvider, draft.trim());
    draft = "";
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="hub-page-scroll">
  <div class="hub-page-pad">
    <h2 class="hub-greeting">{greet}, {settings.profileName ?? 'Kamu'}.</h2>
    <p class="hub-greeting-sub">
      Mau ngapain hari ini? Pilih asisten lalu mulai ngobrol.
    </p>

    <!-- Quick prompt card -->
    <div class="hub-qp-card">
      <textarea
        class="hub-qp-textarea"
        placeholder="Tulis pertanyaan atau perintah… contoh: 'jelasin diff antara useMemo vs useCallback'"
        bind:value={draft}
        onkeydown={handleKeydown}
        rows={3}
      ></textarea>
      <div class="hub-qp-bottom">
        {#each providers as p}
          <button
            class="hub-provider-pill {activeProvider === p.id ? 'active' : ''}"
            onclick={() => (activeProvider = p.id)}
          >
            <span
              class="px"
              style="background:{p.color};width:14px;height:14px;border-radius:4px;flex-shrink:0;display:inline-block"
            ></span>
            {p.label}
          </button>
        {/each}
        <button
          class="hub-qp-send"
          disabled={!draft.trim()}
          onclick={send}
          title="Kirim (⌘↵)"
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d={icons.arrowUp} />
          </svg>
        </button>
      </div>
      <div class="hub-qp-hint">
        <span
          class="chip"
          onclick={() =>
            (draft = "Jelaskan kode ini langkah demi langkah:\n\n")}
          role="button"
          tabindex="0"
          onkeydown={() => {}}>Jelaskan kode</span
        >
        <span
          class="chip"
          onclick={() => (draft = "Terjemahkan ke Bahasa Inggris formal:\n\n")}
          role="button"
          tabindex="0"
          onkeydown={() => {}}>Terjemahkan</span
        >
        <span
          class="chip"
          onclick={() =>
            (draft = "Ringkas dokumen berikut menjadi 5 poin utama:\n\n")}
          role="button"
          tabindex="0"
          onkeydown={() => {}}>Ringkas dokumen</span
        >
        <span
          class="chip"
          onclick={() =>
            (draft = "Buatkan email balasan profesional untuk:\n\n")}
          role="button"
          tabindex="0"
          onkeydown={() => {}}>Tulis email</span
        >
      </div>
    </div>
  </div>
</div>
