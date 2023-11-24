import { defineConfig, loadEnv } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, process.cwd(), '')
    const backendURL = JSON.stringify(env.BACKEND_URL) ?? "http://127.0.0.1:3000"
    console.info(`Backend at ${backendURL}`)
    return {
        plugins: [svelte()],
        server: {
            // https://github.com/expressjs/cors#configuration-options
            cors: {
                origin: backendURL,
            },
            proxy: {
                "/api": {
                    target: backendURL,
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
        }
    }
});