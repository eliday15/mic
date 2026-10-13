/**
 * Estado reactivo por álbum abierto.
 *
 * `createAlbumState(info)` produce un objeto de estado independiente para cada
 * pestaña de álbum: campos, filtros activos (rápido + condiciones apilables +
 * exclusiones manuales),
 * búsqueda, orden (hasta 3 niveles), grupo seleccionado, selección de
 * registros, vista (grilla/tabla), zoom y bandera `dirty`.
 *
 * Un contador `versionConsulta` se incrementa con cualquier cambio que afecte
 * a los resultados (filtro, orden, grupo, búsqueda). La grilla lo observa para
 * decidir cuándo recargar la ventana de registros.
 */

import { SvelteSet } from "svelte/reactivity";
import type {
  AlbumInfo,
  CampoDef,
  CondicionFiltro,
  FiltroRapido,
  OpRel,
  OrdenCampo,
  QueryReq,
  SeleccionGrupo,
  Tabla,
} from "$lib/domain/types";

/** Modo de visualización de los registros. */
export type Vista = "grilla" | "tabla";

/** Límites del zoom de celda (tamaño en píxeles del lado mayor). */
export const ZOOM_MIN = 90;
export const ZOOM_MAX = 420;
const ZOOM_DEFECTO = 180;
const ZOOM_PASO = 30;

/** Número máximo de niveles de orden (como frmOrdenar del original). */
export const MAX_ORDEN = 3;

/**
 * Garantiza la forma de una cadena de condiciones: la primera sin conector y
 * las demás con Y/O (Y si no tenían). Se aplica tras agregar, quitar o mover.
 */
function normalizarConectores(lista: CondicionFiltro[]): CondicionFiltro[] {
  return lista.map((c, i) => {
    const opRel: OpRel | null = i === 0 ? null : (c.opRel ?? "y");
    return c.opRel === opRel ? c : { ...c, opRel };
  });
}

/** Tipo del estado de un álbum (lo que devuelve la factory). */
export type AlbumState = ReturnType<typeof createAlbumState>;

/**
 * Crea el estado reactivo de un álbum a partir de su información de sesión.
 *
 * @param info Información del álbum devuelta por `albumAbrir`/`albumCrear`.
 */
