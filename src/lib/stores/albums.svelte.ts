/**
 * Store de los álbumes abiertos como pestañas.
 *
 * Mantiene la lista de estados de álbum (uno por pestaña) y el id del álbum
 * activo. Abrir/crear delega en los comandos IPC; cerrar invoca `albumCerrar`
 * y descarta el estado local. El estado por álbum vive en `createAlbumState`.
 */

import { albumAbrir, albumCerrar, albumCrear } from "$lib/ipc/commands";
import type { AlbumInfo, CampoNuevo } from "$lib/domain/types";
import { createAlbumState, type AlbumState } from "./albumState.svelte";

class StoreAlbumes {
  /** Estados de los álbumes abiertos, en orden de pestaña. */
  abiertos = $state<AlbumState[]>([]);
  /** Id del álbum activo (`null` si no hay ninguno abierto). */
  activoId = $state<number | null>(null);

  /** Estado del álbum activo, o `null`. */
  get activo(): AlbumState | null {
    if (this.activoId === null) return null;
    return this.abiertos.find((a) => a.albumId === this.activoId) ?? null;
  }

  /** True si hay al menos un álbum abierto. */
  get hayAlbumes(): boolean {
    return this.abiertos.length > 0;
  }

  /** Busca el estado de un álbum por id. */
  porId(albumId: number): AlbumState | null {
    return this.abiertos.find((a) => a.albumId === albumId) ?? null;
  }

  /** True si el álbum (por ruta) ya está abierto en una pestaña. */
  private porRuta(ruta: string): AlbumState | null {
    return this.abiertos.find((a) => a.ruta === ruta) ?? null;
  }

  /**
   * Inserta el estado de un álbum (o activa el existente si la ruta coincide).
   */
  private adoptar(info: AlbumInfo): AlbumState {
    const existente = this.porRuta(info.ruta);
    if (existente) {
      this.activoId = existente.albumId;
      return existente;
    }
    const estado = createAlbumState(info);
    this.abiertos = [...this.abiertos, estado];
    this.activoId = estado.albumId;
    return estado;
  }

  /**
   * Abre un álbum existente desde una ruta. Si ya está abierto, lo activa.
   *
   * @param ruta Ruta del archivo de álbum.
   * @returns El estado del álbum abierto.
   */
  async abrir(ruta: string): Promise<AlbumState> {
    const info = await albumAbrir(ruta);
    return this.adoptar(info);
  }

  /**
   * Crea un álbum nuevo y lo abre como pestaña activa.
   *
   * @param ruta   Ruta de destino del nuevo álbum.
   * @param nombre Nombre del álbum.
   * @param campos Campos iniciales.
   * @returns El estado del álbum creado.
   */
  async crear(
    ruta: string,
    nombre: string,
    campos: CampoNuevo[],
  ): Promise<AlbumState> {
    const info = await albumCrear(ruta, nombre, campos);
    return this.adoptar(info);
  }

  /**
   * Cierra un álbum: invoca al backend y descarta su estado local. Si era el
   * activo, pasa el foco a la pestaña vecina.
   *
   * @param albumId Id del álbum a cerrar.
   */
  async cerrar(albumId: number): Promise<void> {
    const indice = this.abiertos.findIndex((a) => a.albumId === albumId);
    if (indice === -1) return;

    await albumCerrar(albumId);

    const eraActivo = this.activoId === albumId;
    this.abiertos = this.abiertos.filter((a) => a.albumId !== albumId);

    if (eraActivo) {
      const vecino =
        this.abiertos[indice] ?? this.abiertos[indice - 1] ?? null;
      this.activoId = vecino ? vecino.albumId : null;
    }
  }

  /**
   * Activa una pestaña de álbum ya abierto.
   *
   * @param albumId Id del álbum a activar.
   */
  activar(albumId: number): void {
    if (this.abiertos.some((a) => a.albumId === albumId)) {
      this.activoId = albumId;
    }
  }
}

/** Instancia única del store de álbumes. */
export const albumes = new StoreAlbumes();
