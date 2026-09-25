import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
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