export function createAlbumState(info: AlbumInfo) {
  // --- Estado base -------------------------------------------------------
  const albumId = info.albumId;
  let nombre = $state(info.nombre);
  let ruta = $state(info.ruta);
  let campos = $state<CampoDef[]>(info.campos);
  let tieneVariantes = $state(info.tieneVariantes);

  // --- Consulta activa ---------------------------------------------------
  let tabla = $state<Tabla>("principal");
  let busqueda = $state("");
  let filtroRapido = $state<FiltroRapido | null>(null);
  let condiciones = $state<CondicionFiltro[]>([]);
  let orden = $state<OrdenCampo[]>([]);
  let grupoSel = $state<SeleccionGrupo | null>(null);
  let incluirOcultos = $state(false);
  /** Ids quitados a mano del resultado ("Quitar del filtro"), en orden de quita. */
  let excluidos = $state<number[]>([]);

  // --- Selección y presentación -----------------------------------------
  const seleccion = new SvelteSet<number>();
  let total = $state(info.totalRegistros);
  /** Rango de registros visible en la vista (1-based), para la status bar. */
  let rangoVisible = $state<[number, number] | null>(null);
  /** Columnas fijas de la grilla (ex 8/4/2 por línea); null = auto por zoom. */
  let columnasFijas = $state<number | null>(null);
  /** La letra de las cards escala con el zoom (true) o queda fija (false). */
  let letraEscala = $state(true);
  let vista = $state<Vista>("grilla");
  let zoom = $state(ZOOM_DEFECTO);
  let dirty = $state(false);

  // --- Versionado de consulta -------------------------------------------
  let versionConsulta = $state(0);
  function marcarConsulta(): void {
    versionConsulta++;
  }

  // Versión de la lista de filtros guardados: se incrementa al guardar o
  // eliminar un filtro para que el panel lateral recargue la lista.
  let versionFiltros = $state(0);

  // --- Campos visibles ordenados ----------------------------------------
  // Solo los de la tabla mostrada: en la vista principal no deben aparecer
  // columnas de variantes (y viceversa).
  const camposVisibles = $derived(
    campos
      .filter((c) => c.visible && c.tabla === tabla)
      .slice()
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  /** True si hay algún filtro activo (rápido, condiciones, búsqueda o grupo). */
  const hayFiltros = $derived(
    filtroRapido !== null ||
      condiciones.length > 0 ||
      excluidos.length > 0 ||
      busqueda.trim() !== "" ||
      grupoSel !== null,
  );

  return {
    // ---- Identidad / lectura ----
    get albumId() {
      return albumId;
    },
    get nombre() {
      return nombre;
    },
    set nombre(v: string) {
      nombre = v;
    },
    get ruta() {
      return ruta;
    },
    set ruta(v: string) {
      ruta = v;
    },
    get campos() {
      return campos;
    },
    get camposVisibles() {
      return camposVisibles;
    },
    get tieneVariantes() {
      return tieneVariantes;
    },
    set tieneVariantes(v: boolean) {
      tieneVariantes = v;
    },

    // ---- Consulta (lectura) ----
    get tabla() {
      return tabla;
    },
    get busqueda() {
      return busqueda;
    },
    get filtroRapido() {
      return filtroRapido;
    },
    get condiciones() {
      return condiciones;
    },
    get orden() {
      return orden;
    },
    get grupoSel() {
      return grupoSel;
    },
    get hayFiltros() {
      return hayFiltros;
    },
    get incluirOcultos() {
      return incluirOcultos;
    },
    get excluidos() {
      return excluidos;
    },

    // ---- Presentación / selección (lectura) ----
    get seleccion() {
      return seleccion;
    },
    get total() {
      return total;
    },
    get vista() {
      return vista;
    },
    get zoom() {
      return zoom;
    },
    get dirty() {
      return dirty;
    },
    get versionConsulta() {
      return versionConsulta;
    },
    get versionFiltros() {
      return versionFiltros;
    },

    // ---- Mutaciones de estructura ----
    /** Reemplaza la lista de campos (tras crear/editar/reordenar). */
    setCampos(nuevos: CampoDef[]): void {
      campos = nuevos;
    },

    /** Actualiza el total tras una consulta o una alta/baja. */
    setTotal(n: number): void {
      total = n;
    },

    get rangoVisible() {
      return rangoVisible;
    },

    /** Publica el rango visible en la vista (null = sin registros). */
    setRangoVisible(r: [number, number] | null): void {
      if (
        rangoVisible !== null &&
        r !== null &&
        rangoVisible[0] === r[0] &&
        rangoVisible[1] === r[1]
      ) {
        return;
      }
      rangoVisible = r;
    },

    // ---- Mutaciones de consulta (incrementan versionConsulta) ----
    /** Cambia la tabla mostrada (principal/variantes) y reinicia la consulta. */
    setTabla(t: Tabla): void {
      if (tabla === t) return;
      tabla = t;
      // Los ids excluidos pertenecen a la tabla anterior.
      excluidos = [];
      seleccion.clear();
      marcarConsulta();
    },

    /** Fija el texto de búsqueda libre. */
    setBusqueda(texto: string): void {
      if (busqueda === texto) return;
      busqueda = texto;
      marcarConsulta();
    },

    /** Fija (o limpia con `null`) el filtro rápido del panel lateral. */
    setFiltroRapido(f: FiltroRapido | null): void {
      filtroRapido = f;
      marcarConsulta();
    },

    /** Reemplaza el conjunto de condiciones de filtro avanzado. */
    setCondiciones(c: CondicionFiltro[]): void {
      condiciones = c;
      marcarConsulta();
    },

    /** Agrega una condición al final de la lista (se suma con Y por defecto). */
    agregarCondicion(c: CondicionFiltro): void {
      condiciones = normalizarConectores([...condiciones, c]);
      marcarConsulta();
    },

    /** Quita la condición en el índice indicado. */
    quitarCondicion(indice: number): void {
      condiciones = normalizarConectores(
        condiciones.filter((_, i) => i !== indice),
      );
      marcarConsulta();
    },

    /**
     * Mueve una condición de `desde` a `hasta` (índices de la lista actual).
     * El orden importa: la cadena se evalúa de izquierda a derecha.
     */
    moverCondicion(desde: number, hasta: number): void {
      if (
        desde === hasta ||
        desde < 0 ||
        hasta < 0 ||
        desde >= condiciones.length ||
        hasta >= condiciones.length
      ) {
        return;
      }
      const lista = condiciones.slice();
      const [movida] = lista.splice(desde, 1);
      lista.splice(hasta, 0, movida);
      // Cada condición lleva su conector consigo, salvo al cambiar la cabeza:
      // la nueva primera pierde el suyo y se lo cede a la antigua primera, así
      // invertir "A O B" da "B O A" y no "B Y A".
      if (lista[0] !== condiciones[0]) {
        const exCabeza = lista.indexOf(condiciones[0]);
        lista[exCabeza] = { ...condiciones[0], opRel: lista[0].opRel ?? "y" };
      }
      condiciones = normalizarConectores(lista);
      marcarConsulta();
    },

    /** Cambia el conector Y/O de la condición `indice` (no aplica a la primera). */
    setConector(indice: number, opRel: OpRel): void {
      if (indice <= 0 || indice >= condiciones.length) return;
      condiciones = condiciones.map((c, i) =>
        i === indice ? { ...c, opRel } : c,
      );
      marcarConsulta();
    },

    /** Quita condiciones, filtro rápido y exclusiones manuales. */
    limpiarCondiciones(): void {
      filtroRapido = null;
      condiciones = [];
      excluidos = [];
      marcarConsulta();
    },

    // ---- Exclusiones manuales ("Quitar del filtro") ----
    /** Quita del resultado los registros indicados y limpia la selección. */
    excluir(ids: Iterable<number>): void {
      const nuevos = new Set(excluidos);
      for (const id of ids) nuevos.add(id);
      if (nuevos.size === excluidos.length) return;
      excluidos = [...nuevos];
      seleccion.clear();
      marcarConsulta();
    },

    /** Devuelve al resultado un registro excluido. */
    restaurarExcluido(id: number): void {
      excluidos = excluidos.filter((x) => x !== id);
      marcarConsulta();
    },

    /** Devuelve al resultado todos los registros excluidos. */
    restaurarExcluidos(): void {
      if (excluidos.length === 0) return;
      excluidos = [];
      marcarConsulta();
    },

    /**
     * Reemplaza los niveles de orden (se recortan a `MAX_ORDEN`).
     */
    setOrden(o: OrdenCampo[]): void {
      orden = o.slice(0, MAX_ORDEN);
      marcarConsulta();
    },

    /** Alterna o agrega un campo al orden (asc → desc → quitar). */
    alternarOrden(campo: string): void {
      const idx = orden.findIndex((o) => o.campo === campo);
      if (idx === -1) {
        if (orden.length >= MAX_ORDEN) return;
        orden = [...orden, { campo, direccion: "asc" }];
      } else if (orden[idx].direccion === "asc") {
        orden = orden.map((o, i) =>
          i === idx ? { ...o, direccion: "desc" } : o,
        );
      } else {
        orden = orden.filter((_, i) => i !== idx);
      }
      marcarConsulta();
    },

    /** Fija (o limpia con `null`) el grupo jerárquico seleccionado. */
    setGrupoSel(g: SeleccionGrupo | null): void {
      grupoSel = g;
      seleccion.clear();
      marcarConsulta();
    },

    /** Alterna la visibilidad de los registros ocultos (`_auxiliar_`). */
    alternarOcultos(): void {
      incluirOcultos = !incluirOcultos;
      marcarConsulta();
    },

    /** Quita todos los filtros, búsqueda y grupo, y reinicia la consulta. */
    limpiarFiltros(): void {
      filtroRapido = null;
      condiciones = [];
      excluidos = [];
      busqueda = "";
      grupoSel = null;
      marcarConsulta();
    },

    /** Fuerza una recarga de la grilla sin cambiar la consulta. */
    refrescar(): void {
      marcarConsulta();
    },

    /** Señala que la lista de filtros guardados cambió (guardar/eliminar). */
    marcarFiltros(): void {
      versionFiltros++;
    },

    // ---- Presentación ----
    /** Cambia la vista entre grilla y tabla. */
    setVista(v: Vista): void {
      vista = v;
    },

    get columnasFijas() {
      return columnasFijas;
    },

    /** Fija el número de columnas de la grilla (null = auto por zoom). */
    setColumnasFijas(n: number | null): void {
      columnasFijas = n;
    },

    get letraEscala() {
      return letraEscala;
    },

    /** Alterna si la letra de las cards escala con el zoom o queda fija. */
    alternarLetraEscala(): void {
      letraEscala = !letraEscala;
    },

    /** Fija el zoom de celda, acotado a [ZOOM_MIN, ZOOM_MAX]. Mover el zoom
        vuelve al modo auto de columnas. */
    setZoom(px: number): void {
      zoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, Math.round(px)));
      columnasFijas = null;
    },

    /** Aumenta el zoom un paso. */
    aumentarZoom(): void {
      this.setZoom(zoom + ZOOM_PASO);
    },

    /** Reduce el zoom un paso. */
    reducirZoom(): void {
      this.setZoom(zoom - ZOOM_PASO);
    },

    // ---- Selección ----
    /** Alterna la selección de un registro por id. */
    alternarSeleccion(id: number): void {
      if (seleccion.has(id)) seleccion.delete(id);
      else seleccion.add(id);
    },

    /** Selecciona un único registro (limpia el resto). */
    seleccionarUno(id: number): void {
      seleccion.clear();
      seleccion.add(id);
    },

    /** Selecciona un conjunto de ids (reemplaza la selección). */
    seleccionarVarios(ids: Iterable<number>): void {
      seleccion.clear();
      for (const id of ids) seleccion.add(id);
    },

    /** Limpia la selección. */
    limpiarSeleccion(): void {
      seleccion.clear();
    },

    /** True si el registro está seleccionado. */
    estaSeleccionado(id: number): boolean {
      return seleccion.has(id);
    },

    // ---- Dirty ----
    /** Marca o desmarca cambios pendientes sin guardar. */
    setDirty(v: boolean): void {
      dirty = v;
    },

    // ---- Construcción de la petición de consulta ----
    /**
     * Construye una `QueryReq` para una ventana de registros con el estado
     * actual de filtros, orden, grupo y búsqueda.
     *
     * @param offset Desplazamiento del primer registro.
     * @param limit  Cantidad de registros a pedir.
     * @param idPrincipal Principal del que listar variantes (tabla 'variantes').
     */
    construirQuery(
      offset: number,
      limit: number,
      idPrincipal?: number,
    ): QueryReq {
      const b = busqueda.trim();
      return {
        tabla,
        idPrincipal: idPrincipal ?? null,
        grupo: grupoSel,
        filtroRapido,
        condiciones,
        busqueda: b === "" ? null : b,
        orden,
        incluirOcultos,
        excluidos,
        offset,
        limit,
      };
    },
  };
}
