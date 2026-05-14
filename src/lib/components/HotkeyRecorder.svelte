<script lang="ts">
  let { value = $bindable('Ctrl+Shift+Space'), onchange = (_hotkey: string) => {} }: {
    value?: string
    onchange?: (hotkey: string) => void
  } = $props()

  let recording = $state(false)
  let displayValue = $derived(recording ? 'Press keys...' : value)

  function startRecording() { recording = true }

  function handleKeydown(e: KeyboardEvent) {
    if (!recording) return
    e.preventDefault()
    const parts: string[] = []
    if (e.ctrlKey) parts.push('Ctrl')
    if (e.metaKey) parts.push('Meta')
    if (e.altKey) parts.push('Alt')
    if (e.shiftKey) parts.push('Shift')
    const key = e.key
    if (!['Control', 'Meta', 'Alt', 'Shift'].includes(key)) {
      parts.push(key === ' ' ? 'Space' : key.length === 1 ? key.toUpperCase() : key)
      value = parts.join('+')
      recording = false
      onchange(value)
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />
<div class="flex gap-2">
  <div class="flex-1 bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-white font-mono min-h-[38px] flex items-center">
    {displayValue}
  </div>
  <button onclick={startRecording}
    class="px-3 py-2 text-sm rounded-md transition-colors {recording ? 'bg-blue-600 text-white animate-pulse' : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}">
    {recording ? 'Recording...' : 'Record'}
  </button>
</div>
