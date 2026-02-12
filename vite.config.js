import { defineConfig } from 'vite'
import { resolve } from 'path'

export default defineConfig({
  clearScreen: false,
  root: 'src',
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'src/index.html'),
        overlay: resolve(__dirname, 'src/overlay.html'),
      },
    },
  },
})
