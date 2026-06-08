<!--
  Button — botón primitivo. Variantes visuales con tokens de app.css.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Spinner from "./Spinner.svelte";

  type Variante = "primario" | "secundario" | "fantasma" | "peligro";
  type Tamano = "sm" | "md" | "lg";

  interface Props extends HTMLButtonAttributes {
    variante?: Variante;
    tamano?: Tamano;
    /** Muestra spinner y deshabilita la interacción. */
    cargando?: boolean;
    /** Ocupa todo el ancho disponible. */
    ancho?: boolean;
    children?: Snippet;
  }

  let {
    variante = "secundario",
    tamano = "md",
    cargando = false,
    ancho = false,
    disabled = false,
    type = "button",
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class="btn btn--{variante} btn--{tamano}"
  class:btn--ancho={ancho}
  class:btn--cargando={cargando}
  disabled={disabled || cargando}
  {...rest}
>
  {#if cargando}
    <Spinner tamano={14} />
  {/if}
  {@render children?.()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--esp-2);
    height: var(--alto-control);
    padding: 0 var(--esp-3);
    border: 1px solid transparent;
    border-radius: var(--radio);
    background: transparent;
    color: var(--color-texto);
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background var(--transicion),
      border-color var(--transicion),
      color var(--transicion);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn--ancho {
    width: 100%;
  }

  .btn--sm {
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    font-size: var(--tam-fuente-sm);
  }

  .btn--lg {
    height: var(--alto-control-lg);
    padding: 0 var(--esp-4);
  }

  /* Primario */
  .btn--primario {
    background: var(--color-acento);
    color: var(--color-texto-sobre-acento);
  }
  .btn--primario:hover:not(:disabled) {
    background: var(--color-acento-hover);
  }

  /* Secundario */
  .btn--secundario {
    background: var(--color-superficie);
    border-color: var(--color-borde);
  }
  .btn--secundario:hover:not(:disabled) {
    background: var(--color-elevado);
    border-color: var(--color-borde-fuerte);
  }

  /* Fantasma */
  .btn--fantasma {
    background: transparent;
  }
  .btn--fantasma:hover:not(:disabled) {
    background: var(--color-hover);
  }

  /* Peligro */
  .btn--peligro {
    background: var(--color-peligro);
    color: #fff;
  }
  .btn--peligro:hover:not(:disabled) {
    background: var(--color-peligro-hover);
  }
</style>
