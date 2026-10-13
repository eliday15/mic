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

/** Qué foto queda cuando el registro actual y el ligado tienen imagen. */
export type FuenteFotos = "actual" | "ligado";

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
  /**
   * Foto a conservar si ambos registros tienen imagen (si el actual no tiene,
   * la del ligado se importa siempre).
   */
  fotos: FuenteFotos;
  /** Crear aquí los campos que solo existen en el ligado y copiar sus valores. */
  importarCamposNuevos: boolean;
}

/** Resultado de actualizar una liga. */
export interface ResultadoLiga {
  actualizados: number;
  creados: number;
  sinCoincidencia: number;
  /** Fotos importadas del álbum ligado. */
  fotosCopiadas: number;
  /** Nombres de los campos creados en el álbum actual. */
  camposCreados: string[];
}

/** Comparación de estructuras entre el álbum actual y uno a ligar. */
export interface InspeccionLiga {
  /** Campos principales presentes en ambos (se sincronizan). */
  camposComunes: string[];
  /** Campos solo del ligado que se pueden crear aquí. */
  camposSoloLigado: string[];
  /** Campos solo del ligado que no se importan (calculados y multidatos). */
  camposNoImportables: string[];
  /** Registros del álbum ligado. */
  totalRegistros: number;
  /** Registros del álbum ligado con foto. */
  conImagen: number;
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

/** Compara la estructura del álbum actual con la del álbum `rutaAlbum`. */
export function ligaInspeccionar(
  albumId: number,
  rutaAlbum: string,
): Promise<InspeccionLiga> {
  return invoke("liga_inspeccionar", { albumId, rutaAlbum });
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
