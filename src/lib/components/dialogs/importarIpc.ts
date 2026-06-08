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

/** Política de conflicto cuando la llave del archivo ya existe en el álbum. */
export type PoliticaImport = "sustituir" | "mantener" | "rellenar_vacios";

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
  /** Huella (largo+mtime) para validar que el archivo no cambió al aplicar. */
  huella: string;
}

/** Resultado de un análisis en seco (dryRun) o de una importación aplicada. */
export interface ResultadoImportacion {
  actualizados: number;
  creados: number;
  sinCambio: number;
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

/**
 * Importa registros desde el archivo. Con `dryRun = true` recorre la misma
 * lógica en seco y devuelve el resumen previo sin escribir; con `dryRun = false`
 * aplica los cambios. La `huella` (de la inspección) permite al backend validar
 * que el archivo no se modificó entre el resumen y la aplicación.
 */
export function importarRegistros(
  albumId: number,
  rutaArchivo: string,
  campoLlave: string,
  politica: PoliticaImport,
  crearFaltantes: boolean,
  dryRun: boolean,
  huella?: string,
): Promise<ResultadoImportacion> {
  return invoke("importar_registros", {
    albumId,
    rutaArchivo,
    campoLlave,
    politica,
    crearFaltantes,
    dryRun,
    huella,
  });
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
