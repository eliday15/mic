/**
 * Wrappers tipados de `invoke()` para los comandos de importación de registros
 * (CSV/XLSX) y la suscripción al evento de progreso `importacion-progreso`.
 *
 * Local al diálogo de importación (no toca `src/lib/ipc/commands.ts` ni
 * `src/lib/ipc/events.ts`, igual que `ligadosIpc.ts`). Convención: comando
 * snake_case, argumentos camelCase (Tauri los convierte al deserializar).
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TipoCampo } from "$lib/domain/types";

/** Política de conflicto cuando la llave del archivo ya existe en el álbum. */
export type PoliticaImport = "sustituir" | "mantener" | "rellenar_vacios";

/** Una fila del archivo: número (base 1 contando el encabezado) y su llave. */
export interface FilaLlave {
  fila: number;
  llave: string;
}

/** Lista de filas: `total` completo; `filas` truncado a 500 de detalle. */
export interface ListaFilas {
  total: number;
  filas: FilaLlave[];
}

/** Columna del archivo sin campo en el álbum, con el tipo inferido. */
export interface ColumnaExtra {
  columna: string;
  tipo: TipoCampo;
  decimales: number;
}

/** Metadatos del archivo elegido, para poblar la fase de configuración. */
export interface InspeccionImport {
  /** Encabezados del archivo, en su orden original. */
  columnas: string[];
  /** Número de filas de datos (sin contar el encabezado). */
  totalFilas: number;
  /** Codificación detectada del CSV (los XLSX se reportan como "utf-8"). */
  encoding: "utf-8" | "utf-8-bom" | "windows-1252";
  /** Formato del archivo detectado por extensión/contenido. */
  formato: "csv" | "xlsx";
  /** Columnas que casan (sin distinguir mayúsculas) con un campo principal. */
  columnasReconocidas: string[];
  /** Columnas que no coinciden con ningún campo y se ignorarán. */
  columnasNoReconocidas: string[];
  /** Campos elegibles como llave (principales, no calculados ni multidato). */
  camposLlaveSugeridos: string[];
  /** Campos del álbum elegibles como llave, estén o no en el archivo. */
  camposLlaveAlbum: string[];
  /** Columnas extra con el tipo inferido (sin descartar aún la columna llave). */
  columnasExtra: ColumnaExtra[];
  /** Huella (largo+mtime) para validar que el archivo no cambió al aplicar. */
  huella: string;
}

/** Resultado de un análisis en seco (dryRun) o de una importación aplicada. */
export interface ResultadoImportacion {
  actualizados: number;
  creados: number;
  sinCambio: number;
  /** Filas cuya llave casó con un registro existente. */
  coincidentes: number;
  /** Filas cuya llave no casó con ningún registro. */
  noCoincidentes: ListaFilas;
  /** Filas sin coincidencia omitidas (sin `crearFaltantes`). */
  omitidos: number;
  /** Filas con la llave vacía (siempre se omiten). */
  llavesVacias: ListaFilas;
  /** Filas con llave repetida en el archivo (se usa la primera). */
  llavesDuplicadas: ListaFilas;
  /** Columnas extra detectadas, con su tipo inferido. */
  columnasExtra: ColumnaExtra[];
  /** Campos creados (o que se crearían, en dry-run) para las columnas extra. */
  camposCreados: string[];
  errores: string[];
  avisos: string[];
  /** `true` si fue un análisis previo (dry-run); `false` si se aplicó. */
  dryRun: boolean;
}

/** Payload del evento `importacion-progreso`. */
export interface ImportacionProgreso {
  fase: "analizando" | "aplicando";
  hechas: number;
  total: number;
}

/** Inspecciona el archivo: columnas, encoding, casamiento y llaves sugeridas. */
export function importarInspeccionar(
  albumId: number,
  rutaArchivo: string,
): Promise<InspeccionImport> {
  return invoke("importar_inspeccionar", { albumId, rutaArchivo });
}

/** Opciones de `importar_registros` (además del álbum y el archivo). */
export interface OpcionesImport {
  /** Campo del álbum que hace de llave. */
  campoLlave: string;
  /** Columna del archivo con la llave (default: la homónima del campo). */
  columnaLlave: string;
  politica: PoliticaImport;
  /** Dar de alta las filas cuya llave no coincide (si no, se omiten). */
  crearFaltantes: boolean;
  /** Crear campos para las columnas extra e importar sus valores. */
  importarExtras: boolean;
  dryRun: boolean;
  /** Huella de la inspección: el backend valida que el archivo no cambió. */
  huella?: string;
}

/**
 * Importa registros desde el archivo. Con `dryRun = true` recorre la misma
 * lógica en seco y devuelve el resumen previo sin escribir; con `dryRun = false`
 * aplica los cambios.
 */
export function importarRegistros(
  albumId: number,
  rutaArchivo: string,
  opciones: OpcionesImport,
): Promise<ResultadoImportacion> {
  return invoke("importar_registros", { albumId, rutaArchivo, ...opciones });
}

/**
 * Escucha el progreso de un análisis o importación en curso.
 *
 * @param handler Callback con la fase, las filas hechas y el total.
 * @returns Promesa de la función para cancelar la suscripción.
 */
export function escucharImportacionProgreso(
  handler: (p: ImportacionProgreso) => void,
): Promise<UnlistenFn> {
  return listen<ImportacionProgreso>("importacion-progreso", (evento) =>
    handler(evento.payload),
  );
}
