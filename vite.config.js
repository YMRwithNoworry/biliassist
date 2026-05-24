import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    hmr: {
      port: 1421
    },
    watch: {
      usePolling: true,
      interval: 2000,
      ignored: [
        '**/node_modules/**',
        '**/dist/**',
        '**/src-tauri/**',
        '**/target/**',
        '**/prompts/**',
        '**/杂物/**',
        '**/.wrangler/**',
        '**/tauri-app/**',
        '**/docs/**',
        '**/*.cjs',
        '**/*.bat',
        '**/*.sh',
        '**/*.md',
        '**/*.lock'
      ]
    }
  }
}))