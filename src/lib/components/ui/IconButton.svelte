<!--
  IconButton — botón cuadrado solo-icono. El icono se pasa como snippet
  (típicamente un componente de lucide-svelte) y requiere `etiqueta` accesible.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  type Tamano = "sm" | "md" | "lg";

  interface Props extends HTMLButtonAttributes {
    /** Texto accesible (aria-label) obligatorio. */
    etiqueta: string;
    tamano?: Tamano;
    /** Resalta el botón como activo (toggle on). */
    activo?: boolean;
    children?: Snippet;
  }

  let {
    etiqueta,
    tamano = "md",
    activo = false,
    disabled = false,
    type = "button",
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class="icon-btn icon-btn--{tamano}"
  class:icon-btn--activo={activo}
  aria-label={etiqueta}
  aria-pressed={activo}
  title={etiqueta}
  {disabled}
  {...rest}
>
  {@render children?.()}
</button>

<style>
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--alto-control);
    height: var(--alto-control);
    padding: 0;
    border: 1px solid transparent;
    border-radius: var(--radio);
    background: transparent;
    color: var(--color-texto-secundario);
    cursor: pointer;
    transition:
      background var(--transicion),
      color var(--transicion),
      border-color var(--transicion);
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-texto);
  }

  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .icon-btn--activo {
    background: var(--color-acento-tenue);
    color: var(--color-acento);
  }

  .icon-btn--sm {
    width: var(--alto-control-sm);
    height: var(--alto-control-sm);
  }

  .icon-btn--lg {
    width: var(--alto-control-lg);
    height: var(--alto-control-lg);
  }
</style>
