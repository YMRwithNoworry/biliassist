import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: false,
    hmr: {
      port: 1421
    }
  }
}))
