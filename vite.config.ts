import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed dev port and must see Rust errors in the terminal.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  // dotLottie files are fetched at run time, and the CSP (`default-src 'self'`) blocks a fetch of a data: URL, so they
  // are never inlined however small they are.
  build: { assetsInlineLimit: (file) => (file.endsWith(".lottie") ? false : undefined) },
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/crates/**"] } },
});
