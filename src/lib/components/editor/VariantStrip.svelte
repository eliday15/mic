<!--
  VariantStrip — tira horizontal de miniaturas de las variantes de un registro
  principal. Permite seleccionar una variante para editarla y añadir variantes
  nuevas. Carga `variantes_listar` al montar y al pedir refresco.
-->
<script lang="ts">
  import { Plus, ImageOff } from "lucide-svelte";
  import { variantesListar } from "$lib/ipc/commands";
  import { thumbUrl } from "$lib/ipc/thumbnails";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { RegistroLigero } from "$lib/domain/types";

  interface Props {
    albumId: number;
    idPrincipal: number;
    /** Variante seleccionada (id), si alguna. */
    seleccionada?: number | null;
    /** Disparador externo de recarga: cambia para forzar refresco. */
    recarga?: number;
    onSeleccionar?: (id: number) => void;
    onNueva?: () => void;
  }

  let {
    albumId,
    idPrincipal,
    seleccionada = null,
    recarga = 0,
    onSeleccionar,
    onNueva,
  }: Props = $props();

  let variantes = $state<RegistroLigero[]>([]);
  let cargando = $state(false);

  $effect(() => {
    // Releer cuando cambie el principal o el disparador de recarga.
    void recarga;
    void idPrincipal;
    cargar();
  });

  async function cargar(): Promise<void> {
    cargando = true;
    try {
      variantes = await variantesListar(albumId, idPrincipal);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.cargarRegistros);
    } finally {
      cargando = false;
    }
  }
</script>

<div class="vstrip">
  <span class="vstrip__titulo">{t.registro.variantes}</span>
  <div class="vstrip__lista">
    {#each variantes as v (v.id)}
      <button
        type="button"
        class="vstrip__cel"
        class:vstrip__cel--sel={seleccionada === v.id}
        title={`#${v.id}`}
        onclick={() => onSeleccionar?.(v.id)}
      >
        {#if v.imagen}
          <img
            class="vstrip__img"
            src={thumbUrl(albumId, "variantes", v.id, 128, v.imagenVersion ?? 0)}
            alt={`Variante ${v.id}`}
            loading="lazy"
          />
        {:else}
          <span class="vstrip__sinimg"><ImageOff size={16} /></span>
        {/if}
      </button>
    {/each}

    <button
      type="button"
      class="vstrip__nueva"
      title={t.registro.nuevaVariante}
      onclick={onNueva}
    >
      <Plus size={18} />
    </button>
  </div>

  {#if !cargando && variantes.length === 0}
    <span class="vstrip__vacio">{t.registro.sinVariantes}</span>
  {/if}
</div>

<style>
  .vstrip {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .vstrip__titulo {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--color-texto-secundario);
  }
  .vstrip__lista {
    display: flex;
    gap: var(--esp-2);
    overflow-x: auto;
    padding-bottom: var(--esp-1);
  }
  .vstrip__cel,
  .vstrip__nueva {
    flex-shrink: 0;
    width: 64px;
    height: 64px;
    padding: 0;
    border: 2px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-panel);
    cursor: pointer;
    overflow: hidden;
    display: grid;
    place-items: center;
    transition:
      border-color var(--transicion),
      box-shadow var(--transicion);
  }
  .vstrip__cel:hover {
    border-color: var(--color-borde-fuerte);
  }
  .vstrip__cel--sel {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }
  .vstrip__img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .vstrip__sinimg {
    color: var(--color-texto-tenue);
  }
  .vstrip__nueva {
    color: var(--color-texto-secundario);
    border-style: dashed;
  }
  .vstrip__nueva:hover {
    color: var(--color-acento);
    border-color: var(--color-acento);
  }
  .vstrip__vacio {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
</style>
