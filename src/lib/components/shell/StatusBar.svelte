<!--
  StatusBar — barra inferior de estado del álbum activo. Muestra el total de
  registros, chips de los filtros/orden/búsqueda activos y el contador de
  selección.
-->
<script lang="ts">
  import { Search, ArrowDownUp, SlidersHorizontal, FolderTree } from "lucide-svelte";
  import { Chip } from "$lib/components/ui";
  import { formatearEntero } from "$lib/utils/format";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState | null;
  }

  let { estado }: Props = $props();

  const seleccionados = $derived(estado?.seleccion.size ?? 0);
</script>

<div class="sb">
  {#if estado}
    <span class="sb__total tabular">
      {#if estado.rangoVisible}
        {formatearEntero(estado.rangoVisible[0])}–{formatearEntero(estado.rangoVisible[1])} /
      {/if}
      {formatearEntero(estado.total)} {t.grupos.conteo}
    </span>

    <div class="sb__chips">
      {#if estado.busqueda.trim() !== ""}
        <Chip variante="acento" onQuitar={() => estado.setBusqueda("")}>
          <span class="sb__chiptxt"><Search size={11} /> {estado.busqueda}</span>
        </Chip>
      {/if}
      {#if estado.grupoSel}
        <Chip variante="acento" onQuitar={() => estado.setGrupoSel(null)}>
          <span class="sb__chiptxt"><FolderTree size={11} /> {t.grupos.titulo}</span>
        </Chip>
      {/if}
      {#if estado.filtroRapido}
        <Chip variante="acento" onQuitar={() => estado.setFiltroRapido(null)}>
          <span class="sb__chiptxt">
            <SlidersHorizontal size={11} />
            {estado.filtroRapido.campo}: {estado.filtroRapido.valor}
          </span>
        </Chip>
      {/if}
      {#if estado.condiciones.length > 0}
        <Chip variante="acento" onQuitar={() => estado.setCondiciones([])}>
          <span class="sb__chiptxt">
            <SlidersHorizontal size={11} />
            {estado.condiciones.length} {t.filtros.condicion}
          </span>
        </Chip>
      {/if}
      {#if estado.orden.length > 0}
        <Chip variante="neutro" onQuitar={() => estado.setOrden([])}>
          <span class="sb__chiptxt">
            <ArrowDownUp size={11} />
            {estado.orden.map((o) => o.campo).join(", ")}
          </span>
        </Chip>
      {/if}
    </div>

    <div class="sb__flex"></div>

    {#if seleccionados > 0}
      <span class="sb__sel tabular">
        {formatearEntero(seleccionados)} {t.registro.seleccion}
      </span>
    {/if}
  {/if}
</div>

<style>
  .sb {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    height: 26px;
    padding: 0 var(--esp-3);
    background: var(--color-panel);
    border-top: 1px solid var(--color-borde);
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
    flex-shrink: 0;
    overflow: hidden;
  }
  .sb__total {
    flex-shrink: 0;
    font-weight: 600;
  }
  .sb__chips {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    overflow: hidden;
  }
  .sb__chiptxt {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .sb__flex {
    flex: 1;
  }
  .sb__sel {
    flex-shrink: 0;
    color: var(--color-acento);
    font-weight: 600;
  }

  /* Chips compactos en la barra de estado */
  .sb :global(.chip) {
    height: 20px;
    font-size: var(--tam-fuente-xs);
  }
</style>
