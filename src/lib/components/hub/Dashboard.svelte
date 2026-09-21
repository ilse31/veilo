<script lang="ts">
  import { onMount } from "svelte";
  import { icons } from "./icons";

  import { settings } from '$lib/stores/settings.svelte'
  import { listConversations, getUsageStats, type ConvRow, type UsageStats } from '$lib/tauri'
  import {
    formatDelta, formatLatency, formatTokens, hostOf, relativeTime, todayStart,
  } from '$lib/dashboardFormat'

  interface Props {
    apiUrl?: string;
    apiKey?: string;
    onNav?: (page: string) => void;
    onPrompt?: (provider: string, text: string) => void;
    onOpenConversation?: (id: number) => void;
  }
  let { apiUrl = "", apiKey = "", onNav, onPrompt, onOpenConversation }: Props = $props();

  let draft = $state("");
  let activeProvider = $state("native");

  const providers = [
    { id: "native", label: "AI Lokal", color: "#5e6ad2" },
    { id: "gpt", label: "ChatGPT", color: "#19c37d" },
    { id: "claude", label: "Claude", color: "#d97757" },
    { id: "translate", label: "Terjemah", color: "#4285f4" },
  ];


  const RECENT_LIMIT = 6;
  const BADGE_COLORS = ["#5e6ad2", "#19c37d", "#d97757", "#4285f4"];

  let recent = $state<ConvRow[]>([]);
  let stats = $state<UsageStats | null>(null);

  onMount(async () => {
    const [convs, usage] = await Promise.allSettled([
      listConversations(),
      getUsageStats(todayStart()),
    ]);
    if (convs.status === "fulfilled") recent = convs.value.slice(0, RECENT_LIMIT);
    if (usage.status === "fulfilled") stats = usage.value;
  });

  const badgeOf = (title: string) => (title.trim()[0] ?? "?").toUpperCase();
  const badgeColor = (id: number) => BADGE_COLORS[id % BADGE_COLORS.length];

  type Status = { name: string; host: string; label: string; tone: "ok" | "warn" };
  const services = $derived<Status[]>([
    {
      name: "AI Lokal",
      host: apiUrl ? hostOf(apiUrl) : "belum diatur",
      label: apiUrl ? "Aktif" : "Belum diatur",
      tone: apiUrl ? "ok" : "warn",
    },
    { name: "ChatGPT (webview)", host: hostOf(settings.urlGpt), label: "Siap", tone: "ok" },
    { name: "Claude (webview)", host: hostOf(settings.urlClaude), label: "Siap", tone: "ok" },
    { name: "Google Translate", host: hostOf(settings.urlTranslate), label: "Siap", tone: "ok" },
    {
      name: "Mode Senyap",
      host: "screen-share filter",
      label: settings.contentProtected ? "Aktif" : "Standby",
      tone: settings.contentProtected ? "ok" : "warn",
    },
  ]);

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

    <div class="hub-grid-2">
      <section>
        <h3 class="hub-section-title">
          Riwayat Terbaru
          <button class="more" onclick={() => onNav?.("chat")}>Lihat semua →</button>
        </h3>
        <div class="hub-card hub-recent-list">
          {#each recent as c (c.id)}
            <button class="hub-recent-item" onclick={() => onOpenConversation?.(c.id)}>
              <span class="src-icon" style="background:{badgeColor(c.id)}">{badgeOf(c.title)}</span>
              <span class="title-line">{c.title}</span>
              <span class="meta">{relativeTime(c.created_at)}</span>
            </button>
          {:else}
            <p class="hub-empty">Belum ada obrolan. Mulai dengan menulis pertanyaan di atas.</p>
          {/each}
        </div>
      </section>

      <section>
        <h3 class="hub-section-title">Status Sistem</h3>
        <div class="hub-kpi-row">
          <div class="hub-kpi">
            <div class="k">Pesan hari ini</div>
            <div class="v">{stats?.messages_today ?? "—"}</div>
            <div class="d">{stats ? formatDelta(stats.messages_today, stats.messages_yesterday) : ""}</div>
          </div>
          <div class="hub-kpi">
            <div class="k">Token terpakai</div>
            <div class="v">{stats ? `${stats.tokens_estimated ? "≈" : ""}${formatTokens(stats.tokens_today)}` : "—"}</div>
            <div class="d">{stats?.tokens_estimated ? "hari ini (perkiraan)" : "hari ini"}</div>
          </div>
          <div class="hub-kpi">
            <div class="k">Latensi rata-rata</div>
            <div class="v">{formatLatency(stats?.avg_latency_ms ?? null)}</div>
            <div class="d">7 hari terakhir</div>
          </div>
        </div>
        <div class="hub-card hub-status-list">
          {#each services as svc (svc.name)}
            <div class="hub-status-row">
              <span class="dot {svc.tone}"></span>
              <span class="name">{svc.name}</span>
              <span class="val">{svc.host}</span>
              <span class="badge" class:warn={svc.tone === "warn"}>{svc.label}</span>
            </div>
          {/each}
        </div>
      </section>
    </div>
  </div>
</div>
