<!--
  EmptyResults — estado vacío común de la grilla y la tabla. Distingue un
  álbum realmente vacío de una consulta sin coincidencias: en ese caso ofrece
  quitar los filtros (condiciones, exclusiones, búsqueda y grupo).
-->
<script lang="ts">
  import { ImagePlus, SearchX } from "lucide-svelte";
  import { Button, EmptyState } from "$lib/components/ui";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
  }

  let { estado }: Props = $props();
</script>

<div class="vacio" data-testid="sin-resultados">
  {#if estado.hayFiltros}
    <EmptyState titulo={t.vacio.sinCoincidencias} descripcion={t.vacio.sinCoincidenciasDesc}>
      {#snippet icono()}<SearchX size={36} />{/snippet}
      {#snippet accion()}
        <Button variante="secundario" tamano="sm" onclick={() => estado.limpiarFiltros()}>
          {t.vacio.quitarFiltros}
        </Button>
      {/snippet}
    </EmptyState>
  {:else}
    <EmptyState titulo={t.vacio.albumTitulo} descripcion={t.vacio.albumDesc}>
      {#snippet icono()}<ImagePlus size={36} />{/snippet}
    </EmptyState>
  {/if}
</div>

<style>
  .vacio {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--color-texto-secundario);
  }
</style>
