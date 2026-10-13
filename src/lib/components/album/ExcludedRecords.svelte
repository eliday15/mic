<!--
  ExcludedRecords — registros quitados a mano del filtro actual ("Quitar del
  filtro"). Un chip con el conteo restaura todos con su X; al pulsarlo se
  despliega la lista para devolver registros uno a uno. Cada registro se
  muestra por su primer campo visible (la clave) y su "Nombre" si existe.
-->
<script lang="ts">
  import { ChevronDown, EyeOff, Undo2, X } from "lucide-svelte";
  import { registroObtener } from "$lib/ipc/commands";
  import { t } from "$lib/i18n/es";
  import type { Valor } from "$lib/domain/types";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
  }

  let { estado }: Props = $props();

  let abierto = $state(false);
  const n = $derived(estado.excluidos.length);
  /** Etiqueta legible por id, resuelta al desplegar la lista. */
  let etiquetas = $state<Record<number, string>>({});
  /** Ids ya pedidos (no reactivo: evita repetir peticiones en vuelo). */
  const pedidos = new Set<number>();

  /** Campo clave (primer visible escalar) y campo "Nombre" de la tabla. */
  const campoClave = $derived(
    estado.camposVisibles.find((c) => c.tipo !== "multidato")?.nombre ?? null,
  );
  const campoNombre = $derived(
    estado.campos.find(
      (c) =>
        c.tabla === estado.tabla &&
        c.nombre.toLowerCase() === "nombre" &&
        c.nombre !== campoClave,
    )?.nombre ?? null,
  );

  $effect(() => {
    if (!abierto) return;
    for (const id of estado.excluidos) {
      if (pedidos.has(id)) continue;
      pedidos.add(id);
      void resolver(id);
    }
  });

  function texto(v: Valor | undefined): string {
    return v === null || v === undefined ? "" : String(v).trim();
  }

  async function resolver(id: number): Promise<void> {
    try {
      const reg = await registroObtener(estado.albumId, id, estado.tabla);
      const partes = [campoClave, campoNombre]
        .map((c) => (c ? texto(reg.valores[c]) : ""))
        .filter((p) => p !== "");
      etiquetas[id] = partes.length > 0 ? partes.join(" · ") : `${t.filtros.registroN} #${id}`;
    } catch {
      etiquetas[id] = `${t.filtros.registroN} #${id}`;
    }
  }
</script>

{#if n > 0}
  <div class="exr" data-testid="excluidos">
    <div class="exr__chip">
      <button
        type="button"
        class="exr__abrir"
        aria-expanded={abierto}
        onclick={() => (abierto = !abierto)}
      >
        <EyeOff size={12} />
        <span>{n} {n === 1 ? t.filtros.quitado : t.filtros.quitados}</span>
        <span class="exr__flecha" class:exr__flecha--abierta={abierto}>
          <ChevronDown size={12} />
        </span>
      </button>
      <button
        type="button"
        class="exr__x"
        aria-label={t.filtros.restaurarTodos}
        title={t.filtros.restaurarTodos}
        onclick={() => estado.restaurarExcluidos()}
      >
        <X size={12} />
      </button>
    </div>
    {#if abierto}
      <ul class="exr__lista">
        {#each estado.excluidos as id (id)}
          <li class="exr__item">
            <span class="exr__etq" title={etiquetas[id] ?? `#${id}`}>
              {etiquetas[id] ?? `${t.filtros.registroN} #${id}`}
            </span>
            <button
              type="button"
              class="exr__rest"
              title={t.filtros.restaurar}
              aria-label="{t.filtros.restaurar} {etiquetas[id] ?? `#${id}`}"
              onclick={() => estado.restaurarExcluido(id)}
            >
              <Undo2 size={12} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .exr {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .exr__chip {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    height: var(--alto-control-sm);
    padding: 0 var(--esp-1) 0 0;
    border-radius: var(--radio-pill);
    background: var(--color-aviso-tenue);
    color: var(--color-aviso);
    font-size: var(--tam-fuente-sm);
  }
  .exr__abrir {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    height: 100%;
    padding: 0 var(--esp-1) 0 var(--esp-2);
    border: none;
    border-radius: var(--radio-pill);
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .exr__flecha {
    display: inline-flex;
    transition: transform var(--transicion-rapida);
  }
  .exr__flecha--abierta {
    transform: rotate(180deg);
  }
  .exr__x,
  .exr__rest {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    border-radius: var(--radio-pill);
    background: transparent;
    color: inherit;
    opacity: 0.75;
    cursor: pointer;
  }
  .exr__x:hover,
  .exr__rest:hover {
    opacity: 1;
    background: var(--color-hover);
  }
  .exr__lista {
    list-style: none;
    margin: 0;
    padding: 0 0 0 var(--esp-2);
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 160px;
    overflow-y: auto;
  }
  .exr__item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .exr__etq {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .exr__rest {
    flex: none;
    color: var(--color-texto-secundario);
  }
</style>
