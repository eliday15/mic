<!--
  EmptyState — estado vacío con título, descripción y acción opcional.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    titulo: string;
    descripcion?: string;
    /** Icono opcional (snippet, p. ej. componente lucide). */
    icono?: Snippet;
    /** Acción opcional (snippet, p. ej. un Button). */
    accion?: Snippet;
  }

  let { titulo, descripcion, icono, accion }: Props = $props();
</script>

<div class="vacio">
  {#if icono}
    <div class="vacio__icono">{@render icono()}</div>
  {/if}
  <p class="vacio__titulo">{titulo}</p>
  {#if descripcion}
    <p class="vacio__desc">{descripcion}</p>
  {/if}
  {#if accion}
    <div class="vacio__accion">{@render accion()}</div>
  {/if}
</div>

<style>
  .vacio {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--esp-2);
    text-align: center;
    padding: var(--esp-8) var(--esp-4);
    color: var(--color-texto-secundario);
  }

  .vacio__icono {
    color: var(--color-texto-tenue);
    margin-bottom: var(--esp-1);
  }

  .vacio__titulo {
    margin: 0;
    font-weight: 600;
    color: var(--color-texto);
  }

  .vacio__desc {
    margin: 0;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
    max-width: 320px;
  }

  .vacio__accion {
    margin-top: var(--esp-3);
  }
</style>
