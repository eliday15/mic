<!--
  AlbumView — vista principal de un álbum abierto. Compone:
    - SplitPane externo: AlbumSidebar (colapsable) | resto.
    - SplitPane interno: centro (grilla/tabla) | InspectorPanel (colapsable).
  Gestiona el drag&drop de archivos de imagen del sistema (vía el evento de
  webview) creando un registro por archivo (`registro_crear` con imagenOrigen).
  Aloja el RecordEditor y los diálogos de búsqueda/orden/filtros invocados desde
  el shell mediante `ui.modal`.
-->
<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { ImagePlus } from "lucide-svelte";
  import { SplitPane, ConfirmDialog } from "$lib/components/ui";
  import AlbumSidebar from "./AlbumSidebar.svelte";
  import InspectorPanel from "./InspectorPanel.svelte";
  import ThumbnailGrid from "$lib/components/grid/ThumbnailGrid.svelte";
  import TableView from "$lib/components/grid/TableView.svelte";
  import RecordEditor from "$lib/components/editor/RecordEditor.svelte";
  import SearchDialog from "$lib/components/dialogs/SearchDialog.svelte";
  import SortDialog from "$lib/components/dialogs/SortDialog.svelte";
  import AdvancedFiltersDialog from "$lib/components/dialogs/AdvancedFiltersDialog.svelte";
  import TotalizeDialog from "$lib/components/dialogs/TotalizeDialog.svelte";
  import BatchUpdateDialog from "$lib/components/dialogs/BatchUpdateDialog.svelte";
  import PrintDialog from "$lib/components/dialogs/PrintDialog.svelte";
  import CopyAlbumDialog from "$lib/components/dialogs/CopyAlbumDialog.svelte";
  import LinkedAlbumsDialog from "$lib/components/dialogs/LinkedAlbumsDialog.svelte";
  import ExportDialog from "$lib/components/dialogs/ExportDialog.svelte";
  import ImportarDialog from "$lib/components/dialogs/ImportarDialog.svelte";
  import FieldsVisibilityDialog from "$lib/components/dialogs/FieldsVisibilityDialog.svelte";
  import ImageViewer from "$lib/components/viewer/ImageViewer.svelte";
  import FieldStructureEditor from "$lib/components/structure/FieldStructureEditor.svelte";
  import {
    registroCrear,
    registrosEliminar,
    registrosSetAuxiliar,
  } from "$lib/ipc/commands";
  import { CacheVentanas } from "$lib/utils/ventanas";
  import { ui } from "$lib/stores/ui.svelte";
  import { vista } from "$lib/stores/vista.svelte";
  import { t } from "$lib/i18n/es";
  import type { UnlistenFn } from "$lib/ipc/events";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
  }

  let { estado }: Props = $props();

  // Caché de ventanas de datos compartida por grilla y tabla. Un contador
  // `cacheTick` se incrementa cuando carga páginas para forzar re-render.
  let cacheTick = $state(0);
  let cache = $derived.by(() => {
    void estado.albumId;
    return new CacheVentanas(estado, () => cacheTick++);
  });

  // Visibilidad de paneles laterales (compartida con el shell).
  const mostrarSidebar = $derived(vista.sidebarVisible);
  const mostrarInspector = $derived(vista.inspectorVisible);
  let anchoSidebar = $state(260);
  let anchoCentro = $state(640);

  // Editor de registro.
  let editorAbierto = $state(false);
  let editorId = $state<number | null>(null);
  // svelte-ignore state_referenced_locally
  let editorTabla = $state(estado.tabla);
  // Principal del que crear una variante (al abrir el editor en tabla 'variantes').
  let editorIdPrincipal = $state<number | null>(null);

  // Recargas del inspector tras cambios.
  let recargaInspector = $state(0);

  // Diálogos controlados por ui.modal pero scoped al álbum activo.
  const dlgBuscar = $derived(ui.esModalActivo("buscar"));
  const dlgOrden = $derived(ui.esModalActivo("orden"));
  const dlgFiltros = $derived(ui.esModalActivo("filtros"));
  const dlgCampos = $derived(ui.esModalActivo("campos"));
  const dlgTotalizar = $derived(ui.esModalActivo("totalizar"));
  const dlgActMasiva = $derived(ui.esModalActivo("act-masiva"));
  const dlgImprimir = $derived(ui.esModalActivo("imprimir"));
  const dlgCopiar = $derived(ui.esModalActivo("copiar-album"));
  const dlgLigados = $derived(ui.esModalActivo("ligados"));
  const dlgExportar = $derived(ui.esModalActivo("exportar"));
  const dlgImportar = $derived(ui.esModalActivo("importar"));
  const dlgCamposVista = $derived(ui.esModalActivo("campos-vista"));

  // Visor de imagen al 100 %.
  let visorAbierto = $state(false);
  let visorInicial = $state(0);

  // Confirmación de borrado.
  let confirmarBorrado = $state(false);
  let borrando = $state(false);

  // Drop de archivos del SO.
  let sobreDrop = $state(false);
  let unlistenDrop: UnlistenFn | null = null;

  const EXT_IMG = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"];

  function esImagen(ruta: string): boolean {
    const punto = ruta.lastIndexOf(".");
    if (punto === -1) return false;
    return EXT_IMG.includes(ruta.slice(punto + 1).toLowerCase());
  }

  $effect(() => {
    // En modo navegador (mock) no hay webview de Tauri: sin drag&drop nativo.
    if ((window as { __MIC_MOCK__?: boolean }).__MIC_MOCK__) return;
    let activo = true;
    (async () => {
      const webview = getCurrentWebview();
      const un = await webview.onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === "enter" || p.type === "over") {
          sobreDrop = true;
        } else if (p.type === "leave") {
          sobreDrop = false;
        } else if (p.type === "drop") {
          sobreDrop = false;
          crearDesdeArchivos(p.paths);
        }
      });
      if (activo) unlistenDrop = un;
      else un();
    })();
    return () => {
      activo = false;
      unlistenDrop?.();
      unlistenDrop = null;
    };
  });

  async function crearDesdeArchivos(rutas: string[]): Promise<void> {
    const imagenes = rutas.filter(esImagen);
    if (imagenes.length === 0) return;
    await ui.conBusy(async () => {
      let creados = 0;
      for (const ruta of imagenes) {
        try {
          await registroCrear(estado.albumId, "principal", {}, {}, ruta);
          creados++;
        } catch (e) {
          ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
        }
      }
      if (creados > 0) {
        estado.refrescar();
        ui.exito(`${creados} ${t.mensaje.creado}`);
      }
    });
  }

  // --- Acciones del editor / grilla -------------------------------------
  function abrirEditor(id: number | null): void {
    editorId = id;
    editorTabla = estado.tabla;
    editorIdPrincipal = null;
    editorAbierto = true;
  }

  function onGuardadoEditor(): void {
    estado.refrescar();
    recargaInspector++;
  }

  function abrirVisor(id?: number): void {
    const inicial = id ?? [...estado.seleccion][0] ?? cache.idsCargados()[0];
    if (inicial === undefined) {
      ui.aviso(t.mensaje.sinSeleccion);
      return;
    }
    visorInicial = inicial;
    visorAbierto = true;
  }

  /** Oculta (o vuelve a mostrar) la selección actual sin eliminarla. */
  async function setOcultos(oculto: boolean): Promise<void> {
    const ids = [...estado.seleccion];
    if (ids.length === 0) {
      ui.aviso(t.mensaje.sinSeleccion);
      return;
    }
    try {
      await registrosSetAuxiliar(estado.albumId, ids, estado.tabla, oculto);
      estado.limpiarSeleccion();
      estado.refrescar();
      ui.exito(oculto ? t.mensaje.ocultados : t.mensaje.mostrados);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  function accionGrilla(accion: string, id: number): void {
    switch (accion) {
      case "editar":
        abrirEditor(id);
        break;
      case "ver100":
        abrirVisor(id);
        break;
      case "ocultar":
        estado.seleccionarUno(id);
        void setOcultos(true);
        break;
      case "mostrar":
        estado.seleccionarUno(id);
        void setOcultos(false);
        break;
      case "variantes":
        abrirEditor(id);
        break;
      case "eliminar":
        confirmarBorrado = true;
        break;
    }
  }

  async function eliminarSeleccion(): Promise<void> {
    const ids = [...estado.seleccion];
    if (ids.length === 0) {
      confirmarBorrado = false;
      return;
    }
    borrando = true;
    try {
      await registrosEliminar(estado.albumId, ids, estado.tabla);
      estado.limpiarSeleccion();
      estado.refrescar();
      ui.exito(t.mensaje.eliminado);
      confirmarBorrado = false;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.eliminarRegistro);
    } finally {
      borrando = false;
    }
  }

  // --- Observadores del bus de acciones del shell -----------------------
  // Cada señal es un contador; al incrementarse, ejecutamos la acción. Se
  // ignora el valor inicial montando una marca por señal.
  let ultimaNueva = vista.senalNuevaImagen;
  $effect(() => {
    if (vista.senalNuevaImagen !== ultimaNueva) {
      ultimaNueva = vista.senalNuevaImagen;
      abrirEditor(null);
    }
  });

  let ultimaVariante = vista.senalNuevaVariante;
  $effect(() => {
    if (vista.senalNuevaVariante !== ultimaVariante) {
      ultimaVariante = vista.senalNuevaVariante;
      const id = [...estado.seleccion][0];
      if (id !== undefined) {
        // Captura de una variante nueva del registro seleccionado.
        editorId = null;
        editorTabla = "variantes";
        editorIdPrincipal = id;
        editorAbierto = true;
      } else {
        ui.aviso(t.mensaje.sinSeleccion);
      }
    }
  });

  let ultimaEditar = vista.senalEditar;
  $effect(() => {
    if (vista.senalEditar !== ultimaEditar) {
      ultimaEditar = vista.senalEditar;
      const id = [...estado.seleccion][0];
      if (id !== undefined) abrirEditor(id);
      else ui.aviso(t.mensaje.sinSeleccion);
    }
  });

  let ultimaEliminar = vista.senalEliminar;
  $effect(() => {
    if (vista.senalEliminar !== ultimaEliminar) {
      ultimaEliminar = vista.senalEliminar;
      if (estado.seleccion.size > 0) confirmarBorrado = true;
      else ui.aviso(t.mensaje.sinSeleccion);
    }
  });

  // "Seleccionar todo" opera sobre las páginas ya cargadas en memoria (las que
  // el usuario ha desplazado). Para grandes volúmenes el backend serviría una
  // consulta de ids; en v1 se selecciona el conjunto disponible y se avisa si
  // pudiera estar incompleto.
  let ultimaSelTodo = vista.senalSeleccionarTodo;
  $effect(() => {
    if (vista.senalSeleccionarTodo !== ultimaSelTodo) {
      ultimaSelTodo = vista.senalSeleccionarTodo;
      const ids = cache.idsCargados();
      estado.seleccionarVarios(ids);
      if (ids.length < estado.total) {
        ui.aviso(t.registro.seleccionParcial(ids.length, estado.total));
      }
    }
  });

  // "Invertir selección": entre los registros ya cargados, los no
  // seleccionados pasan a estarlo y viceversa. Mismo aviso de parcialidad.
  let ultimaInvertir = vista.senalInvertirSeleccion;
  $effect(() => {
    if (vista.senalInvertirSeleccion !== ultimaInvertir) {
      ultimaInvertir = vista.senalInvertirSeleccion;
      const ids = cache.idsCargados();
      const invertidos = ids.filter((id) => !estado.seleccion.has(id));
      estado.seleccionarVarios(invertidos);
      if (ids.length < estado.total) {
        ui.aviso(t.registro.seleccionParcial(invertidos.length, estado.total));
      }
    }
  });

  let ultimaOcultar = vista.senalOcultar;
  $effect(() => {
    if (vista.senalOcultar !== ultimaOcultar) {
      ultimaOcultar = vista.senalOcultar;
      void setOcultos(true);
    }
  });

  let ultimaMostrar = vista.senalMostrar;
  $effect(() => {
    if (vista.senalMostrar !== ultimaMostrar) {
      ultimaMostrar = vista.senalMostrar;
      void setOcultos(false);
    }
  });

  let ultimaVisor = vista.senalVisor;
  $effect(() => {
    if (vista.senalVisor !== ultimaVisor) {
      ultimaVisor = vista.senalVisor;
      abrirVisor();
    }
  });

  // El inspector ocultado con Esc reaparece solo al volver a seleccionar.
  let inspectorOcultoPorEsc = $state(false);

  $effect(() => {
    if (estado.seleccion.size > 0 && inspectorOcultoPorEsc) {
      inspectorOcultoPorEsc = false;
      vista.inspectorVisible = true;
    }
  });

  /**
   * Escape global de la vista, en cascada: 1) suelta el campo enfocado
   * (inspector / tabla); 2) limpia la selección; 3) oculta el inspector vacío
   * para ver la grilla entera (reaparece al seleccionar de nuevo). Los
   * overlays (modales, visor, menús) gestionan su propio Escape.
   */
  function onEscVista(e: KeyboardEvent): void {
    if (e.key !== "Escape") return;
    if (
      editorAbierto ||
      visorAbierto ||
      confirmarBorrado ||
      ui.modal !== null ||
      document.querySelector("[role=menu]") !== null
    ) {
      return;
    }
    const el = document.activeElement as HTMLElement | null;
    if (
      el &&
      (el.tagName === "INPUT" ||
        el.tagName === "TEXTAREA" ||
        el.tagName === "SELECT")
    ) {
      e.preventDefault();
      el.blur();
      return;
    }
    if (estado.seleccion.size > 0) {
      e.preventDefault();
      estado.limpiarSeleccion();
      return;
    }
    if (vista.inspectorVisible) {
      e.preventDefault();
      inspectorOcultoPorEsc = true;
      vista.inspectorVisible = false;
    }
  }
</script>

<svelte:window onkeydown={onEscVista} />

{#snippet contenidoCentral()}
  <div class="av__contenido">
    {#if estado.vista === "grilla"}
      <ThumbnailGrid
        {estado}
        {cache}
        tick={cacheTick}
        onAbrir={(id) => abrirEditor(id)}
        onAccion={accionGrilla}
        onRefrescar={() => {
          cacheTick++;
          recargaInspector++;
        }}
      />
    {:else}
      <TableView
        {estado}
        {cache}
        tick={cacheTick}
        onRefrescar={() => cacheTick++}
        onAbrir={(id) => abrirEditor(id)}
      />
    {/if}

    {#if sobreDrop}
      <div class="av__drop">
        <ImagePlus size={36} />
        <span>{t.registro.elegirImagen}</span>
      </div>
    {/if}
  </div>
{/snippet}

<!-- El SplitPane del inspector solo existe cuando el panel está visible: al
     ocultarlo (Esc / Ver → Inspector) la grilla ocupa todo el ancho. -->
{#snippet centroConInspector()}
  <div class="av__centro">
    {#if mostrarInspector}
      <SplitPane
        orientacion="horizontal"
        bind:tamano={anchoCentro}
        min={360}
        max={1400}
      >
        {#snippet primero()}
          {@render contenidoCentral()}
        {/snippet}
        {#snippet segundo()}
          <InspectorPanel
            {estado}
            recarga={recargaInspector}
            onAbrirEditor={(id) => abrirEditor(id)}
          />
        {/snippet}
      </SplitPane>
    {:else}
      {@render contenidoCentral()}
    {/if}
  </div>
{/snippet}

<div class="av">
  {#if mostrarSidebar}
    <SplitPane
      orientacion="horizontal"
      bind:tamano={anchoSidebar}
      min={200}
      max={420}
    >
      {#snippet primero()}
        <AlbumSidebar {estado} onAvanzados={() => ui.abrirModal("filtros")} />
      {/snippet}
      {#snippet segundo()}
        {@render centroConInspector()}
      {/snippet}
    </SplitPane>
  {:else}
    {@render centroConInspector()}
  {/if}
</div>

<!-- Editor de registro -->
{#if editorAbierto}
  <RecordEditor
    bind:abierto={editorAbierto}
    albumId={estado.albumId}
    campos={estado.campos}
    id={editorId}
    tabla={editorTabla}
    idPrincipal={editorIdPrincipal}
    onGuardado={onGuardadoEditor}
  />
{/if}

<!-- Diálogos del álbum -->
{#if dlgBuscar}
  <SearchDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgOrden}
  <SortDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgFiltros}
  <AdvancedFiltersDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgCampos}
  <FieldStructureEditor {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgTotalizar}
  <TotalizeDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgActMasiva}
  <BatchUpdateDialog
    {estado}
    abierto
    onAplicado={() => {
      estado.refrescar();
      recargaInspector++;
    }}
    onCerrar={() => ui.cerrarModal()}
  />
{/if}
{#if dlgImprimir}
  <PrintDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgCopiar}
  <CopyAlbumDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgLigados}
  <LinkedAlbumsDialog
    {estado}
    abierto
    onAplicado={() => estado.refrescar()}
    onCerrar={() => ui.cerrarModal()}
  />
{/if}
{#if dlgExportar}
  <ExportDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgImportar}
  <ImportarDialog
    {estado}
    abierto
    onAplicado={() => estado.refrescar()}
    onCerrar={() => ui.cerrarModal()}
  />
{/if}
{#if dlgCamposVista}
  <FieldsVisibilityDialog {estado} abierto onCerrar={() => ui.cerrarModal()} />
{/if}

<!-- Visor de imagen al 100 % -->
{#if visorAbierto}
  <ImageViewer
    bind:abierto={visorAbierto}
    albumId={estado.albumId}
    tabla={estado.tabla}
    ids={cache.idsCargados()}
    inicial={visorInicial}
  />
{/if}

{#if confirmarBorrado}
  <ConfirmDialog
    bind:abierto={confirmarBorrado}
    titulo={t.confirmar.eliminarRegistros}
    mensaje={t.confirmar.eliminarRegistrosDesc}
    textoConfirmar={t.accion.eliminar}
    peligro
    cargando={borrando}
    onConfirmar={eliminarSeleccion}
  />
{/if}

<style>
  .av {
    height: 100%;
    width: 100%;
  }
  .av__centro {
    height: 100%;
  }
  .av__contenido {
    position: relative;
    height: 100%;
  }
  .av__drop {
    position: absolute;
    inset: var(--esp-3);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--esp-2);
    border: 2px dashed var(--color-acento);
    border-radius: var(--radio-lg);
    background: var(--color-acento-tenue);
    color: var(--color-acento);
    font-weight: 600;
    pointer-events: none;
    z-index: var(--z-sticky);
  }
</style>
