// Dedicated Vite config for the Tauri desktop bundle.
//
// The default config (vite.config.ts) wraps Lovable's TanStack Start pipeline,
// which produces an SSR-style build (client assets + server.js, no index.html).
// Tauri needs a plain static SPA, so this config builds one independently into
// dist-spa/ without touching the web flow.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import tsconfigPaths from "vite-tsconfig-paths";
import { tanstackRouter } from "@tanstack/router-plugin/vite";

export default defineConfig({
  plugins: [tanstackRouter(), react(), tailwindcss(), tsconfigPaths()],
  define: {
    "import.meta.env.VITE_TAURI": JSON.stringify(true),
  },
  build: {
    outDir: "dist-spa",
    emptyOutDir: true,
  },
});
