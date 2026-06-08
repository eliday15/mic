<!--
  ZoomSlider — control de zoom de las miniaturas. Enlaza el tamaño de celda del
  álbum activo (90–420 px) y ofrece presets de columnas fijas por fila
  (Auto/2/4/6/8, ex 8/4/2 por línea del original). Mover el zoom vuelve a Auto.
-->
<script lang="ts">
  import { ZoomIn, ZoomOut, Type } from "lucide-svelte";
  import { ZOOM_MIN, ZOOM_MAX } from "$lib/stores/albumState.svelte";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
  }

  let { estado }: Props = $props();

  /** Presets de columnas por fila (null = auto según zoom). */
  const PRESETS: (number | null)[] = [null, 2, 4, 6, 8];

  function onInput(e: Event): void {
    estado.setZoom(Number((e.currentTarget as HTMLInputElement).value));
  }
</script>

<div class="zoom">
  <button
    type="button"
    class="zoom__btn"
    aria-label="Reducir miniaturas"
    onclick={() => estado.reducirZoom()}
  >
    <ZoomOut size={15} />
  </button>
  <input
    class="zoom__rango"
    type="range"
    min={ZOOM_MIN}
    max={ZOOM_MAX}
    step="2"
    value={estado.zoom}
    aria-label="Tamaño de miniaturas"
    oninput={onInput}
  />
  <button
    type="button"
    class="zoom__btn"
    aria-label="Aumentar miniaturas"
    onclick={() => estado.aumentarZoom()}
  >
    <ZoomIn size={15} />
  </button>

  <!-- La letra de las cards escala con el zoom (toggle) -->
  <button
    type="button"
    class="zoom__letra"
    class:zoom__letra--activo={estado.letraEscala}
    aria-pressed={estado.letraEscala}
    title={estado.letraEscala
      ? "La letra escala con el zoom (clic: dejarla fija)"
      : "Letra fija (clic: escalar con el zoom)"}
    onclick={() => estado.alternarLetraEscala()}
  >
    <Type size={13} />
  </button>

  <!-- Columnas por fila (ex 8/4/2 por línea) -->
  <div class="zoom__cols" role="group" aria-label="Columnas por fila">
    {#each PRESETS as p (p ?? "auto")}
      <button
        type="button"
        class="zoom__col"
        class:zoom__col--activo={estado.columnasFijas === p}
        aria-pressed={estado.columnasFijas === p}
        title={p === null ? "Columnas automáticas" : `${p} por fila`}
        onclick={() => estado.setColumnasFijas(p)}
      >
        {p === null ? "A" : p}
      </button>
    {/each}
  </div>
</div>

<style>
  .zoom {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
  }
  .zoom__btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--alto-control-sm);
    height: var(--alto-control-sm);
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto-secundario);
    cursor: pointer;
  }
  .zoom__btn:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }
  .zoom__rango {
    width: 110px;
    height: 4px;
    appearance: none;
    background: var(--color-borde-fuerte);
    border-radius: var(--radio-pill);
    outline: none;
    cursor: pointer;
  }
  .zoom__rango::-webkit-slider-thumb {
    appearance: none;
    width: 13px;
    height: 13px;
    border-radius: 50%;
    background: var(--color-acento);
    cursor: pointer;
  }
  .zoom__rango::-moz-range-thumb {
    width: 13px;
    height: 13px;
    border: none;
    border-radius: 50%;
    background: var(--color-acento);
    cursor: pointer;
  }

  .zoom__letra {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    margin-left: var(--esp-1);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto-tenue);
    cursor: pointer;
  }
  .zoom__letra:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }
  .zoom__letra--activo {
    color: var(--color-acento);
    border-color: var(--color-acento);
  }

  .zoom__cols {
    display: inline-flex;
    gap: 2px;
    margin-left: var(--esp-2);
    padding: 2px;
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
  }
  .zoom__col {
    min-width: 22px;
    height: 20px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-xs);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .zoom__col:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }
  .zoom__col--activo {
    background: var(--color-acento);
    color: #fff;
  }
</style>
