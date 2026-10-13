<!--
  FiltersPanel — panel de filtros del sidebar. Los filtros libres (campo +
  operador + valor) se suman a la lista de condiciones activas, que se puede
  reordenar y combinar con Y/O; los registros seleccionados se pueden quitar a
  mano del resultado. Incluye el acceso al AdvancedFiltersDialog y la lista de
  filtros guardados para aplicarlos con un clic.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { EyeOff, Filter, Plus, SlidersHorizontal } from "lucide-svelte";
  import { Select, TextInput, Button, EmptyState } from "$lib/components/ui";
  import ActiveFiltersList from "./ActiveFiltersList.svelte";
  import ExcludedRecords from "./ExcludedRecords.svelte";
  import { filtrosListar, filtroObtener, registrosQuery } from "$lib/ipc/commands";
  import { t } from "$lib/i18n/es";
  import type { OpComp } from "$lib/domain/types";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
    /** Abre el diálogo de filtros avanzados. */
    onAvanzados: () => void;
  }

  let { estado, onAvanzados }: Props = $props();

  let campoLibre = $state("");
  let opLibre = $state<OpComp>("igual");
  let valorLibre = $state("");
  let guardados = $state<string[]>([]);

  const opcionesOp = (Object.keys(t.filtros.op) as OpComp[]).map((op) => ({
    valor: op,
    etiqueta: t.filtros.op[op],
  }));
  /** Tope de ids a traer para cruzar la selección con el resultado actual. */
  const LIMITE_RESULTADO = 100000;

  // Ids del resultado actual, solo cuando hace falta: si la consulta cambia con
  // registros ya seleccionados, algunos pueden haber quedado ocultos por el
  // filtro. Lo seleccionado DESPUÉS del último cambio siempre es visible (se
  // eligió en la grilla), así que basta recalcular al cambiar la consulta.
  let idsResultado = $state<Set<number> | null>(null);
  $effect(() => {
    void estado.versionConsulta;
    let vigente = true;
    untrack(() => {
      idsResultado = null;
      if (estado.seleccion.size === 0) return;
      registrosQuery(estado.albumId, estado.construirQuery(0, LIMITE_RESULTADO))
        .then((p) => {
          if (vigente) idsResultado = new Set(p.registros.map((r) => r.id));
        })
        .catch(() => {
          // Sin el resultado se cuenta la selección completa.
        });
    });
    return () => {
      vigente = false;
    };
  });

  // Seleccionados que siguen visibles en el resultado actual.
  const seleccionVisible = $derived(
    idsResultado === null
      ? [...estado.seleccion]
      : [...estado.seleccion].filter((id) => idsResultado?.has(id)),
  );
  const seleccionados = $derived(seleccionVisible.length);

  const opcionesCampo = $derived(
    estado.campos
      .filter((c) => c.tabla === estado.tabla && c.tipo !== "multidato")
      .sort((a, b) => a.ordenVisible - b.ordenVisible)
      .map((c) => ({ valor: c.nombre, etiqueta: c.nombre })),
  );

  $effect(() => {
    void estado.albumId;
    // Recargar al cambiar la versión de filtros (guardar/eliminar en el diálogo).
    void estado.versionFiltros;
    cargarGuardados();
  });

  async function cargarGuardados(): Promise<void> {
    try {
      guardados = await filtrosListar(estado.albumId);
    } catch {
      guardados = [];
    }
  }

  /** Suma un filtro libre al final de la lista activa (Y con los anteriores). */
  function agregarLibre(): void {
    const valor = valorLibre.trim();
    if (campoLibre === "" || valor === "") return;
    estado.agregarCondicion({
      opRel: estado.condiciones.length === 0 ? null : "y",
      campo: campoLibre,
      opComp: opLibre,
      valor,
    });
    valorLibre = "";
  }

  async function aplicarGuardado(nombre: string): Promise<void> {
    try {
      const conds = await filtroObtener(estado.albumId, nombre);
      estado.setCondiciones(conds);
    } catch {
      /* error reflejado globalmente al recargar */
    }
  }

  function onKeyLibre(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      agregarLibre();
    }
  }
</script>

<div class="fp">
  <!-- Filtro libre: cada uno se suma a la lista activa -->
  <section class="fp__sec">
    <div class="fp__cabe">
      <span class="fp__titulo">{t.filtros.libres}</span>
      <Button
        variante="fantasma"
        tamano="sm"
        aria-label={t.filtros.titulo}
        title={t.filtros.titulo}
        onclick={onAvanzados}
      >
        <SlidersHorizontal size={14} />
      </Button>
    </div>
    <Select
      bind:value={campoLibre}
      opciones={opcionesCampo}
      placeholder={t.filtros.campo}
    />
    <Select bind:value={opLibre} opciones={opcionesOp} etiqueta={t.filtros.operador} />
    <div class="fp__valor" onkeydown={onKeyLibre} role="presentation">
      <TextInput bind:value={valorLibre} placeholder={t.filtros.valor}>
        {#snippet prefijo()}
          <Filter size={14} />
        {/snippet}
      </TextInput>
    </div>
    <Button
      variante="primario"
      tamano="sm"
      ancho
      disabled={campoLibre === "" || valorLibre.trim() === ""}
      onclick={agregarLibre}
    >
      <Plus size={14} />
      {t.filtros.agregarFiltro}
    </Button>
  </section>

  <!-- Filtros activos (apilados, reordenables) y exclusiones manuales -->
  <section class="fp__sec">
    <span class="fp__titulo">{t.filtros.activo}</span>
    {#if estado.condiciones.length === 0 && estado.excluidos.length === 0}
      <span class="fp__vacio">{t.filtros.sinActivos}</span>
    {:else}
      <ActiveFiltersList {estado} />
      <ExcludedRecords {estado} />
      <Button variante="fantasma" tamano="sm" onclick={() => estado.limpiarCondiciones()}>
        {t.filtros.limpiarTodo}
      </Button>
    {/if}
    {#if seleccionados > 0}
      <Button
        variante="secundario"
        tamano="sm"
        ancho
        onclick={() => estado.excluir(seleccionVisible)}
      >
        <EyeOff size={14} />
        {t.filtros.quitarSeleccion} ({seleccionados})
      </Button>
    {/if}
  </section>

  <!-- Filtros guardados -->
  <section class="fp__sec">
    <span class="fp__titulo">{t.filtros.guardados}</span>
    {#if guardados.length === 0}
      <EmptyState titulo={t.filtros.sinFiltros} />
    {:else}
      <ul class="fp__guardados">
        {#each guardados as nombre (nombre)}
          <li>
            <button
              type="button"
              class="fp__guardado"
              onclick={() => aplicarGuardado(nombre)}
            >
              {nombre}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .fp {
    display: flex;
    flex-direction: column;
    gap: var(--esp-5);
    padding: var(--esp-1);
  }
  .fp__sec {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .fp__cabe {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .fp__titulo {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-texto-secundario);
  }
  .fp__vacio {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
  .fp__guardados {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .fp__guardado {
    width: 100%;
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto);
    text-align: left;
    cursor: pointer;
  }
  .fp__guardado:hover {
    background: var(--color-hover);
  }
</style>
