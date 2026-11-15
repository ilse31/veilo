<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import Settings from '$lib/components/Settings.svelte'
  import Hub from '$lib/components/Hub.svelte'
  import AnswerCard from '$lib/components/copilot/AnswerCard.svelte'

  const label = getCurrentWindow().label
</script>

{#if label === 'settings'}
  <Settings />

{:else if label === 'hub' || label === 'ai-view'}
  <Hub />

{:else if label === 'copilot-card'}
  <AnswerCard />

{:else}
  <!-- fallback: nothing to render -->
{/if}
