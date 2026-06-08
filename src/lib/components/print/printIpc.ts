/**
 * Wrappers tipados de `invoke()` para los comandos de reportes (impresión).
 *
 * Se mantienen aparte de `$lib/ipc/commands` porque el sistema de impresión es
 * un módulo autocontenido. Convención idéntica al resto: nombre de comando en
 * snake_case, argumentos en camelCase (Tauri los convierte a snake_case).
 */

import { invoke } from "@tauri-apps/api/core";
import type { ConfigReporte, ReporteGuardado } from "./tipos";

/** Lista los reportes guardados del álbum (orden alfabético por nombre). */
export function reportesListar(albumId: number): Promise<ReporteGuardado[]> {
  return invoke("reportes_listar", { albumId });
}

/** Guarda (upsert por nombre) la configuración de un reporte. */
export function reporteGuardar(
  albumId: number,
  nombre: string,
  config: ConfigReporte,
): Promise<void> {
  return invoke("reporte_guardar", { albumId, nombre, config });
}

/** Elimina un reporte guardado por nombre. */
export function reporteEliminar(albumId: number, nombre: string): Promise<void> {
  return invoke("reporte_eliminar", { albumId, nombre });
}
