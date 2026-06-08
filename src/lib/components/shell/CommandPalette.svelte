<!--
  CommandPalette — paleta de comandos (⌘K) estilo Linear. Búsqueda difusa sobre
  acciones, filtros guardados y grupos del álbum activo, agrupadas por sección.
  Lista navegable con teclado (flechas + Enter); Escape cierra.
-->
<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { Search } from "lucide-svelte";
  import { accionesPaleta, ejecutarAccion } from "$lib/acciones";
  import { albumes } from "$lib/stores/albums.svelte";
  import { filtrosListar, filtroObtener, gruposListar } from "$lib/ipc/commands";
  import { t } from "$lib/i18n/es";
  import type { Grupo } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(false), onCerrar }: Props = $props();

  interface Comando {
    id: string;
    etiqueta: string;
    grupo: string;
    ejecutar: () => void;
  }

  let consulta = $state("");
  let resaltado = $state(0);
  let entrada = $state<HTMLInputElement | null>(null);
  let filtrosGuardados = $state<string[]>([]);
  let grupos = $state<Grupo[]>([]);

  // Carga datos contextuales del álbum activo al abrir.
  $effect(() => {
    if (!abierto) return;
    consulta = "";
    resaltado = 0;
    queueMicrotask(() => entrada?.focus());
    cargarContexto();
  });

  async function cargarContexto(): Promise<void> {
    const a = albumes.activo;
    if (!a) {
      filtrosGuardados = [];
      grupos = [];
      return;
    }
    try {
      [filtrosGuardados, grupos] = await Promise.all([
        filtrosListar(a.albumId),
        gruposListar(a.albumId),
      ]);
    } catch {
      filtrosGuardados = [];
      grupos = [];
    }
  }

  // Construye la lista completa de comandos disponibles.
  const comandos = $derived.by<Comando[]>(() => {
    const a = albumes.activo;
    const lista: Comando[] = [];

    for (const acc of accionesPaleta()) {
      if (acc.requiereAlbum && !a) continue;
      lista.push({
        id: `acc:${acc.id}`,
        etiqueta: acc.etiqueta,
        grupo: acc.grupo,
        ejecutar: () => ejecutarAccion(acc.id),
      });
    }

    if (a) {
      for (const nombre of filtrosGuardados) {
        lista.push({
          id: `flt:${nombre}`,
          etiqueta: nombre,
          grupo: t.filtros.guardados,
          ejecutar: async () => {
            const conds = await filtroObtener(a.albumId, nombre);
            a.setCondiciones(conds);
          },
        });
      }
      for (const g of grupos) {
        lista.push({
          id: `grp:${g.id}`,
          etiqueta: g.nombre,
          grupo: t.grupos.titulo,
          ejecutar: () => a.setGrupoSel({ grupoId: g.id, valores: [] }),
        });
      }
    }

    return lista;
  });

  /** Coincidencia difusa: todos los caracteres de la consulta en orden. */
  function coincide(texto: string, q: string): boolean {
    if (q === "") return true;
    const t2 = texto.toLowerCase();
    const q2 = q.toLowerCase();
    let i = 0;
    for (const ch of t2) {
      if (ch === q2[i]) i++;
      if (i === q2.length) return true;
    }
    return i === q2.length;
  }

  const filtrados = $derived(
    comandos.filter((c) => coincide(c.etiqueta + " " + c.grupo, consulta.trim())),
  );

  // Agrupa preservando el orden de aparición de los grupos.
  const agrupados = $derived.by(() => {
    const mapa = new Map<string, Comando[]>();
    for (const c of filtrados) {
      const arr = mapa.get(c.grupo) ?? [];
      arr.push(c);
      mapa.set(c.grupo, arr);
    }
    return [...mapa.entries()];
  });

  // Índice plano para navegación por teclado.
  const planos = $derived(filtrados);

  $effect(() => {
    if (resaltado >= planos.length) resaltado = Math.max(0, planos.length - 1);
  });

  function ejecutar(c: Comando): void {
    cerrar();
    void c.ejecutar();
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }

  function onKeydown(e: KeyboardEvent): void {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (planos.length > 0) resaltado = (resaltado + 1) % planos.length;
        break;
      case "ArrowUp":
        e.preventDefault();
        if (planos.length > 0)
          resaltado = (resaltado - 1 + planos.length) % planos.length;
        break;
      case "Enter":
        e.preventDefault();
        if (planos[resaltado]) ejecutar(planos[resaltado]);
        break;
      case "Escape":
        e.preventDefault();
        // Solo cierra la paleta: que no llegue a la capa global de modales.
        e.stopPropagation();
        cerrar();
        break;
    }
  }

  function indiceDe(c: Comando): number {
    return planos.findIndex((x) => x.id === c.id);
  }
</script>

{#if abierto}
  <div
    class="cp__overlay"
    transition:fade={{ duration: 100 }}
    onclick={cerrar}
    role="presentation"
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="cp"
      transition:scale={{ duration: 130, start: 0.97 }}
      role="dialog"
      aria-modal="true"
      aria-label="Paleta de comandos"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="cp__buscar">
        <Search size={16} />
        <input
          bind:this={entrada}
          class="cp__input"
          type="text"
          placeholder={t.busqueda.placeholder}
          bind:value={consulta}
          onkeydown={onKeydown}
          aria-label={t.busqueda.placeholder}
        />
      </div>

      <div class="cp__lista" role="listbox">
        {#if planos.length === 0}
          <div class="cp__vacio">{t.busqueda.sinResultados}</div>
        {:else}
          {#each agrupados as [grupo, items] (grupo)}
            <div class="cp__grupo">{grupo}</div>
            {#each items as c (c.id)}
              {@const idx = indiceDe(c)}
              <button
                type="button"
                class="cp__op"
                class:cp__op--activo={idx === resaltado}
                role="option"
                aria-selected={idx === resaltado}
                onmousemove={() => (resaltado = idx)}
                onclick={() => ejecutar(c)}
              >
                {c.etiqueta}
              </button>
            {/each}
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .cp__overlay {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: var(--color-overlay);
  }
  .cp {
    width: 100%;
    max-width: 560px;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--color-elevado);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-lg);
    box-shadow: var(--sombra-3);
    overflow: hidden;
  }
  .cp__buscar {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    padding: var(--esp-3) var(--esp-4);
    border-bottom: 1px solid var(--color-borde);
    color: var(--color-texto-secundario);
  }
  .cp__input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--color-texto);
    font-size: var(--tam-fuente-lg);
    outline: none;
  }
  .cp__input::placeholder {
    color: var(--color-texto-tenue);
  }
  .cp__lista {
    overflow-y: auto;
    padding: var(--esp-1);
  }
  .cp__vacio {
    padding: var(--esp-5);
    text-align: center;
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-sm);
  }
  .cp__grupo {
    padding: var(--esp-2) var(--esp-2) var(--esp-1);
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-texto-tenue);
  }
  .cp__op {
    display: flex;
    align-items: center;
    width: 100%;
    height: var(--alto-control);
    padding: 0 var(--esp-2);
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto);
    text-align: left;
    cursor: pointer;
  }
  .cp__op--activo {
    background: var(--color-acento-tenue);
    color: var(--color-acento);
  }
</style>
