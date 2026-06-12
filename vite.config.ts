import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const host = process.env.TAURI_DEV_HOST
const isDebug = !!process.env.TAURI_ENV_DEBUG
const isWindows = process.env.TAURI_ENV_PLATFORM === 'windows'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: isWindows ? 'chrome105' : 'safari13',
    minify: !isDebug,
    sourcemap: isDebug,
  },
})
