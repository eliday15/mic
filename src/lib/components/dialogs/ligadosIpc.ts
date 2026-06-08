/**
 * Wrappers tipados de `invoke()` para los comandos de álbumes ligados y la
 * suscripción al evento de progreso `liga-progreso`.
 *
 * Local al diálogo de ligados (no toca `src/lib/ipc/commands.ts` ni
 * `src/lib/ipc/events.ts`). Convención: comando snake_case, argumentos
 * camelCase (Tauri los convierte al deserializar).
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Una liga: sincroniza datos desde `rutaAlbum` usando el campo `llave`. */
export interface Liga {
  /** Id en la tabla `ligados`. `0` al crear (lo asigna el backend). */
  id: number;
  /** Ruta absoluta del álbum `.micdb` del que se copian los datos. */
  rutaAlbum: string;
  /** Nombre visible del campo llave común a ambos álbumes. */
  llave: string;
  /** Si la llave del ligado no existe en el actual, ¿dar de alta el registro? */
  crearFaltantes: boolean;
}

/** Resultado de actualizar una liga. */
export interface ResultadoLiga {
  actualizados: number;
  creados: number;
  sinCoincidencia: number;
}

/** Payload del evento `liga-progreso`. */
export interface LigaProgreso {
  hechas: number;
  total: number;
}

/** Lista las ligas definidas en el álbum. */
export function ligadosListar(albumId: number): Promise<Liga[]> {
  return invoke("ligados_listar", { albumId });
}

/** Guarda una liga (`id === 0` crea, en otro caso edita). Devuelve el id. */
export function ligaGuardar(albumId: number, liga: Liga): Promise<number> {
  return invoke("liga_guardar", { albumId, liga });
}

/** Elimina una liga por id. */
export function ligaEliminar(albumId: number, ligaId: number): Promise<void> {
  return invoke("liga_eliminar", { albumId, ligaId });
}

/** Actualiza una liga (sincroniza datos del álbum ligado al actual). */
export function ligaActualizar(
  albumId: number,
  ligaId: number,
): Promise<ResultadoLiga> {
  return invoke("liga_actualizar", { albumId, ligaId });
}

/** Actualiza todas las ligas del álbum. Devuelve un resultado por liga. */
export function ligasActualizarTodas(
  albumId: number,
): Promise<ResultadoLiga[]> {
  return invoke("ligas_actualizar_todas", { albumId });
}

/**
 * Escucha el progreso de actualización de una liga en curso.
 *
 * @param handler Callback con las filas hechas y el total.
 * @returns Promesa de la función para cancelar la suscripción.
 */
export function escucharLigaProgreso(
  handler: (p: LigaProgreso) => void,
): Promise<UnlistenFn> {
  return listen<LigaProgreso>("liga-progreso", (evento) =>
    handler(evento.payload),
  );
}
