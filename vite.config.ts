import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import { fileViewerRenderers } from '@file-viewer/vite-plugin';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss(), fileViewerRenderers({copyAssets: true})],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  build: {
    // The packaged WebView loads this local entry directly from the APK.
    // Forced initial-module splitting breaks execution order on Android.
    chunkSizeWarningLimit: 5000,
  },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 15173,
    strictPort: true,
    host: host ? "0.0.0.0" : false,
    hmr: host
      ? {
          host,
          port: 15174,
          protocol: "ws",
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
