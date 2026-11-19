<script lang="ts">
  import { onMount } from 'svelte'
  import { getVersion } from '@tauri-apps/api/app'
  import { icons } from './icons'
  import { settings } from '$lib/stores/settings.svelte'

  interface Props {
    current: string
    onNav: (page: string) => void
    apiOk?: boolean
  }
  let { current, onNav, apiOk = false }: Props = $props()

  let appVersion = $state('')
  $effect(() => {
    getVersion().then(v => appVersion = v)
  })

  interface NavItem {
    id: string
    label: string
    kbd: string
    icon?: keyof typeof icons | 'copilot'
    src?: 'gpt' | 'claude' | 'translate' | 'native'
    badge?: string
  }
  interface NavGroup { label: string; items: NavItem[] }

  const navGroups: NavGroup[] = [
    {
      label: 'Umum',
      items: [
        { id: 'dashboard', label: 'Dasbor',  icon: 'dashboard', kbd: '⌘1' },
        { id: 'copilot',   label: 'Copilot', icon: 'copilot',   kbd: '⌘6', badge: 'Beta' },
      ],
    },
    {
      label: 'Asisten',
      items: [
        { id: 'gpt',       label: 'ChatGPT',    kbd: '⌘2', src: 'gpt' },
        { id: 'claude',    label: 'Claude',     kbd: '⌘3', src: 'claude' },
        { id: 'translate', label: 'Terjemahan', kbd: '⌘4', src: 'translate' },
        { id: 'chat',      label: 'Obrolan AI', kbd: '⌘5', src: 'native' },
      ],
    },
    {
      label: 'Alat',
      items: [
        { id: 'markdown', label: 'Baca Markdown', icon: 'fileText', kbd: '⌘7' },
      ],
    },
    {
      label: 'Sistem',
      items: [
        { id: 'settings', label: 'Pengaturan', icon: 'settings', kbd: '⌘,' },
      ],
    },
  ]

  const allItems = navGroups.flatMap(g => g.items)

  // ── ⌘K quick-switch: filter nav, Enter picks first match ──
  let query = $state('')
  let searchEl = $state<HTMLInputElement | null>(null)

  const matches = $derived(
    query.trim()
      ? allItems.filter(i => i.label.toLowerCase().includes(query.trim().toLowerCase()))
      : allItems,
  )

  onMount(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        searchEl?.focus()
        searchEl?.select()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  })

  function onSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && matches.length) {
      onNav(matches[0].id)
      query = ''
      searchEl?.blur()
    } else if (e.key === 'Escape') {
      query = ''
      searchEl?.blur()
    }
  }
</script>

<aside class="hub-sidebar">
  <div class="titlebar-spacer" data-tauri-drag-region></div>

  <div class="hub-brand">
    <img class="hub-brand-logo" src="/veilo.svg" alt="" width="24" height="24" />
    <div class="hub-brand-name">Veilo</div>
    <div class="hub-brand-sub">{appVersion ? `v${appVersion}` : '...'}</div>
  </div>

  <!-- Quick switch -->
  <div class="hub-search-wrap">
    <div class="hub-search" class:focused={query}>
      <svg class="hub-search-ic" width="13" height="13" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/>
      </svg>
      <input
        bind:this={searchEl}
        bind:value={query}
        onkeydown={onSearchKeydown}
        class="hub-search-input"
        type="text"
        placeholder="Cari…"
        spellcheck="false"
      />
      <span class="kbd">⌘K</span>
    </div>
  </div>

  {#if query.trim()}
    <!-- Hasil quick-switch: daftar flat, tanpa grup -->
    <div class="hub-nav-list hub-search-results">
      {#each matches as item (item.id)}
        <div
          class="hub-nav-item"
          onclick={() => { onNav(item.id); query = '' }}
          role="button" tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && (onNav(item.id), query = '')}
        >
          <span class="nav-ic">
            {#if item.src}
              <span class="src-dot {item.src}"></span>
            {:else}
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
                {#if item.icon === 'copilot'}
                  <circle cx="12" cy="12" r="3" />
                  <path d="M12 1v6m0 10v6m11-11h-6M7 12H1" />
                {:else if item.icon}
                  {@html icons[item.icon]}
                {/if}
              </svg>
            {/if}
          </span>
          <span>{item.label}</span>
          <span class="kbd">{item.kbd}</span>
        </div>
      {:else}
        <div class="hub-search-empty">Tidak ada hasil</div>
      {/each}
    </div>
  {:else}
    {#each navGroups as group (group.label)}
      <div class="hub-section-label">{group.label}</div>
      <div class="hub-nav-list">
        {#each group.items as item (item.id)}
          <div
            class="hub-nav-item {current === item.id ? 'active' : ''}"
            onclick={() => onNav(item.id)}
            role="button" tabindex="0"
            onkeydown={(e) => e.key === 'Enter' && onNav(item.id)}
          >
            <span class="nav-ic">
              {#if item.src}
                <span class="src-dot {item.src}"></span>
              {:else}
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
                     stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
                  {#if item.icon === 'copilot'}
                    <circle cx="12" cy="12" r="3" />
                    <path d="M12 1v6m0 10v6m11-11h-6M7 12H1" />
                  {:else if item.icon}
                    {@html icons[item.icon]}
                  {/if}
                </svg>
              {/if}
            </span>
            <span>{item.label}</span>
            {#if item.badge}
              <span class="nav-badge">{item.badge}</span>
            {/if}
            <span class="kbd">{item.kbd}</span>
          </div>
        {/each}
      </div>
    {/each}
  {/if}

  <div class="spacer"></div>

  <!-- User card -->
  <div class="hub-user-card">
    <div class="av">{(settings.profileName ?? 'A')[0].toUpperCase()}</div>
    <div class="info">
      <div class="name">{settings.profileName ?? 'Akun Lokal'}</div>
      <div class="plan">{apiOk ? 'API tersambung' : 'API belum diatur'}</div>
    </div>
  </div>
</aside>

<style>
  .titlebar-spacer {
    height: 36px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: var(--bg-side);
    -webkit-app-region: drag;
  }
  .spacer { flex: 1; }

  .hub-search-wrap { padding: 0 10px 10px; }
  .hub-search {
    display: flex; align-items: center; gap: 7px;
    padding: 6px 9px; border-radius: 7px;
    background: var(--bg-elev); border: 1px solid var(--border);
    color: var(--text-faint);
    transition: border-color .1s;
  }
  .hub-search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .hub-search-ic { flex-shrink: 0; }
  .hub-search-input {
    flex: 1; min-width: 0;
    background: transparent; border: 0; outline: none;
    font-family: inherit; font-size: 12.5px; color: var(--text);
  }
  .hub-search-input::placeholder { color: var(--text-faint); }
  .hub-search .kbd {
    font-family: var(--mono); font-size: 10px;
    color: var(--text-faint); padding: 1px 5px;
    border: 1px solid var(--border); border-radius: 4px;
    background: var(--bg);
  }
  .hub-search-empty {
    padding: 8px 12px; font-size: 12px; color: var(--text-faint);
  }
  .hub-search-results { flex: 1; overflow: auto; }

  .nav-ic {
    width: 16px; height: 16px; flex-shrink: 0;
    display: flex; align-items: center; justify-content: center;
    opacity: .85;
  }
  .hub-nav-item.active .nav-ic { opacity: 1; color: var(--accent); }
  .nav-badge {
    font-size: 9.5px; font-weight: 600; letter-spacing: .04em;
    padding: 1.5px 6px; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent);
  }
</style>
