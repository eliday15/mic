<!--
  Chip — etiqueta compacta, opcionalmente removible (filtros activos, tags).
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { X } from "lucide-svelte";

  type Variante = "neutro" | "acento" | "peligro";

  interface Props {
    variante?: Variante;
    /** Si se define, muestra la X y se invoca al pulsarla. */
    onQuitar?: () => void;
    /** Etiqueta accesible del botón de quitar. */
    etiquetaQuitar?: string;
    children?: Snippet;
  }

  let {
    variante = "neutro",
    onQuitar,
    etiquetaQuitar = "Quitar",
    children,
  }: Props = $props();
</script>

<span class="chip chip--{variante}">
  <span class="chip__txt">{@render children?.()}</span>
  {#if onQuitar}
    <button
      type="button"
      class="chip__quitar"
      aria-label={etiquetaQuitar}
      onclick={onQuitar}
    >
      <X size={12} />
    </button>
  {/if}
</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border-radius: var(--radio-pill);
    font-size: var(--tam-fuente-sm);
    border: 1px solid var(--color-borde);
    background: var(--color-superficie);
    max-width: 220px;
  }

  .chip__txt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip--acento {
    background: var(--color-acento-tenue);
    border-color: transparent;
    color: var(--color-acento);
  }

  .chip--peligro {
    background: var(--color-peligro-tenue);
    border-color: transparent;
    color: var(--color-peligro);
  }

  .chip__quitar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    margin-right: -2px;
    padding: 0;
    border: none;
    border-radius: var(--radio-pill);
    background: transparent;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
    transition: opacity var(--transicion-rapida);
  }

  .chip__quitar:hover {
    opacity: 1;
    background: var(--color-hover);
  }
</style>
