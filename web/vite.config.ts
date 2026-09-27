import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  // The render worker is constructed as a module worker, so build it as one.
  // Vite's default is an IIFE, which happens to run inside a module too — but
  // only by accident, and the accident would stop covering us the moment the
  // worker wanted a static import.
  worker: { format: 'es' },
  // `css: true` so a `?raw` import of the stylesheet returns the stylesheet.
  // With vitest's default (off) it returns an empty string, which is not an
  // error anywhere — it is how the audit in theme.test.ts passed for its whole
  // life without reading a single rule.
  test: { css: true },
  server: {
    port: 5173,
    // The Rust API owns /api; everything else is the SPA.
    proxy: {
      '/api': {
        target: process.env.RUSTYCV_API ?? 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
    },
  },
})
