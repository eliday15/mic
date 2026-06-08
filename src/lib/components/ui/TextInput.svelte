<!--
  TextInput — campo de texto con binding bidireccional (`value`), estado de
  error y prefijo/sufijo opcionales (snippets).
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLInputAttributes } from "svelte/elements";

  interface Props extends Omit<HTMLInputAttributes, "value" | "size"> {
    /** Valor enlazado bidireccionalmente. */
    value?: string;
    /** Marca visual de error y `aria-invalid`. */
    invalido?: boolean;
    /** Snippet renderizado antes del input (icono, etc.). */
    prefijo?: Snippet;
    /** Snippet renderizado después del input. */
    sufijo?: Snippet;
  }

  let {
    value = $bindable(""),
    invalido = false,
    prefijo,
    sufijo,
    type = "text",
    ...rest
  }: Props = $props();
</script>

<div class="campo" class:campo--invalido={invalido}>
  {#if prefijo}
    <span class="campo__adorno">{@render prefijo()}</span>
  {/if}
  <input
    class="campo__input"
    {type}
    bind:value
    aria-invalid={invalido}
    {...rest}
  />
  {#if sufijo}
    <span class="campo__adorno">{@render sufijo()}</span>
  {/if}
</div>

<style>
  .campo {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    height: var(--alto-control);
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
    transition: border-color var(--transicion);
  }

  .campo:focus-within {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }

  .campo--invalido {
    border-color: var(--color-peligro);
  }
  .campo--invalido:focus-within {
    box-shadow: 0 0 0 2px var(--color-peligro-tenue);
  }

  .campo__input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--color-texto);
    outline: none;
  }

  .campo__input::placeholder {
    color: var(--color-texto-tenue);
  }

  .campo__adorno {
    display: inline-flex;
    align-items: center;
    color: var(--color-texto-secundario);
    flex-shrink: 0;
  }
</style>
