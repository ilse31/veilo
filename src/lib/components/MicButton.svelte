<script lang="ts">
  import { speechState, transcript, config } from '$lib/stores'
  import { injectText } from '$lib/tauri'
  import { get } from 'svelte/store'

  let recognition: any = null

  function startListening() {
    const SR = (window as any).SpeechRecognition ?? (window as any).webkitSpeechRecognition
    if (!SR) {
      speechState.set('error')
      setTimeout(() => speechState.set('idle'), 2000)
      return
    }

    recognition = new SR()
    recognition.continuous = false
    recognition.interimResults = false
    recognition.lang = get(config).language

    recognition.onstart = () => speechState.set('listening')

    recognition.onresult = async (event: any) => {
      const text = event.results[0][0].transcript
      transcript.set(text)
      speechState.set('processing')
      try {
        await injectText(text)
      } catch (e) {
        console.error('[noscreen] inject failed', e)
      } finally {
        speechState.set('idle')
      }
    }

    recognition.onerror = () => {
      speechState.set('error')
      setTimeout(() => speechState.set('idle'), 2000)
    }

    recognition.onend = () => {
      if (get(speechState) === 'listening') speechState.set('idle')
    }

    recognition.start()
  }

  function stopListening() {
    recognition?.stop()
    recognition = null
    speechState.set('idle')
  }

  function toggle() {
    if (get(speechState) === 'listening') stopListening()
    else startListening()
  }
</script>

<button
  onclick={toggle}
  class="w-10 h-10 rounded-full flex items-center justify-center transition-all duration-200
    focus:outline-none focus-visible:ring-2 focus-visible:ring-white
    {$speechState === 'listening'
      ? 'bg-green-500 animate-pulse shadow-lg shadow-green-500/50'
      : $speechState === 'error'
        ? 'bg-red-500'
        : $speechState === 'processing'
          ? 'bg-yellow-500'
          : 'bg-gray-700 hover:bg-gray-600'}"
  title={$speechState === 'listening' ? 'Stop recording' : 'Start voice input'}
>
  {#if $speechState === 'listening'}
    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 24 24">
      <rect x="6" y="6" width="12" height="12" rx="2" />
    </svg>
  {:else}
    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 24 24">
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-6 10a6 6 0 0 0 12 0h2a8 8 0 0 1-7 7.94V21h-2v-2.06A8 8 0 0 1 4 11H6z"/>
    </svg>
  {/if}
</button>
