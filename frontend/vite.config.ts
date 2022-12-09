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
                secure: false,
                ws: true,
                configure: (proxy, _options) => {
                    proxy.on("error", (err, _req, _res) => {
                        console.log("proxy error", err);
                    });
                    proxy.on("proxyReq", (proxyReq, req, _res) => {
                        console.log("Sending Request to the Target:", req.method, req.url);
                    });
                    proxy.on("proxyRes", (proxyRes, req, _res) => {
                        console.log(
                            "Received Response from the Target:",
                            proxyRes.statusCode,
                            req.url
                        );
                    });
                },
            },
        },
    },
});
