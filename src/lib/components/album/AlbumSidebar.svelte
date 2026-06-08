<!--
  AlbumSidebar — barra lateral izquierda con dos pestañas: Grupos y Filtros.
  Aloja GroupTree y FiltersPanel. Delega la apertura del diálogo de filtros
  avanzados al contenedor (AlbumView).
-->
<script lang="ts">
  import { Tabs } from "$lib/components/ui";
  import GroupTree from "./GroupTree.svelte";
  import FiltersPanel from "./FiltersPanel.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
    onAvanzados: () => void;
  }

  let { estado, onAvanzados }: Props = $props();

  let activa = $state<string | number>("grupos");
</script>

<div class="sidebar">
  <Tabs
    pestanas={[
      { id: "grupos", etiqueta: t.grupos.titulo },
      { id: "filtros", etiqueta: t.filtros.titulo },
    ]}
    bind:activa
  />
  <div class="sidebar__cuerpo">
    {#if activa === "grupos"}
      <GroupTree {estado} />
    {:else}
      <FiltersPanel {estado} {onAvanzados} />
    {/if}
  </div>
</div>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-panel);
    border-right: 1px solid var(--color-borde);
  }
  .sidebar__cuerpo {
    flex: 1;
    overflow-y: auto;
    padding: var(--esp-3);
    min-height: 0;
  }
</style>
