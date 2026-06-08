<!--
  PrintDialog — diálogo de impresión / reportes (ex frmprint / frmprint2 /
  frmPreliminar del VB6). Configura un reporte con o sin imágenes, lo guarda en
  el álbum, muestra una vista previa con zoom y lo imprime con `window.print()`.
-->
<script lang="ts">
  import { Eye, Printer, Save, Trash2 } from "lucide-svelte";
  import { Modal, Button, Select, TextInput } from "$lib/components/ui";
  import PrintSheet from "$lib/components/print/PrintSheet.svelte";
  import {
    reportesListar,
    reporteGuardar,
    reporteEliminar,
  } from "$lib/components/print/printIpc";
  import { configPorDefecto } from "$lib/components/print/tipos";
  import type {
    ConfigReporte,
    ImagenesPorLinea,
    Orientacion,
    Papel,
    ReporteGuardado,
    TipoReporte,
  } from "$lib/components/print/tipos";
  import { registrosQuery } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { RegistroLigero } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  /** Tope de registros a traer para el reporte (todos los filtrados). */
  const LIMITE_REPORTE = 100000;

  // svelte-ignore state_referenced_locally
  let config = $state<ConfigReporte>(configInicial());
  let guardados = $state<ReporteGuardado[]>([]);
  let reporteSel = $state(""); // "" = (nuevo reporte)
  let nombreReporte = $state("");

  // Vista previa.
  let registros = $state<RegistroLigero[]>([]);
  let cargando = $state(false);
  let imprimiendo = $state(false);
  let zoom = $state(75);

  /** Config inicial: campos = visibles de la tabla activa, en orden. */
  function configInicial(): ConfigReporte {
    const base = configPorDefecto();
    base.campos = estado.camposVisibles
      .filter((c) => c.tabla === estado.tabla && c.tipo !== "multidato")
      .map((c) => c.nombre);
    return base;
  }

  // Campos disponibles para imprimir (no multidato, de la tabla activa).
  const camposDisponibles = $derived(
    estado.campos
      .filter((c) => c.tabla === estado.tabla && c.tipo !== "multidato")
      .slice()
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  // Definiciones de campos para PrintSheet.
  const camposDefs = $derived(estado.campos);

  const opcionesTipo: { valor: TipoReporte; etiqueta: string }[] = [
    { valor: "ci", etiqueta: t.reportes.conImagenes },
    { valor: "si", etiqueta: t.reportes.sinImagenes },
  ];

  const opcionesImgLinea: { valor: ImagenesPorLinea; etiqueta: string }[] = [
    { valor: 1, etiqueta: "1" },
    { valor: 2, etiqueta: "2" },
    { valor: 4, etiqueta: "4" },
    { valor: 8, etiqueta: "8" },
  ];

  const opcionesOrientacion: { valor: Orientacion; etiqueta: string }[] = [
    { valor: "vertical", etiqueta: t.reportes.vertical },
    { valor: "horizontal", etiqueta: t.reportes.horizontal },
  ];

  const opcionesPapel: { valor: Papel; etiqueta: string }[] = [
    { valor: "carta", etiqueta: t.reportes.carta },
    { valor: "oficio", etiqueta: t.reportes.oficio },
    { valor: "a4", etiqueta: t.reportes.a4 },
  ];

  const opcionesAgrupacion = $derived([
    { valor: "", etiqueta: t.reportes.sinAgrupacion },
    ...camposDisponibles.map((c) => ({ valor: c.nombre, etiqueta: c.nombre })),
  ]);

  // Selector de reporte guardado: "(nuevo reporte)" + guardados.
  const opcionesReporte = $derived([
    { valor: "", etiqueta: t.reportes.nuevoReporte },
    ...guardados.map((r) => ({ valor: r.nombre, etiqueta: r.nombre })),
  ]);

  $effect(() => {
    if (abierto) void cargarGuardados();
  });

  async function cargarGuardados(): Promise<void> {
    try {
      guardados = await reportesListar(estado.albumId);
    } catch {
      guardados = [];
    }
  }

  /** Carga la configuración de un reporte guardado al seleccionarlo. */
  function seleccionarReporte(nombre: string): void {
    reporteSel = nombre;
    if (nombre === "") return;
    const r = guardados.find((x) => x.nombre === nombre);
    if (r) {
      config = { ...configPorDefecto(), ...r.config };
      nombreReporte = r.nombre;
    }
  }

  /** Alterna la inclusión de un campo en el reporte, preservando el orden. */
  function alternarCampo(nombre: string): void {
    if (config.campos.includes(nombre)) {
      config.campos = config.campos.filter((c) => c !== nombre);
    } else {
      // Inserta respetando el orden de `camposDisponibles`.
      const orden = camposDisponibles.map((c) => c.nombre);
      const nuevo = [...config.campos, nombre].sort(
        (a, b) => orden.indexOf(a) - orden.indexOf(b),
      );
      config.campos = nuevo;
    }
  }

  function campoIncluido(nombre: string): boolean {
    return config.campos.includes(nombre);
  }

  async function cargarRegistros(): Promise<boolean> {
    cargando = true;
    try {
      const req = estado.construirQuery(0, LIMITE_REPORTE);
      const pagina = await registrosQuery(estado.albumId, req);
      registros = pagina.registros;
      return true;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.cargarRegistros);
      return false;
    } finally {
      cargando = false;
    }
  }

  async function vistaPrevia(): Promise<void> {
    await cargarRegistros();
  }

  async function imprimir(): Promise<void> {
    // Asegura que la hoja esté renderizada con los registros actuales.
    if (registros.length === 0) {
      const ok = await cargarRegistros();
      if (!ok) return;
    }
    imprimiendo = true;
    // Espera a que el DOM aplique la clase de impresión antes de print().
    await new Promise((r) => requestAnimationFrame(() => r(null)));
    window.print();
    imprimiendo = false;
  }

  async function guardar(): Promise<void> {
    const nombre = nombreReporte.trim();
    if (nombre === "") {
      ui.aviso(t.reportes.nombre);
      return;
    }
    try {
      await reporteGuardar(estado.albumId, nombre, $state.snapshot(config));
      ui.exito(t.mensaje.guardado);
      await cargarGuardados();
      reporteSel = nombre;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  async function eliminar(): Promise<void> {
    const nombre = reporteSel;
    if (nombre === "") return;
    try {
      await reporteEliminar(estado.albumId, nombre);
      ui.exito(t.mensaje.eliminado);
      reporteSel = "";
      nombreReporte = "";
      await cargarGuardados();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.reportes.titulo} ancho="xl" onCerrar={cerrar}>
  <div class="pr">
    <!-- Columna izquierda: configuración -->
    <div class="pr__form">
      <!-- Reporte guardado -->
      <div class="pr__grupo">
        <span class="pr__etq">{t.reportes.reporte}</span>
        <Select
          value={reporteSel}
          opciones={opcionesReporte}
          onCambio={seleccionarReporte}
        />
        <div class="pr__acciones-rep">
          <TextInput
            bind:value={nombreReporte}
            placeholder={t.reportes.nombre}
          />
          <Button variante="secundario" tamano="sm" onclick={guardar}>
            <Save size={14} />
            {t.reportes.guardarReporte}
          </Button>
          <Button
            variante="fantasma"
            tamano="sm"
            onclick={eliminar}
            disabled={reporteSel === ""}
          >
            <Trash2 size={14} />
            {t.reportes.eliminarReporte}
          </Button>
        </div>
      </div>

      <!-- Tipo y título -->
      <div class="pr__fila">
        <label class="pr__grupo">
          <span class="pr__etq">{t.reportes.tipo}</span>
          <Select bind:value={config.tipo} opciones={opcionesTipo} />
        </label>
        <label class="pr__grupo">
          <span class="pr__etq">{t.reportes.tituloDoc}</span>
          <TextInput
            bind:value={config.titulo}
            placeholder={estado.nombre}
          />
        </label>
      </div>

      <!-- Campos a imprimir -->
      <div class="pr__grupo">
        <span class="pr__etq">{t.reportes.camposIncluidos}</span>
        <div class="pr__campos">
          {#each camposDisponibles as c (c.id)}
            <label class="pr__check">
              <input
                type="checkbox"
                checked={campoIncluido(c.nombre)}
                onchange={() => alternarCampo(c.nombre)}
              />
              <span>{c.nombre}</span>
            </label>
          {/each}
        </div>
      </div>

      <!-- Presentación -->
      <div class="pr__fila">
        {#if config.tipo === "ci"}
          <label class="pr__grupo">
            <span class="pr__etq">{t.reportes.imagenesPorLinea}</span>
            <Select
              bind:value={config.imagenesPorLinea}
              opciones={opcionesImgLinea}
            />
          </label>
        {:else}
          <label class="pr__grupo">
            <span class="pr__etq">{t.reportes.agrupacion}</span>
            <Select
              value={config.agrupacion ?? ""}
              opciones={opcionesAgrupacion}
              onCambio={(v) => (config.agrupacion = v === "" ? null : v)}
            />
          </label>
        {/if}
        <label class="pr__grupo">
          <span class="pr__etq">{t.reportes.orientacion}</span>
          <Select
            bind:value={config.orientacion}
            opciones={opcionesOrientacion}
          />
        </label>
        <label class="pr__grupo">
          <span class="pr__etq">{t.reportes.tamanoPapel}</span>
          <Select bind:value={config.papel} opciones={opcionesPapel} />
        </label>
      </div>

      <!-- Opciones -->
      <div class="pr__opciones">
        <label class="pr__check">
          <input type="checkbox" bind:checked={config.ponFecha} />
          <span>{t.reportes.ponFecha}</span>
        </label>
        <label class="pr__check">
          <input type="checkbox" bind:checked={config.ponPagina} />
          <span>{t.reportes.ponPagina}</span>
        </label>
        <label class="pr__check">
          <input type="checkbox" bind:checked={config.ponTotales} />
          <span>{t.reportes.ponTotales}</span>
        </label>
      </div>
    </div>

    <!-- Columna derecha: vista previa -->
    <div class="pr__previa">
      <div class="pr__previa-barra">
        <Button variante="secundario" tamano="sm" onclick={vistaPrevia}>
          <Eye size={14} />
          {t.reportes.vistaPrevia}
        </Button>
        <div class="pr__zoom">
          {#each [50, 75, 100] as z (z)}
            <button
              type="button"
              class="pr__zoom-btn"
              class:pr__zoom-btn--activo={zoom === z}
              onclick={() => (zoom = z)}
            >
              {z}%
            </button>
          {/each}
        </div>
      </div>

      <div class="pr__lienzo">
        {#if cargando}
          <p class="pr__msg">{t.reportes.generando}</p>
        {:else if registros.length === 0}
          <p class="pr__msg">{t.reportes.vistaPrevia}</p>
        {:else}
          <div
            class="pr__escala"
            style:transform={`scale(${zoom / 100})`}
          >
            <PrintSheet
              {config}
              {registros}
              campos={camposDefs}
              albumId={estado.albumId}
              tabla={estado.tabla}
              nombreAlbum={estado.nombre}
              {imprimiendo}
            />
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#snippet pie()}
    <span class="pr__conteo">
      {registros.length}
      {t.reportes.registros}
    </span>
    <Button variante="fantasma" onclick={cerrar}>{t.accion.cerrar}</Button>
    <Button variante="primario" onclick={imprimir} cargando={cargando}>
      <Printer size={14} />
      {t.reportes.imprimir}
    </Button>
  {/snippet}
</Modal>

<style>
  .pr {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: var(--esp-4);
    min-height: 60vh;
  }
  .pr__form {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
    overflow-y: auto;
    max-height: 64vh;
    padding-right: var(--esp-2);
  }
  .pr__fila {
    display: flex;
    gap: var(--esp-2);
  }
  .pr__fila .pr__grupo {
    flex: 1;
    min-width: 0;
  }
  .pr__grupo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .pr__etq {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .pr__acciones-rep {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--esp-2);
    margin-top: var(--esp-1);
  }
  .pr__acciones-rep :global(.campo) {
    flex: 1;
    min-width: 120px;
  }
  .pr__campos {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px var(--esp-2);
    max-height: 160px;
    overflow-y: auto;
    padding: var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
  }
  .pr__opciones {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-3);
  }
  .pr__check {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
  }

  .pr__previa {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    min-width: 0;
  }
  .pr__previa-barra {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--esp-2);
  }
  .pr__zoom {
    display: inline-flex;
    gap: 2px;
  }
  .pr__zoom-btn {
    padding: 2px var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-xs);
    cursor: pointer;
  }
  .pr__zoom-btn--activo {
    background: var(--color-acento-tenue);
    color: var(--color-acento);
    border-color: var(--color-acento);
  }
  .pr__lienzo {
    flex: 1;
    overflow: auto;
    background: var(--color-panel);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    padding: var(--esp-3);
  }
  .pr__escala {
    transform-origin: top center;
  }
  .pr__msg {
    margin: 0;
    padding: var(--esp-6) 0;
    text-align: center;
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-sm);
  }
  .pr__conteo {
    margin-right: auto;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
</style>
