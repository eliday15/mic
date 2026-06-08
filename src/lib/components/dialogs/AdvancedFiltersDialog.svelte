<!--
  AdvancedFiltersDialog — editor de filtros avanzados (AND/OR) equivalente a
  frmFA del original. Filas con conector, campo, operador y valor; +/- para
  agregar/quitar; y gestión de filtros guardados (cargar/guardar/eliminar) por
  nombre. Aplica las condiciones al estado del álbum.
-->
<script lang="ts">
  import { Plus, Minus, Save, FolderOpen, Trash2 } from "lucide-svelte";
  import {
    Modal,
    Button,
    TextInput,
    Select,
    IconButton,
  } from "$lib/components/ui";
  import {
    filtrosListar,
    filtroObtener,
    filtroGuardar,
    filtroEliminar,
  } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type {
    CondicionFiltro,
    OpComp,
    OpRel,
  } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // svelte-ignore state_referenced_locally
  let condiciones = $state<CondicionFiltro[]>(
    estado.condiciones.length > 0
      ? estado.condiciones.map((c) => ({ ...c }))
      : [filaNueva(true)],
  );

  let guardados = $state<string[]>([]);
  let nombreFiltro = $state("");

  const opcionesCampo = $derived(
    estado.campos
      .filter((c) => c.tabla === estado.tabla && c.tipo !== "multidato")
      .sort((a, b) => a.ordenVisible - b.ordenVisible)
      .map((c) => ({ valor: c.nombre, etiqueta: c.nombre })),
  );

  const opcionesOp: { valor: OpComp; etiqueta: string }[] = [
    { valor: "igual", etiqueta: t.filtros.op.igual },
    { valor: "distinto", etiqueta: t.filtros.op.distinto },
    { valor: "mayor", etiqueta: t.filtros.op.mayor },
    { valor: "menor", etiqueta: t.filtros.op.menor },
    { valor: "mayor_igual", etiqueta: t.filtros.op.mayor_igual },
    { valor: "menor_igual", etiqueta: t.filtros.op.menor_igual },
    { valor: "contiene", etiqueta: t.filtros.op.contiene },
    { valor: "empieza", etiqueta: t.filtros.op.empieza },
  ];

  const opcionesRel: { valor: OpRel; etiqueta: string }[] = [
    { valor: "y", etiqueta: t.filtros.rel.y },
    { valor: "o", etiqueta: t.filtros.rel.o },
  ];

  function filaNueva(primera: boolean): CondicionFiltro {
    return {
      opRel: primera ? null : "y",
      campo: estado.campos[0]?.nombre ?? "",
      opComp: "igual",
      valor: "",
    };
  }

  $effect(() => {
    if (abierto) cargarGuardados();
  });

  async function cargarGuardados(): Promise<void> {
    try {
      guardados = await filtrosListar(estado.albumId);
    } catch {
      guardados = [];
    }
  }

  function agregar(): void {
    condiciones = [...condiciones, filaNueva(false)];
  }

  function quitar(indice: number): void {
    condiciones = condiciones.filter((_, i) => i !== indice);
    if (condiciones.length === 0) condiciones = [filaNueva(true)];
    else condiciones[0].opRel = null;
  }

  function aplicar(): void {
    const limpias = condiciones
      .filter((c) => c.campo !== "")
      .map((c, i) => ({ ...c, opRel: i === 0 ? null : (c.opRel ?? "y") }));
    estado.setCondiciones(limpias);
    cerrar();
  }

  function limpiarTodo(): void {
    estado.setCondiciones([]);
    cerrar();
  }

  async function guardar(): Promise<void> {
    const nombre = nombreFiltro.trim();
    if (nombre === "") {
      ui.aviso(t.filtros.nombreFiltro);
      return;
    }
    try {
      const limpias = condiciones
        .filter((c) => c.campo !== "")
        .map((c, i) => ({ ...c, opRel: i === 0 ? null : (c.opRel ?? "y") }));
      await filtroGuardar(estado.albumId, nombre, limpias);
      ui.exito(t.mensaje.guardado);
      nombreFiltro = "";
      await cargarGuardados();
      estado.marcarFiltros();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  async function cargar(nombre: string): Promise<void> {
    try {
      const conds = await filtroObtener(estado.albumId, nombre);
      condiciones = conds.length > 0 ? conds : [filaNueva(true)];
      nombreFiltro = nombre;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  async function eliminar(nombre: string): Promise<void> {
    try {
      await filtroEliminar(estado.albumId, nombre);
      await cargarGuardados();
      estado.marcarFiltros();
      ui.exito(t.mensaje.eliminado);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.filtros.titulo} ancho="lg" onCerrar={cerrar}>
  <div class="af">
    <div class="af__filas">
      {#each condiciones as cond, i (i)}
        <div class="af__fila">
          <div class="af__rel">
            {#if i === 0}
              <span class="af__donde">{t.filtros.condicion}</span>
            {:else}
              <Select
                value={cond.opRel ?? "y"}
                opciones={opcionesRel}
                onCambio={(v) => (cond.opRel = v)}
              />
            {/if}
          </div>
          <div class="af__campo">
            <Select bind:value={cond.campo} opciones={opcionesCampo} />
          </div>
          <div class="af__op">
            <Select bind:value={cond.opComp} opciones={opcionesOp} />
          </div>
          <div class="af__valor">
            <TextInput bind:value={cond.valor} placeholder={t.filtros.valor} />
          </div>
          <IconButton etiqueta={t.filtros.quitarCondicion} tamano="sm" onclick={() => quitar(i)}>
            <Minus size={14} />
          </IconButton>
        </div>
      {/each}
    </div>

    <Button variante="fantasma" tamano="sm" onclick={agregar}>
      <Plus size={14} />
      {t.filtros.agregarCondicion}
    </Button>

    <div class="af__guardados">
      <span class="af__titulo">{t.filtros.guardados}</span>
      {#if guardados.length === 0}
        <span class="af__vacio">{t.filtros.sinFiltros}</span>
      {:else}
        <ul class="af__lista">
          {#each guardados as nombre (nombre)}
            <li class="af__item">
              <span class="af__nombre">{nombre}</span>
              <IconButton etiqueta={t.accion.abrir} tamano="sm" onclick={() => cargar(nombre)}>
                <FolderOpen size={14} />
              </IconButton>
              <IconButton etiqueta={t.accion.eliminar} tamano="sm" onclick={() => eliminar(nombre)}>
                <Trash2 size={14} />
              </IconButton>
            </li>
          {/each}
        </ul>
      {/if}

      <div class="af__guardar">
        <TextInput bind:value={nombreFiltro} placeholder={t.filtros.nombreFiltro} />
        <Button variante="secundario" tamano="sm" onclick={guardar}>
          <Save size={14} />
          {t.accion.guardar}
        </Button>
      </div>
    </div>
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={limpiarTodo}>{t.filtros.limpiarTodo}</Button>
    <Button variante="primario" onclick={aplicar}>{t.accion.aplicar}</Button>
  {/snippet}
</Modal>

<style>
  .af {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .af__filas {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .af__fila {
    display: grid;
    grid-template-columns: 90px 1fr 150px 1fr auto;
    align-items: center;
    gap: var(--esp-2);
  }
  .af__donde {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .af__guardados {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding-top: var(--esp-3);
    border-top: 1px solid var(--color-borde);
  }
  .af__titulo {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .af__vacio {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
  .af__lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 160px;
    overflow-y: auto;
  }
  .af__item {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    padding: var(--esp-1);
    border-radius: var(--radio-sm);
  }
  .af__item:hover {
    background: var(--color-hover);
  }
  .af__nombre {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .af__guardar {
    display: flex;
    gap: var(--esp-2);
  }
  .af__guardar :global(.campo) {
    flex: 1;
  }
</style>
