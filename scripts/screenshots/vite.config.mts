import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const here = fileURLToPath(new URL(".", import.meta.url));

/**
 * Builds the screenshot harness only. The real app is bundled by the root
 * `vite.config.ts`; here the Tauri IPC modules are swapped for local mocks so
 * the screenshots can be rendered in a plain browser.
 */
export default defineConfig({
  root: here,
  base: "./",
  plugins: [react()],
  resolve: {
    alias: {
      "@tauri-apps/api/core": `${here}mock-core.ts`,
      "@tauri-apps/api/event": `${here}mock-event.ts`,
    },
  },
  build: {
    outDir: "/tmp/sf-shots-dist",
    emptyOutDir: true,
  },
});
