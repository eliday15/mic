/**
 * Punto de entrada del frontend: monta el componente raíz en `#app`.
 *
 * En desarrollo, si la app corre en un navegador normal (sin Tauri), instala
 * primero el mock IPC para que toda la UI funcione contra datos en memoria.
 */

import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { tema } from "$lib/stores/theme.svelte";
import { ui } from "$lib/stores/ui.svelte";
import { albumes } from "$lib/stores/albums.svelte";

async function iniciar(): Promise<void> {
  // Modo navegador (vite dev sin Tauri): mock IPC con álbum demo en memoria.
  // El import es dinámico para que el mock jamás entre al bundle de producción.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { instalarMock } = await import("$lib/ipc/mock");
    instalarMock();
  }

  // Aplica el tema persistido antes del primer render para evitar parpadeo.
  tema.inicializar();

  // Instrumentación de desarrollo: expone los stores reales para depuración
  // (consola del webview / pruebas automatizadas). Nunca en producción.
  if (import.meta.env.DEV) {
    (window as unknown as Record<string, unknown>).__mic = { ui, albumes, tema };
  }

  const contenedor = document.getElementById("app");
  if (!contenedor) {
    throw new Error("No se encontró el contenedor #app en index.html");
  }

  mount(App, { target: contenedor });
}

void iniciar();
