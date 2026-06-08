/**
 * Catálogo central de acciones de la aplicación, compartido por la barra de
 * menús, la barra de herramientas y la paleta de comandos. Cada acción se
 * identifica por un id estable y se ejecuta con `ejecutarAccion(id)`.
 *
 * Las acciones que abren diálogos modales del álbum activo lo hacen vía
 * `ui.abrirModal(id)` (los consume `AlbumView`). Las acciones que tocan la vista
 * del álbum (nueva imagen, eliminar, paneles) pasan por el store `vista`. Abrir
 * álbum / crear / migrar son globales.
 */

import { open, save } from "@tauri-apps/plugin-dialog";
import { albumes } from "$lib/stores/albums.svelte";
import { ui } from "$lib/stores/ui.svelte";
import { tema } from "$lib/stores/theme.svelte";
import { vista } from "$lib/stores/vista.svelte";
import {
  albumCompactar,
  albumRecalcular,
  albumEmpacar,
  albumDesempacar,
  registrosCrearDesdeCarpeta,
} from "$lib/ipc/commands";
import { t } from "$lib/i18n/es";

/** Abre el diálogo del sistema para elegir un álbum y lo abre. */
export async function abrirAlbumDialogo(): Promise<void> {
  const sel = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "MIC", extensions: ["micdb"] }],
  });
  if (typeof sel !== "string") return;
  try {
    await albumes.abrir(sel);
    ui.exito(t.mensaje.albumAbierto);
  } catch (e) {
    ui.error(typeof e === "string" ? e : t.error.cargarAlbum);
  }
}

