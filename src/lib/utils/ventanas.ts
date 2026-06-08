/**
 * Caché de ventanas de registros para los virtualizadores (grilla y tabla).
 *
 * Pide al backend páginas alineadas (`registros_query` con offset/limit) y las
 * memoriza por `(versionConsulta, offset)`. Cuando cambia `versionConsulta` la
 * caché se descarta entera (los resultados ya no son válidos). Mantiene también
 * el total reportado por la última consulta.
 *
 * No es reactivo por sí mismo: el componente que lo usa fuerza el re-render
 * incrementando un contador propio tras cada carga.
 */

import { registrosQuery } from "$lib/ipc/commands";
import { ui } from "$lib/stores/ui.svelte";
import { t } from "$lib/i18n/es";
import type { AlbumState } from "$lib/stores/albumState.svelte";
import type { RegistroLigero } from "$lib/domain/types";

/** Tamaño de página solicitado al backend (alineado a múltiplos). */
export const TAM_PAGINA = 200;

/** Resultado de pedir un rango de filas. */
export interface RangoRegistros {
  /** Registros disponibles indexados por su índice absoluto. */
  porIndice: Map<number, RegistroLigero>;
  /** Total de registros de la consulta. */
  total: number;
}

/** Gestor de ventanas de datos para un álbum. */
export class CacheVentanas {
  private paginas = new Map<number, RegistroLigero[]>();
  private enVuelo = new Set<number>();
  private version = -1;
  private totalActual = 0;
  /** Última versión de consulta cuyo error ya se notificó (evita spam). */
  private versionConError = -1;

  constructor(
    private readonly estado: AlbumState,
    /** Callback que se invoca tras cargar una página (para re-render). */
    private readonly alCargar: () => void,
    /** Id del principal cuando se listan variantes. */
    private readonly idPrincipal?: number,
  ) {}

  /** Total conocido (de la última consulta). */
  get total(): number {
    return this.totalActual;
  }

  /** Invalida toda la caché si cambió la versión de consulta. */
  private sincronizarVersion(): void {
    if (this.version !== this.estado.versionConsulta) {
      this.version = this.estado.versionConsulta;
      this.paginas.clear();
      this.enVuelo.clear();
    }
  }

  /** Índice de página que contiene un índice absoluto de fila. */
  private paginaDe(indice: number): number {
    return Math.floor(indice / TAM_PAGINA);
  }

  /** Devuelve el registro en un índice absoluto, o `undefined` si no está. */
  obtener(indice: number): RegistroLigero | undefined {
    this.sincronizarVersion();
    const pag = this.paginaDe(indice);
    const datos = this.paginas.get(pag);
    if (!datos) return undefined;
    return datos[indice - pag * TAM_PAGINA];
  }

  /**
   * Garantiza que las páginas que cubren `[desde, hasta)` estén cargadas o en
   * curso de carga. Las que falten se piden de forma asíncrona.
   */
  asegurarRango(desde: number, hasta: number): void {
    this.sincronizarVersion();
    const primera = this.paginaDe(Math.max(0, desde));
    const ultima = this.paginaDe(Math.max(0, hasta - 1));
    for (let p = primera; p <= ultima; p++) {
      if (!this.paginas.has(p) && !this.enVuelo.has(p)) {
        this.cargarPagina(p);
      }
    }
  }

  /** Carga una página concreta del backend. */
  private async cargarPagina(pagina: number): Promise<void> {
    this.enVuelo.add(pagina);
    const versionPedida = this.version;
    const offset = pagina * TAM_PAGINA;
    try {
      const req = this.estado.construirQuery(
        offset,
        TAM_PAGINA,
        this.idPrincipal,
      );
      const page = await registrosQuery(this.estado.albumId, req);
      // Descarta si la versión cambió mientras esperábamos.
      if (versionPedida !== this.estado.versionConsulta) return;
      this.paginas.set(pagina, page.registros);
      this.totalActual = page.total;
      this.estado.setTotal(page.total);
      this.alCargar();
    } catch (e) {
      // Antes el fallo era silencioso y la grilla quedaba "pensando" en
      // esqueletos. Se notifica una vez por consulta; reintenta al re-scrollear.
      if (
        versionPedida === this.estado.versionConsulta &&
        this.versionConError !== versionPedida
      ) {
        this.versionConError = versionPedida;
        ui.error(typeof e === "string" ? e : t.error.cargarRegistros);
      }
    } finally {
      this.enVuelo.delete(pagina);
    }
  }

  /** Fuerza recarga total (p. ej. tras alta/baja de registros). */
  reiniciar(): void {
    this.version = -1;
    this.paginas.clear();
    this.enVuelo.clear();
  }

  /** Ids de todos los registros actualmente cargados en memoria. */
  idsCargados(): number[] {
    this.sincronizarVersion();
    const ids: number[] = [];
    for (const pagina of this.paginas.values()) {
      for (const reg of pagina) ids.push(reg.id);
    }
    return ids;
  }
}
