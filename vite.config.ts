import { defineConfig } from 'vite'
import { sveltekit } from '@sveltejs/kit/vite'
import { resolve } from 'path'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [sveltekit()],

  resolve: {
    alias: { '$lib': resolve(__dirname, './src/lib') },
  },

  build: {
    rollupOptions: {
      input: {
        'control-bar': resolve(__dirname, 'control-bar.html'),
        'settings': resolve(__dirname, 'settings.html'),
      },
    },
  },

  // Vite options tailored for Tauri development
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },

  test: {
    environment: 'jsdom',
    alias: {
      '$lib': resolve(__dirname, './src/lib'),
    },
  },
})
