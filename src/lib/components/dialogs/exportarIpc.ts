/**
 * Wrapper tipado del comando `exportar_registros` (ex-frmExp del VB6).
 *
 * Aislado en el módulo del diálogo (no en `src/lib/ipc/commands.ts`) porque la
 * exportación es una funcionalidad acoplada a su UI. Convención de invoke: el
 * nombre del comando va en snake_case; los argumentos en camelCase.
 */

import { invoke } from "@tauri-apps/api/core";
import type { QueryReq } from "$lib/domain/types";

/**
 * Formato de salida de la exportación.
 *
 * `csv-mic` genera un CSV que el "Importar..." del MIC clásico (VB6) puede
 * leer: Windows-1252 sin BOM, sin comillas, primera columna = campo llave.
 */
export type FormatoExport = "csv" | "xlsx" | "csv-mic";

/**
 * Exporta el conjunto filtrado actual (según `req`) a `rutaDestino`.
 *
 * @param albumId Id de sesión del álbum.
 * @param req Petición de consulta (filtro/orden); el backend ignora offset/limit.
 * @param campos Nombres visibles de los campos a exportar, en ese orden.
 * @param formato "csv" | "xlsx".
 * @param rutaDestino Ruta absoluta del archivo de salida.
 * @returns Número de registros exportados.
 */
export function exportarRegistros(
  albumId: number,
  req: QueryReq,
  campos: string[],
  formato: FormatoExport,
  rutaDestino: string,
): Promise<number> {
  return invoke("exportar_registros", {
    albumId,
    req,
    campos,
    formato,
    rutaDestino,
  });
}
