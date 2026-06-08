/**
 * Réplica TypeScript exacta del contrato de API (CONTRACT.md) y del modelo
 * Rust en `crates/mic-core/src/model.rs`. Serialización serde camelCase.
 *
 * Esta es la fuente de verdad de tipos para todo el frontend. No debe
 * divergir del backend: cualquier cambio aquí debe reflejar el contrato.
 */

// ---------------------------------------------------------------------------
// Enumeraciones básicas
// ---------------------------------------------------------------------------

/** Tipo de un campo configurable (equivale al enum TipoCar del VB6). */
export type TipoCampo =
  | "texto"
  | "numerico"
  | "moneda"
  | "fecha"
  | "calculado"
  | "multidato";

/** Tabla destino de un campo o registro. */
export type Tabla = "principal" | "variantes";

/** Dirección de un nivel de ordenamiento. */
export type Direccion = "asc" | "desc";

/** Operador de comparación de filtros avanzados (frmFA). */
export type OpComp =
  | "igual"
  | "distinto"
  | "mayor"
  | "menor"
  | "mayor_igual"
  | "menor_igual"
  | "contiene"
  | "empieza";

/** Conector lógico entre condiciones de filtro (Y / O). */
export type OpRel = "y" | "o";

// ---------------------------------------------------------------------------
// Valor dinámico
// ---------------------------------------------------------------------------

/**
 * Valor dinámico de un campo. JSON plano (untagged en serde):
 * `null | string | number | boolean`. Fechas = string ISO `YYYY-MM-DD`.
 */
export type Valor = null | string | number | boolean;

/** Mapa nombre-de-campo (visible) → valor. Nunca se indexa por colFisica. */
export type Valores = Record<string, Valor>;

// ---------------------------------------------------------------------------
// Definición de campos
// ---------------------------------------------------------------------------

/**
 * Definición de un campo configurable (equivale a clsImagenMic del VB6).
 * `valores` siempre se indexa por `nombre`, nunca por `colFisica`.
 */
export interface CampoDef {
  id: number;
  /** Nombre visible elegido por el usuario (puede tener espacios/acentos). */
  nombre: string;
  /** Nombre físico de la columna SQLite: `f_<id>`. */
  colFisica: string;
  tabla: Tabla;
  tipo: TipoCampo;
  /** Decimales de presentación (moneda/numérico). Solo formato. */
  decimales: number;
  totalizable: boolean;
  /** Fórmula para campos calculados (ex-sInfo del VB6). */
  formula: string | null;
  visible: boolean;
  modificable: boolean;
  ordenVisible: number;
  /** Formato de presentación (número y calculado): moneda, porcentaje o nulo. */
  formato: FormatoCampo | null;
}

/** Formato de presentación de un valor numérico (solo presentación). */
export type FormatoCampo = "moneda" | "porcentaje";

/** Datos para crear/editar un campo (sin id ni colFisica, que asigna mic-db). */
export interface CampoNuevo {
  nombre: string;
  tabla: Tabla;
  tipo: TipoCampo;
  decimales?: number;
  totalizable?: boolean;
  formula?: string | null;
  visible?: boolean;
  modificable?: boolean;
  ordenVisible?: number;
  formato?: FormatoCampo | null;
}

/** Plantilla de álbum: nombre + lista de campos iniciales (ex-frmNuevo/.xms). */
export interface Plantilla {
  nombre: string;
  campos: CampoNuevo[];
}

// ---------------------------------------------------------------------------
// Consultas (scroll virtual / tabla)
// ---------------------------------------------------------------------------

/** Un nivel de ordenamiento (hasta 3, como frmOrdenar). */
export interface OrdenCampo {
  campo: string;
  direccion: Direccion;
}

/** Una condición de filtro avanzado. */
export interface CondicionFiltro {
  /** Conector con la condición anterior (null en la primera). */
  opRel: OpRel | null;
  campo: string;
  opComp: OpComp;
  valor: string;
}

/** Filtro rápido del panel lateral: campo = valor. */
export interface FiltroRapido {
  campo: string;
  valor: string;
}

/** Selección en el árbol de grupos: grupo + valores elegidos por nivel. */
export interface SeleccionGrupo {
  grupoId: number;
  /** Valores seleccionados por nivel (1 a 3); null = todo el nivel. */
  valores: (string | null)[];
}

/** Petición de ventana de registros para el scroll virtual. */
export interface QueryReq {
  tabla: Tabla;
  /** Solo variantes de este principal (cuando tabla = 'variantes'). */
  idPrincipal?: number | null;
  /** Grupo jerárquico activo (hasta 3 niveles). */
  grupo?: SeleccionGrupo | null;
  filtroRapido?: FiltroRapido | null;
  condiciones: CondicionFiltro[];
  /** Búsqueda libre FTS5 (sin acentos). */
  busqueda?: string | null;
  /** Hasta 3 niveles de orden. */
  orden: OrdenCampo[];
  /** Incluir registros ocultos (`_auxiliar_`); por defecto se excluyen. */
  incluirOcultos?: boolean;
  offset: number;
  limit: number;
}

