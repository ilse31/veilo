import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { resolve } from 'path'

// Separate vitest config that uses the plain svelte plugin (not sveltekit())
// This avoids SvelteKit's SSR resolve condition overriding Svelte's browser build.
export default defineConfig({
  plugins: [
    svelte({ hot: false }),
  ],
  resolve: {
    conditions: ['browser'],
  },
  test: {
    environment: 'jsdom',
    alias: {
      '$lib': resolve(__dirname, './src/lib'),
      '$app/environment': resolve(__dirname, './src/lib/__mocks__/app-environment.ts'),
    },
  },
})
