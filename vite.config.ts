import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

// @tauri-apps/cli sets this when targeting a physical mobile device / LAN host.
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],

  // Tauri expects a fixed port and clean output for its CLI to attach to.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // Don't let Vite watch the Rust side.
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    // TLiquid is macOS-first (WKWebView). Safari 15 is a safe modern baseline.
    target: "safari15",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // Single window (the menu-bar panel) → a single `index.html` entry. Settings
    // and result are views inside it, not separate windows. See src-tauri/src/windows.rs.
    rollupOptions: {
      input: { main: resolve(__dirname, "index.html") },
    },
  },
});
