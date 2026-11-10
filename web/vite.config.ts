import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const server = process.env.SOUNDBOARD_SERVER ?? 'http://127.0.0.1:7878'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    host: true,
    proxy: {
      '/api': server,
      '/ws': { target: server, ws: true },
    },
  },
})
