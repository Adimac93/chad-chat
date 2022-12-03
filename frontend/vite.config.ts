import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vitejs.dev/config/
// https://stackoverflow.com/questions/71534594/change-vite-proxy-location-automatically-in-dev-vs-prod-builds
export default defineConfig({
  plugins: [svelte()],
  server: {
    proxy: {
      "/api": {
        target: "https://chad-chat-api.up.railway.app",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, '')
      },
      "/chat/websocket": {
        target: "wss://chad-chat-api.up.railway.app",
        secure: false,
        changeOrigin: true,
        ws: true,
      }
      
    }
  }
})
