import { defineConfig } from "@lovable.dev/vite-tanstack-config";

export default defineConfig({
  tanstackStart: {
    // Disable SSR - run as pure SPA
    server: { 
      handler: () => new Response(null, { status: 404 })
    },
  },
  vite: {
    define: {
      "import.meta.env.VITE_TAURI": JSON.stringify(false),
    },
  },
});