/** Registro ligero para la grilla (sin multidatos completos). */
export interface RegistroLigero {
  id: number;
  imagen: string | null;
  /** mtime de la imagen para versionar la URL de miniatura. */
  imagenVersion: number | null;
  tieneVariantes: boolean;
  /** Registro oculto (`_auxiliar_`); solo aparece con `incluirOcultos`. */
  oculto: boolean;
  valores: Valores;
}

/** Página de resultados para el scroll virtual. */
export interface QueryPage {
  total: number;
  offset: number;
  registros: RegistroLigero[];
}

/** Suma de un campo totalizable (resultado de "Totalizar"). */
export interface TotalCampo {
  campo: string;
  suma: number;
}

/** Resultado de totalizar el conjunto filtrado actual. */
export interface Totales {
  /** Número de registros que cumplen el filtro. */
  registros: number;
  totales: TotalCampo[];
}

/** Estadísticas de un campo numérico sobre el conjunto filtrado. */
export interface EstadisticaCampo {
  campo: string;
  /** Valores no nulos considerados. */
  cuenta: number;
  suma: number;
  media: number | null;
  mediana: number | null;
  /** Valor más frecuente. */
  moda: number | null;
  /** Cuántas veces aparece la moda. */
  modaConteo: number;
  minimo: number | null;
  maximo: number | null;
}

/** Resultado del panel de estadísticas (ex-Totalizar ampliado). */
export interface Estadisticas {
  registros: number;
  campos: EstadisticaCampo[];
}

/** Registro completo (editor): valores + multidatos. */
export interface RegistroCompleto {
  id: number;
  tabla: Tabla;
  imagen: string | null;
  imagenVersion: number | null;
  valores: Valores;
  /** nombre de campo multidato → lista de valores. */
  multidatos: Record<string, string[]>;
}

/** Resultado de fijar la imagen de un registro. */
export interface ImagenSet {
  imagen: string;
  imagenVersion: number;
}

// ---------------------------------------------------------------------------
// Grupos, categorías, álbum
// ---------------------------------------------------------------------------

/** Definición de un grupo jerárquico (tabla Grupos del original). */
export interface Grupo {
  id: number;
  nombre: string;
  /** Campo del nivel 1 (obligatorio). */
  por: string;
  /** Campo del nivel 2 (opcional). */
  luego1: string | null;
  /** Campo del nivel 3 (opcional). */
  luego2: string | null;
}

/** Nodo del árbol de grupos resuelto (valores distintos por nivel). */
export interface NodoGrupo {
  valor: string;
  conteo: number;
  hijos: NodoGrupo[];
}

/** Valor de categoría para autocomplete de multidatos. */
export interface CategoriaVal {
  valor: string;
  esDefault: boolean;
}

/** Información del álbum abierto. */
export interface AlbumInfo {
  /** Id de sesión asignado al abrir (clave para todos los comandos). */
  albumId: number;
  ruta: string;
  nombre: string;
  totalRegistros: number;
  tieneVariantes: boolean;
  campos: CampoDef[];
}

/** Álbum reciente (lista de inicio). */
export interface AlbumReciente {
  ruta: string;
  nombre: string;
}

/** Filtro avanzado guardado (con nombre). */
export interface FiltroGuardado {
  nombre: string;
  condiciones: CondicionFiltro[];
}

// ---------------------------------------------------------------------------
// Migración desde Access (.mdb)
// ---------------------------------------------------------------------------

/** Descripción de un campo detectado en la base Access original. */
export interface CampoMdb {
  nombre: string;
  tipo: string;
}

/** Inspección previa de un .mdb antes de migrar. */
export interface MdbInspeccion {
  tablas: string[];
  campos: CampoMdb[];
  totalEstimado: number;
  tieneVariantes: boolean;
}

/** Reporte final de una migración ejecutada. */
export interface MigracionReporte {
  filasPrincipal: number;
  filasVariantes: number;
  filasMultidatos: number;
  imagenesEncontradas: number;
  imagenesFaltantes: string[];
  advertencias: string[];
}

// ---------------------------------------------------------------------------
// Eventos (backend → frontend)
// ---------------------------------------------------------------------------

/** Payload del evento `migracion-progreso`. */
export interface MigracionProgreso {
  fase: string;
  hechas: number;
  total: number;
}

/** Payload del evento `album-cambiado`. */
export interface AlbumCambiado {
  albumId: number;
  ids: number[];
}

// ---------------------------------------------------------------------------
// Constantes de tamaño de miniatura
// ---------------------------------------------------------------------------

/** Tamaños de miniatura soportados por el protocolo `thumb` (0 = original). */
export type TamanoThumb = 0 | 128 | 256 | 512;
