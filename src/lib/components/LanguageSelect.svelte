<script lang="ts">
  let { value = $bindable('id-ID'), onchange = (_lang: string) => {} }: {
    value?: string
    onchange?: (lang: string) => void
  } = $props()

  const languages = [
    { code: 'id-ID', label: 'Indonesian (id-ID)' },
    { code: 'en-US', label: 'English US (en-US)' },
    { code: 'en-GB', label: 'English UK (en-GB)' },
    { code: 'ja-JP', label: 'Japanese (ja-JP)' },
    { code: 'zh-CN', label: 'Chinese Simplified (zh-CN)' },
    { code: 'ko-KR', label: 'Korean (ko-KR)' },
  ]
</script>

<select bind:value onchange={() => onchange(value)}
  class="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500">
  {#each languages as lang}
    <option value={lang.code}>{lang.label}</option>
  {/each}
</select>
