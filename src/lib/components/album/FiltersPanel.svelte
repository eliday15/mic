<!--
  FiltersPanel — panel de filtros del sidebar. Ofrece un filtro rápido
  (campo + valor), muestra las condiciones avanzadas activas como chips, un botón
  para abrir el AdvancedFiltersDialog, y la lista de filtros guardados para
  aplicarlos con un clic.
-->
<script lang="ts">
  import { Filter, SlidersHorizontal } from "lucide-svelte";
  import {
    Select,
    TextInput,
    Button,
    Chip,
    EmptyState,
  } from "$lib/components/ui";
  import { filtrosListar, filtroObtener } from "$lib/ipc/commands";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
    /** Abre el diálogo de filtros avanzados. */
    onAvanzados: () => void;
  }

  let { estado, onAvanzados }: Props = $props();

  // svelte-ignore state_referenced_locally
  let campoRapido = $state(estado.filtroRapido?.campo ?? "");
  // svelte-ignore state_referenced_locally
  let valorRapido = $state(estado.filtroRapido?.valor ?? "");
  let guardados = $state<string[]>([]);

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

  function aplicarRapido(): void {
    if (campoRapido === "" || valorRapido.trim() === "") {
      estado.setFiltroRapido(null);
      return;
    }
    estado.setFiltroRapido({ campo: campoRapido, valor: valorRapido.trim() });
  }

  function limpiarRapido(): void {
    valorRapido = "";
    estado.setFiltroRapido(null);
  }

  function etiquetaOp(op: string): string {
    return (t.filtros.op as Record<string, string>)[op] ?? op;
  }

  async function aplicarGuardado(nombre: string): Promise<void> {
    try {
      const conds = await filtroObtener(estado.albumId, nombre);
      estado.setCondiciones(conds);
    } catch {
      /* error reflejado globalmente al recargar */
    }
  }

  function onKeyRapido(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      aplicarRapido();
    }
  }
</script>

<div class="fp">
  <!-- Filtro rápido -->
  <section class="fp__sec">
    <span class="fp__titulo">{t.filtros.rapido}</span>
    <Select
      bind:value={campoRapido}
      opciones={opcionesCampo}
      placeholder={t.filtros.campo}
    />
    <div class="fp__valor" onkeydown={onKeyRapido} role="presentation">
      <TextInput bind:value={valorRapido} placeholder={t.filtros.valor}>
        {#snippet prefijo()}
          <Filter size={14} />
        {/snippet}
      </TextInput>
    </div>
    <div class="fp__acc">
      <Button variante="primario" tamano="sm" ancho onclick={aplicarRapido}>
        {t.accion.aplicar}
      </Button>
      {#if estado.filtroRapido}
        <Button variante="fantasma" tamano="sm" onclick={limpiarRapido}>
          {t.accion.limpiar}
        </Button>
      {/if}
    </div>
  </section>

  <!-- Condiciones avanzadas activas -->
  <section class="fp__sec">
    <div class="fp__cabe">
      <span class="fp__titulo">{t.filtros.titulo}</span>
      <Button
        variante="fantasma"
        tamano="sm"
        aria-label={t.filtros.titulo}
        onclick={onAvanzados}
      >
        <SlidersHorizontal size={14} />
      </Button>
    </div>
    {#if estado.condiciones.length === 0}
      <span class="fp__vacio">{t.vacio.descripcion}</span>
    {:else}
      <div class="fp__chips">
        {#each estado.condiciones as cond, i (i)}
          <Chip
            variante="acento"
            onQuitar={() => estado.quitarCondicion(i)}
          >
            {cond.campo} {etiquetaOp(cond.opComp)} {cond.valor}
          </Chip>
        {/each}
      </div>
      <Button variante="fantasma" tamano="sm" onclick={() => estado.setCondiciones([])}>
        {t.filtros.limpiarTodo}
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
  .fp__acc {
    display: flex;
    gap: var(--esp-2);
  }
  .fp__acc :global(.btn--primario) {
    flex: 1;
  }
  .fp__vacio {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
  .fp__chips {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    align-items: flex-start;
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