/** Compacta el álbum activo. */
async function compactarActivo(): Promise<void> {
  const a = albumes.activo;
  if (!a) return;
  await ui.conBusy(async () => {
    try {
      await albumCompactar(a.albumId);
      ui.exito(t.mensaje.guardado);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  });
}

/** Recalcula los campos calculados del álbum activo (ex "Act. Calculados"). */
async function recalcularActivo(): Promise<void> {
  const a = albumes.activo;
  if (!a) return;
  await ui.conBusy(async () => {
    try {
      const n = await albumRecalcular(a.albumId);
      a.refrescar();
      ui.exito(`${n} ${t.mensaje.recalculados}`);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  });
}

/** Alta masiva de imágenes desde una carpeta (ex "Imagenes de Dir"). */
async function imagenesDesdeCarpeta(): Promise<void> {
  const a = albumes.activo;
  if (!a) return;
  const sel = await open({ directory: true, multiple: false });
  if (typeof sel !== "string") return;
  await ui.conBusy(async () => {
    try {
      const n = await registrosCrearDesdeCarpeta(a.albumId, sel);
      a.refrescar();
      ui.exito(`${n} ${t.mensaje.creado}`);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  });
}

/** Empaca el álbum activo en un `.zip` (ex EmpacarAlbum/frm3Botones). */
async function empacarActivo(): Promise<void> {
  const a = albumes.activo;
  if (!a) return;
  const sel = await save({
    defaultPath: `${a.nombre}.zip`,
    filters: [{ name: "ZIP", extensions: ["zip"] }],
  });
  if (typeof sel !== "string") return;
  await ui.conBusy(async () => {
    try {
      await albumEmpacar(a.albumId, sel);
      ui.exito(t.mensaje.empacado);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  });
}

/** Desempaca un `.zip` de álbum a un directorio y lo abre (ex DesempacarAlbum). */
async function desempacarDialogo(): Promise<void> {
  const zip = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "ZIP", extensions: ["zip"] }],
  });
  if (typeof zip !== "string") return;
  const destino = await open({ directory: true, multiple: false });
  if (typeof destino !== "string") return;
  await ui.conBusy(async () => {
    try {
      const ruta = await albumDesempacar(zip, destino);
      await albumes.abrir(ruta);
      ui.exito(t.mensaje.desempacado);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  });
}

/** Cierra el álbum activo. */
async function cerrarActivo(): Promise<void> {
  const a = albumes.activo;
  if (!a) return;
  try {
    await albumes.cerrar(a.albumId);
  } catch (e) {
    ui.error(typeof e === "string" ? e : t.error.generico);
  }
}

/**
 * Ejecuta una acción por su id. Devuelve `true` si el id fue reconocido.
 */
export function ejecutarAccion(id: string): boolean {
  const a = albumes.activo;
  switch (id) {
    // --- Globales ---
    case "nuevo-album":
      ui.abrirModal("nuevo-album");
      return true;
    case "abrir":
      void abrirAlbumDialogo();
      return true;
    case "migrar":
      ui.abrirModal("migrar");
      return true;
    case "compactar":
      void compactarActivo();
      return true;
    case "cerrar-album":
      void cerrarActivo();
      return true;

    // --- Edición (bus de vista) ---
    case "nueva-imagen":
      vista.nuevaImagen();
      return true;
    case "nueva-variante":
      vista.nuevaVariante();
      return true;
    case "editar-registro":
      vista.editar();
      return true;
    case "eliminar":
      vista.eliminar();
      return true;
    case "seleccionar-todo":
      vista.seleccionarTodo();
      return true;
    case "deseleccionar":
      a?.limpiarSeleccion();
      return true;
    case "invertir-seleccion":
      vista.invertirSeleccion();
      return true;
    case "ocultar":
      vista.ocultar();
      return true;
    case "mostrar":
      vista.mostrar();
      return true;

    // --- Vista ---
    case "vista-grilla":
      a?.setVista("grilla");
      return true;
    case "vista-tabla":
      a?.setVista("tabla");
      return true;
    case "panel-grupos":
      vista.alternarSidebar();
      return true;
    case "inspector":
      vista.alternarInspector();
      return true;
    case "campos-vista":
      if (a) ui.abrirModal("campos-vista");
      return true;
    case "mostrar-ocultos":
      a?.alternarOcultos();
      return true;
    case "visor":
      vista.abrirVisor();
      return true;
    case "tema":
      tema.alternar();
      return true;

    // --- Herramientas (modales del álbum) ---
    case "buscar":
      if (a) ui.abrirModal("buscar");
      return true;
    case "ordenar":
      if (a) ui.abrirModal("orden");
      return true;
    case "filtros":
      if (a) ui.abrirModal("filtros");
      return true;
    case "ligados":
      if (a) ui.abrirModal("ligados");
      return true;
    case "campos":
      if (a) ui.abrirModal("campos");
      return true;
    case "totalizar":
      if (a) ui.abrirModal("totalizar");
      return true;
    case "act-masiva":
      if (a) ui.abrirModal("act-masiva");
      return true;
    case "imagenes-carpeta":
      void imagenesDesdeCarpeta();
      return true;
    case "copiar-album":
      if (a) ui.abrirModal("copiar-album");
      return true;
    case "empacar":
      if (a) void empacarActivo();
      return true;
    case "desempacar":
      void desempacarDialogo();
      return true;
    case "exportar":
      if (a) ui.abrirModal("exportar");
      return true;
    case "importar-registros":
      if (a) ui.abrirModal("importar");
      return true;
    case "imprimir":
      if (a) ui.abrirModal("imprimir");
      return true;
    case "recalcular":
      void recalcularActivo();
      return true;

    // --- Ayuda ---
    case "acerca":
      ui.push(`${t.app.titulo} ${t.app.subtitulo}`, "info");
      return true;

    default:
      return false;
  }
}

/** Descriptor de una acción para la paleta de comandos. */
export interface AccionPaleta {
  id: string;
  etiqueta: string;
  grupo: string;
  /** Requiere álbum activo. */
  requiereAlbum?: boolean;
}

/** Lista de acciones para la paleta de comandos (⌘K). */
export function accionesPaleta(): AccionPaleta[] {
  return [
    { id: "nuevo-album", etiqueta: t.archivo.nuevoAlbum, grupo: t.menu.archivo },
    { id: "abrir", etiqueta: t.archivo.abrir, grupo: t.menu.archivo },
    { id: "migrar", etiqueta: t.archivo.importar, grupo: t.menu.archivo },
    { id: "imprimir", etiqueta: t.archivo.imprimir, grupo: t.menu.archivo, requiereAlbum: true },
    { id: "copiar-album", etiqueta: t.archivo.copiarAlbum, grupo: t.menu.archivo, requiereAlbum: true },
    { id: "empacar", etiqueta: t.archivo.empacar, grupo: t.menu.archivo, requiereAlbum: true },
    { id: "desempacar", etiqueta: t.archivo.desempacar, grupo: t.menu.archivo },
    { id: "exportar", etiqueta: t.archivo.exportar, grupo: t.menu.archivo, requiereAlbum: true },
    { id: "importar-registros", etiqueta: t.importacion.accionMenu, grupo: t.menu.archivo, requiereAlbum: true },
    { id: "compactar", etiqueta: t.archivo.compactar, grupo: t.menu.archivo, requiereAlbum: true },
    { id: "cerrar-album", etiqueta: t.archivo.cerrarAlbum, grupo: t.menu.archivo, requiereAlbum: true },

    { id: "nueva-imagen", etiqueta: t.editar.nuevaImagen, grupo: t.menu.editar, requiereAlbum: true },
    { id: "editar-registro", etiqueta: t.editar.editarRegistro, grupo: t.menu.editar, requiereAlbum: true },
    { id: "ocultar", etiqueta: t.editar.ocultar, grupo: t.menu.editar, requiereAlbum: true },
    { id: "mostrar", etiqueta: t.editar.mostrar, grupo: t.menu.editar, requiereAlbum: true },
    { id: "eliminar", etiqueta: t.editar.eliminar, grupo: t.menu.editar, requiereAlbum: true },
    { id: "seleccionar-todo", etiqueta: t.editar.seleccionarTodo, grupo: t.menu.editar, requiereAlbum: true },
    { id: "invertir-seleccion", etiqueta: t.editar.invertirSeleccion, grupo: t.menu.editar, requiereAlbum: true },

    { id: "vista-grilla", etiqueta: t.ver.grilla, grupo: t.menu.ver, requiereAlbum: true },
    { id: "vista-tabla", etiqueta: t.ver.tabla, grupo: t.menu.ver, requiereAlbum: true },
    { id: "panel-grupos", etiqueta: t.ver.panelGrupos, grupo: t.menu.ver, requiereAlbum: true },
    { id: "inspector", etiqueta: t.ver.inspector, grupo: t.menu.ver, requiereAlbum: true },
    { id: "campos-vista", etiqueta: t.ver.camposVista, grupo: t.menu.ver, requiereAlbum: true },
    { id: "mostrar-ocultos", etiqueta: t.ver.mostrarOcultos, grupo: t.menu.ver, requiereAlbum: true },
    { id: "visor", etiqueta: t.ver.visor, grupo: t.menu.ver, requiereAlbum: true },
    { id: "tema", etiqueta: t.ver.tema, grupo: t.menu.ver },

    { id: "buscar", etiqueta: t.herramientas.buscar, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "ordenar", etiqueta: t.herramientas.ordenar, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "filtros", etiqueta: t.herramientas.filtros, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "totalizar", etiqueta: t.herramientas.totalizar, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "act-masiva", etiqueta: t.herramientas.actMasiva, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "imagenes-carpeta", etiqueta: t.herramientas.imagenesCarpeta, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "recalcular", etiqueta: t.herramientas.recalcular, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "ligados", etiqueta: t.herramientas.ligados, grupo: t.menu.herramientas, requiereAlbum: true },
    { id: "campos", etiqueta: t.herramientas.campos, grupo: t.menu.herramientas, requiereAlbum: true },
  ];
}
