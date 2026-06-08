import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  // No se fuerza `runes: true` globalmente: Svelte 5 detecta runes por archivo
  // (todos nuestros componentes los usan) y así las dependencias en modo legado
  // —p. ej. lucide-svelte, que usa `$$props`— siguen compilando sin romper.
  vitePlugin: {
    dynamicCompileOptions({ filename }) {
      // Fuerza runes solo en nuestro código fuente, nunca en node_modules.
      if (!filename.includes("node_modules")) {
        return { runes: true };
      }
    },
  },
};
