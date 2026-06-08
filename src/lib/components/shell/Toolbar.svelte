<!--
  Toolbar — barra de herramientas con iconos (lucide) para las acciones más
  frecuentes: nuevo álbum, abrir, nueva imagen, buscar, ordenar, filtros, toggle
  grilla/tabla, ZoomSlider, y alternancia de tema. Las acciones se delegan al
  catálogo central (`ejecutarAccion`).
-->
<script lang="ts">
  import {
    FilePlus2,
    FolderOpen,
    ImagePlus,
    Search,
    ArrowDownUp,
    SlidersHorizontal,
    LayoutGrid,
    Table2,
    Sun,
    Moon,
    PanelLeft,
    PanelRight,
  } from "lucide-svelte";
  import { IconButton } from "$lib/components/ui";
  import ZoomSlider from "./ZoomSlider.svelte";
  import { ejecutarAccion } from "$lib/acciones";
  import { tema } from "$lib/stores/theme.svelte";
  import { vista } from "$lib/stores/vista.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    /** Álbum activo, o `null` si no hay ninguno. */
    estado: AlbumState | null;
  }

  let { estado }: Props = $props();

  const hayAlbum = $derived(estado !== null);
  const esGrilla = $derived(estado?.vista === "grilla");
</script>

<div class="tb">
  <!-- Grupo: archivo -->
  <div class="tb__grupo">
    <IconButton etiqueta={t.archivo.nuevoAlbum} onclick={() => ejecutarAccion("nuevo-album")}>
      <FilePlus2 size={17} />
    </IconButton>
    <IconButton etiqueta={t.archivo.abrir} onclick={() => ejecutarAccion("abrir")}>
      <FolderOpen size={17} />
    </IconButton>
  </div>

  <div class="tb__sep"></div>

  <!-- Grupo: edición -->
  <div class="tb__grupo">
    <IconButton
      etiqueta={t.editar.nuevaImagen}
      disabled={!hayAlbum}
      onclick={() => ejecutarAccion("nueva-imagen")}
    >
      <ImagePlus size={17} />
    </IconButton>
  </div>

  <div class="tb__sep"></div>

  <!-- Grupo: consulta -->
  <div class="tb__grupo">
    <IconButton
      etiqueta={t.herramientas.buscar}
      disabled={!hayAlbum}
      onclick={() => ejecutarAccion("buscar")}
    >
      <Search size={17} />
    </IconButton>
    <IconButton
      etiqueta={t.herramientas.ordenar}
      disabled={!hayAlbum}
      activo={(estado?.orden.length ?? 0) > 0}
      onclick={() => ejecutarAccion("ordenar")}
    >
      <ArrowDownUp size={17} />
    </IconButton>
    <IconButton
      etiqueta={t.herramientas.filtros}
      disabled={!hayAlbum}
      activo={estado?.hayFiltros ?? false}
      onclick={() => ejecutarAccion("filtros")}
    >
      <SlidersHorizontal size={17} />
    </IconButton>
  </div>

  <div class="tb__sep"></div>

  <!-- Grupo: vista -->
  <div class="tb__grupo">
    <IconButton
      etiqueta={t.ver.grilla}
      disabled={!hayAlbum}
      activo={esGrilla}
      onclick={() => estado?.setVista("grilla")}
    >
      <LayoutGrid size={17} />
    </IconButton>
    <IconButton
      etiqueta={t.ver.tabla}
      disabled={!hayAlbum}
      activo={hayAlbum && !esGrilla}
      onclick={() => estado?.setVista("tabla")}
    >
      <Table2 size={17} />
    </IconButton>
  </div>

  {#if estado && esGrilla}
    <div class="tb__sep"></div>
    <ZoomSlider {estado} />
  {/if}

  <!-- Empuja el resto a la derecha -->
  <div class="tb__flex"></div>

  <!-- Grupo: paneles + tema -->
  <div class="tb__grupo">
    <IconButton
      etiqueta={t.ver.panelGrupos}
      disabled={!hayAlbum}
      activo={vista.sidebarVisible}
      onclick={() => vista.alternarSidebar()}
    >
      <PanelLeft size={17} />
    </IconButton>
    <IconButton
      etiqueta={t.ver.inspector}
      disabled={!hayAlbum}
      activo={vista.inspectorVisible}
      onclick={() => vista.alternarInspector()}
    >
      <PanelRight size={17} />
    </IconButton>
    <IconButton etiqueta={t.ver.tema} onclick={() => tema.alternar()}>
      {#if tema.esOscuro}
        <Sun size={17} />
      {:else}
        <Moon size={17} />
      {/if}
    </IconButton>
  </div>
</div>

<style>
  .tb {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    height: 44px;
    padding: 0 var(--esp-3);
    background: var(--color-superficie);
    border-bottom: 1px solid var(--color-borde);
  }
  .tb__grupo {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .tb__sep {
    width: 1px;
    height: 20px;
    margin: 0 var(--esp-2);
    background: var(--color-borde);
  }
  .tb__flex {
    flex: 1;
  }
</style>
