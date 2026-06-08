<!--
  Tooltip — burbuja informativa que aparece al pasar el ratón o enfocar el
  contenido envuelto. El contenido va en el slot por defecto; el texto en `texto`.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  type Lado = "arriba" | "abajo" | "izquierda" | "derecha";

  interface Props {
    texto: string;
    lado?: Lado;
    /** Retardo en ms antes de mostrarse. */
    retardo?: number;
    children: Snippet;
  }

  let { texto, lado = "arriba", retardo = 350, children }: Props = $props();

  let visible = $state(false);
  let temporizador: ReturnType<typeof setTimeout> | null = null;

  function mostrar(): void {
    temporizador = setTimeout(() => (visible = true), retardo);
  }

  function ocultar(): void {
    if (temporizador) {
      clearTimeout(temporizador);
      temporizador = null;
    }
    visible = false;
  }
</script>

<span
  class="tooltip-anchor"
  onmouseenter={mostrar}
  onmouseleave={ocultar}
  onfocusin={mostrar}
  onfocusout={ocultar}
  role="presentation"
>
  {@render children()}
  {#if visible}
    <span class="tooltip tooltip--{lado}" role="tooltip">{texto}</span>
  {/if}
</span>

<style>
  .tooltip-anchor {
    position: relative;
    display: inline-flex;
  }

  .tooltip {
    position: absolute;
    z-index: var(--z-tooltip);
    padding: var(--esp-1) var(--esp-2);
    border-radius: var(--radio-sm);
    background: var(--color-elevado);
    color: var(--color-texto);
    border: 1px solid var(--color-borde);
    box-shadow: var(--sombra-2);
    font-size: var(--tam-fuente-xs);
    white-space: nowrap;
    pointer-events: none;
    animation: aparecer var(--transicion-rapida) ease-out;
  }

  .tooltip--arriba {
    bottom: calc(100% + var(--esp-1));
    left: 50%;
    transform: translateX(-50%);
  }
  .tooltip--abajo {
    top: calc(100% + var(--esp-1));
    left: 50%;
    transform: translateX(-50%);
  }
  .tooltip--izquierda {
    right: calc(100% + var(--esp-1));
    top: 50%;
    transform: translateY(-50%);
  }
  .tooltip--derecha {
    left: calc(100% + var(--esp-1));
    top: 50%;
    transform: translateY(-50%);
  }

  @keyframes aparecer {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
