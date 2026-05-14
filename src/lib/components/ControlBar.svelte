<script lang="ts">
  import MicButton from './MicButton.svelte'
  import { openSettings, toggleVisibility } from '$lib/tauri'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  const appWindow = getCurrentWindow()

  async function startDrag(e: MouseEvent) {
    e.preventDefault()
    await appWindow.startDragging()
  }
</script>

<div class="flex items-center gap-2 px-3 h-12 bg-gray-900/90 backdrop-blur-sm rounded-full border border-gray-700/50 select-none" style="width: 280px;">
  <button onmousedown={startDrag} class="cursor-grab active:cursor-grabbing text-gray-500 hover:text-gray-300 transition-colors p-1 rounded" title="Drag to move" aria-label="Drag handle">
    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
      <circle cx="9" cy="7" r="1.5"/><circle cx="15" cy="7" r="1.5"/>
      <circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/>
      <circle cx="9" cy="17" r="1.5"/><circle cx="15" cy="17" r="1.5"/>
    </svg>
  </button>
  <MicButton />
  <div class="flex-1"></div>
  <button onclick={() => openSettings()} class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-white hover:bg-gray-700 transition-all" title="Settings" aria-label="Open settings">
    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
      <circle cx="12" cy="12" r="3"/>
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
    </svg>
  </button>
  <button onclick={() => toggleVisibility()} class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-white hover:bg-red-600 transition-all" title="Hide (hotkey to show again)" aria-label="Hide overlay">
    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
      <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
    </svg>
  </button>
</div>
