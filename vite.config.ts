import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      // Mismo alias que `paths.$lib/*` de tsconfig.json, para que el
      // empaquetador (Rollup) resuelva las importaciones `$lib/...`.
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // no recargar por cambios en el backend Rust
      ignored: ["**/crates/**", "**/target/**"],
    },
  },
});
