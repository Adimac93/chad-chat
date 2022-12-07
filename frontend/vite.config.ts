import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
// https://stackoverflow.com/questions/71534594/change-vite-proxy-location-automatically-in-dev-vs-prod-builds
export default defineConfig({
    plugins: [svelte()],
    server: {
        proxy: {
            "/api": {
                target: "http://127.0.0.1:3000",
                changeOrigin: true,
                rewrite: (path) => path.replace(/^\/api/, ""),
            },
            "/api/chat/websocket": {
                target: "ws://127.0.0.1:3000",
                changeOrigin: true,
                ws: true,
            },
        },
    },
});
