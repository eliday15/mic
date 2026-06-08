/**
 * Suscripciones tipadas a eventos emitidos por el backend (Tauri → frontend).
 *
 * Cada función devuelve una promesa de la función `unlisten` que cancela la
 * suscripción. Conviene guardarla y llamarla al desmontar el componente.
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AlbumCambiado, MigracionProgreso } from "$lib/domain/types";

/** Nombres de los eventos del backend (deben coincidir con Rust). */
export const EVENTOS = {
  migracionProgreso: "migracion-progreso",
  albumCambiado: "album-cambiado",
} as const;

/**
 * Escucha el progreso de una migración en curso.
 *
 * @param handler Callback con la fase, las filas hechas y el total.
 * @returns Promesa de la función para cancelar la suscripción.
 */
export function escucharMigracionProgreso(
  handler: (p: MigracionProgreso) => void,
): Promise<UnlistenFn> {
  return listen<MigracionProgreso>(EVENTOS.migracionProgreso, (evento) =>
    handler(evento.payload),
  );
}

/**
 * Escucha avisos de registros modificados fuera del flujo normal, para que la
 * grilla pueda refrescar las filas afectadas.
 *
 * @param handler Callback con el id del álbum y los ids de registros tocados.
 * @returns Promesa de la función para cancelar la suscripción.
 */
export function escucharAlbumCambiado(
  handler: (p: AlbumCambiado) => void,
): Promise<UnlistenFn> {
  return listen<AlbumCambiado>(EVENTOS.albumCambiado, (evento) =>
    handler(evento.payload),
  );
}

export type { UnlistenFn };
