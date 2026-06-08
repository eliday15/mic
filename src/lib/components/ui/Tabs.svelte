<!--
  Tabs — barra de pestañas controlada. Las pestañas se pasan como lista; la
  activa se enlaza con `activa` (id). Cada pestaña admite cierre opcional.
-->
<script lang="ts">
  import { X } from "lucide-svelte";

  interface Pestana {
    id: string | number;
    etiqueta: string;
    /** Si true, muestra el botón de cierre. */
    cerrable?: boolean;
  }

  interface Props {
    pestanas: Pestana[];
    /** Id de la pestaña activa (enlazado). */
    activa?: string | number;
    onActivar?: (id: string | number) => void;
    onCerrar?: (id: string | number) => void;
  }

  let { pestanas, activa = $bindable(), onActivar, onCerrar }: Props =
    $props();

  function activar(id: string | number): void {
    activa = id;
    onActivar?.(id);
  }
</script>

<div class="tabs" role="tablist">
  {#each pestanas as p (p.id)}
    <div class="tab" class:tab--activa={p.id === activa}>
      <button
        type="button"
        class="tab__btn"
        role="tab"
        aria-selected={p.id === activa}
        onclick={() => activar(p.id)}
      >
        <span class="tab__txt">{p.etiqueta}</span>
      </button>
      {#if p.cerrable}
        <button
          type="button"
          class="tab__cerrar"
          aria-label="Cerrar pestaña"
          onclick={() => onCerrar?.(p.id)}
        >
          <X size={12} />
        </button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    align-items: stretch;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--color-panel);
    border-bottom: 1px solid var(--color-borde);
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    border-top: 2px solid transparent;
    color: var(--color-texto-secundario);
    transition: background var(--transicion);
  }

  .tab:hover {
    background: var(--color-hover);
  }

  .tab--activa {
    background: var(--color-superficie);
    border-top-color: var(--color-acento);
    color: var(--color-texto);
  }

  .tab__btn {
    height: var(--alto-control-lg);
    padding: 0 var(--esp-2) 0 var(--esp-3);
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    max-width: 200px;
  }

  .tab__txt {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab__cerrar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    margin-right: var(--esp-2);
    padding: 0;
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
  }

  .tab__cerrar:hover {
    opacity: 1;
    background: var(--color-activo);
  }
</style>
