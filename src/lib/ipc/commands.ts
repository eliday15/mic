/**
 * Wrappers tipados de `invoke()` para cada comando Tauri del CONTRACT.md.
 *
 * Convención: el nombre del comando es snake_case exacto; los argumentos se
 * pasan en camelCase (Tauri los convierte a snake_case al deserializar).
 * Todos los comandos devuelven `Result<T, String>` en Rust: el `String` de
 * error se propaga como excepción rechazada con un mensaje en español.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AlbumInfo,
  AlbumReciente,
  CampoDef,
  CampoNuevo,
  CategoriaVal,
  CondicionFiltro,
  Grupo,
  ImagenSet,
  MdbInspeccion,
  Estadisticas,
  MigracionReporte,
  NodoGrupo,
  Plantilla,
  QueryPage,
  QueryReq,
  RegistroCompleto,
  RegistroLigero,
  Tabla,
  Totales,
  Valor,
  Valores,
} from "$lib/domain/types";

// ---------------------------------------------------------------------------
// Álbum
// ---------------------------------------------------------------------------

/** Crea un álbum nuevo con los campos iniciales indicados. */
export function albumCrear(
  ruta: string,
  nombre: string,
  campos: CampoNuevo[],
): Promise<AlbumInfo> {
  return invoke("album_crear", { ruta, nombre, campos });
}

/** Abre un álbum existente y devuelve su información de sesión. */
export function albumAbrir(ruta: string): Promise<AlbumInfo> {
  return invoke("album_abrir", { ruta });
}

/** Cierra la sesión de un álbum. */
export function albumCerrar(albumId: number): Promise<void> {
  return invoke("album_cerrar", { albumId });
}

/** Compacta (VACUUM) la base del álbum. */
export function albumCompactar(albumId: number): Promise<void> {
  return invoke("album_compactar", { albumId });
}

/** Lista los álbumes abiertos recientemente. */
export function albumesRecientes(): Promise<AlbumReciente[]> {
  return invoke("albumes_recientes", {});
}

/** Recalcula los campos calculados de todo el álbum; devuelve cuántos tocó. */
export function albumRecalcular(albumId: number): Promise<number> {
  return invoke("album_recalcular", { albumId });
}

/**
 * Copia el álbum a otra ruta (completo o solo estructura). Devuelve el número
 * de imágenes copiadas.
 */
export function albumCopiar(
  albumId: number,
  rutaDestino: string,
  soloEstructura: boolean,
): Promise<number> {
  return invoke("album_copiar", { albumId, rutaDestino, soloEstructura });
}

/**
 * Empaca el álbum en un `.zip` (base + carpeta `imagenes/`). Devuelve el número
 * de archivos empacados.
 */
export function albumEmpacar(
  albumId: number,
  rutaZip: string,
): Promise<number> {
  return invoke("album_empacar", { albumId, rutaZip });
}

/**
 * Desempaca un `.zip` de álbum a `dirDestino`. Devuelve la ruta del `.micdb`
 * extraído (no lo abre).
 */
export function albumDesempacar(
  rutaZip: string,
  dirDestino: string,
): Promise<string> {
  return invoke("album_desempacar", { rutaZip, dirDestino });
}

/** Lista las plantillas de álbum guardadas. */
export function plantillasListar(): Promise<Plantilla[]> {
  return invoke("plantillas_listar", {});
}

/** Guarda (upsert por nombre) una plantilla con los campos indicados. */
export function plantillaGuardar(
  nombre: string,
  campos: CampoNuevo[],
): Promise<void> {
  return invoke("plantilla_guardar", { nombre, campos });
}

/** Elimina la plantilla con el nombre indicado. */
export function plantillaEliminar(nombre: string): Promise<void> {
  return invoke("plantilla_eliminar", { nombre });
}

// ---------------------------------------------------------------------------
// Campos (estructura del álbum)
// ---------------------------------------------------------------------------

/** Lista las definiciones de campos del álbum. */
export function camposListar(albumId: number): Promise<CampoDef[]> {
  return invoke("campos_listar", { albumId });
}

/** Crea un campo nuevo. */
export function campoCrear(
  albumId: number,
  def: CampoNuevo,
): Promise<CampoDef> {
  return invoke("campo_crear", { albumId, def });
}

/** Edita la definición de un campo existente. */
export function campoEditar(
  albumId: number,
  campoId: number,
  def: CampoNuevo,
): Promise<CampoDef> {
  return invoke("campo_editar", { albumId, campoId, def });
}

/** Elimina un campo. */
export function campoEliminar(
  albumId: number,
  campoId: number,
): Promise<void> {
  return invoke("campo_eliminar", { albumId, campoId });
}

/** Reordena los campos (ids en el orden visible deseado). */
export function camposReordenar(
  albumId: number,
  orden: number[],
): Promise<void> {
  return invoke("campos_reordenar", { albumId, orden });
}

/** Evalúa una fórmula con un juego de valores (vista previa del editor). */
export function formulaProbar(
  albumId: number,
  formula: string,
  valores: Valores,
): Promise<Valor> {
  return invoke("formula_probar", { albumId, formula, valores });
}

// ---------------------------------------------------------------------------
// Registros
// ---------------------------------------------------------------------------

/** Consulta una ventana de registros para el scroll virtual / tabla. */
export function registrosQuery(
  albumId: number,
  req: QueryReq,
): Promise<QueryPage> {
  return invoke("registros_query", { albumId, req });
}

/** Obtiene un registro completo (valores + multidatos) para el editor. */
export function registroObtener(
  albumId: number,
  id: number,
  tabla: Tabla,
): Promise<RegistroCompleto> {
  return invoke("registro_obtener", { albumId, id, tabla });
}

/** Crea un registro; devuelve el id asignado. */
export function registroCrear(
  albumId: number,
  tabla: Tabla,
  valores: Valores,
  multidatos: Record<string, string[]>,
  imagenOrigen?: string,
  idPrincipal?: number,
): Promise<number> {
  return invoke("registro_crear", {
    albumId,
    tabla,
    valores,
    multidatos,
    imagenOrigen,
    idPrincipal,
  });
}

/** Edita un registro; devuelve el registro con calculados recalculados. */
export function registroEditar(
  albumId: number,
  id: number,
  tabla: Tabla,
  valores: Valores,
  multidatos?: Record<string, string[]>,
): Promise<RegistroCompleto> {
  return invoke("registro_editar", {
    albumId,
    id,
    tabla,
    valores,
    multidatos,
  });
}

/** Elimina uno o varios registros. */
export function registrosEliminar(
  albumId: number,
  ids: number[],
  tabla: Tabla,
): Promise<void> {
  return invoke("registros_eliminar", { albumId, ids, tabla });
}

/** Fija la imagen de un registro copiándola a `imagenes/`. */
export function registroImagenSet(
  albumId: number,
  id: number,
  tabla: Tabla,
  rutaOrigen: string,
): Promise<ImagenSet> {
  return invoke("registro_imagen_set", { albumId, id, tabla, rutaOrigen });
}

/** Edita un conjunto de campos en lote (inspector multi-selección). */
export function registrosEditarLote(
  albumId: number,
  ids: number[],
  tabla: Tabla,
  valores: Valores,
): Promise<void> {
  return invoke("registros_editar_lote", { albumId, ids, tabla, valores });
}

/** Oculta o muestra registros sin eliminarlos (`_auxiliar_`). */
export function registrosSetAuxiliar(
  albumId: number,
  ids: number[],
  tabla: Tabla,
  oculto: boolean,
): Promise<void> {
  return invoke("registros_set_auxiliar", { albumId, ids, tabla, oculto });
}

/** Suma los campos totalizables del conjunto filtrado actual. */
export function registrosTotalizar(
  albumId: number,
  req: QueryReq,
): Promise<Totales> {
  return invoke("registros_totalizar", { albumId, req });
}

/**
 * Estadísticas (cuenta, suma, media, mediana, moda, mín/máx) de los campos
 * numéricos indicados, sobre el conjunto filtrado actual.
 */
export function registrosEstadisticas(
  albumId: number,
  req: QueryReq,
  campos: string[],
): Promise<Estadisticas> {
  return invoke("registros_estadisticas", { albumId, req, campos });
}

/**
 * Actualización masiva: aplica `valores` a todos los registros que cumplen el
 * filtro de `req`. Devuelve cuántos registros tocó.
 */
export function registrosActualizarMasivo(
  albumId: number,
  req: QueryReq,
  valores: Valores,
): Promise<number> {
  return invoke("registros_actualizar_masivo", { albumId, req, valores });
}

// ---------------------------------------------------------------------------
// Variantes
// ---------------------------------------------------------------------------

/**
 * Alta masiva: crea un registro por cada imagen de la carpeta (no recursivo).
 * Devuelve cuántos registros creó. Progreso vía evento `carpeta-progreso`.
 */
export function registrosCrearDesdeCarpeta(
  albumId: number,
  carpeta: string,
): Promise<number> {
  return invoke("registros_crear_desde_carpeta", { albumId, carpeta });
}

/** Lista las variantes de un registro principal. */
export function variantesListar(
  albumId: number,
  idPrincipal: number,
): Promise<RegistroLigero[]> {
  return invoke("variantes_listar", { albumId, idPrincipal });
}

// ---------------------------------------------------------------------------
// Multidatos y categorías
// ---------------------------------------------------------------------------

/** Sugiere valores de categoría por prefijo (autocomplete). */
export function categoriasSugerir(
  albumId: number,
  campoId: number,
  principal: boolean,
  prefijo: string,
): Promise<string[]> {
  return invoke("categorias_sugerir", { albumId, campoId, principal, prefijo });
}

/** Lista todas las categorías de un campo multidato. */
export function categoriasListar(
  albumId: number,
  campoId: number,
  principal: boolean,
): Promise<CategoriaVal[]> {
  return invoke("categorias_listar", { albumId, campoId, principal });
}

/** Actualiza el catálogo de categorías de un campo multidato. */
export function categoriasActualizar(
  albumId: number,
  campoId: number,
  principal: boolean,
  valores: CategoriaVal[],
): Promise<void> {
  return invoke("categorias_actualizar", {
    albumId,
    campoId,
    principal,
    valores,
  });
}

// ---------------------------------------------------------------------------
// Grupos
// ---------------------------------------------------------------------------

/** Lista los grupos jerárquicos definidos. */
export function gruposListar(albumId: number): Promise<Grupo[]> {
  return invoke("grupos_listar", { albumId });
}

/** Guarda un grupo (id=0 → crear); devuelve el id. */
export function grupoGuardar(albumId: number, grupo: Grupo): Promise<number> {
  return invoke("grupo_guardar", { albumId, grupo });
}

/** Elimina un grupo. */
export function grupoEliminar(albumId: number, grupoId: number): Promise<void> {
  return invoke("grupo_eliminar", { albumId, grupoId });
}

/** Resuelve el árbol de valores distintos por nivel de un grupo. */
export function grupoArbol(
  albumId: number,
  grupoId: number,
): Promise<NodoGrupo[]> {
  return invoke("grupo_arbol", { albumId, grupoId });
}

// ---------------------------------------------------------------------------
// Filtros avanzados guardados
// ---------------------------------------------------------------------------

/** Lista los nombres de los filtros guardados. */
export function filtrosListar(albumId: number): Promise<string[]> {
  return invoke("filtros_listar", { albumId });
}

/** Obtiene las condiciones de un filtro guardado. */
export function filtroObtener(
  albumId: number,
  nombre: string,
): Promise<CondicionFiltro[]> {
  return invoke("filtro_obtener", { albumId, nombre });
}

/** Guarda (o reemplaza) un filtro con nombre. */
export function filtroGuardar(
  albumId: number,
  nombre: string,
  condiciones: CondicionFiltro[],
): Promise<void> {
  return invoke("filtro_guardar", { albumId, nombre, condiciones });
}

/** Elimina un filtro guardado. */
export function filtroEliminar(albumId: number, nombre: string): Promise<void> {
  return invoke("filtro_eliminar", { albumId, nombre });
}

// ---------------------------------------------------------------------------
// Miniaturas (invalidación de caché)
// ---------------------------------------------------------------------------

/** Borra los thumbs cacheados de un registro tras cambiar su imagen. */
export function thumbInvalidar(
  albumId: number,
  id: number,
  tabla: Tabla,
): Promise<void> {
  return invoke("thumb_invalidar", { albumId, id, tabla });
}

// ---------------------------------------------------------------------------
// Migración desde Access (.mdb)
// ---------------------------------------------------------------------------

/** Comprueba si `mdb-tools` está disponible en el sistema. */
export function migracionVerificarMdbtools(): Promise<boolean> {
  return invoke("migracion_verificar_mdbtools", {});
}

/** Inspecciona un .mdb antes de migrar (tablas, campos, conteos). */
export function migracionInspeccionar(
  rutaMdb: string,
): Promise<MdbInspeccion> {
  return invoke("migracion_inspeccionar", { rutaMdb });
}

/** Ejecuta la migración de un .mdb a un álbum SQLite nuevo. */
export function migracionEjecutar(
  rutaMdb: string,
  rutaDestino: string,
): Promise<MigracionReporte> {
  return invoke("migracion_ejecutar", { rutaMdb, rutaDestino });
}

/** Agrupación de todos los comandos para importación cómoda como espacio. */
export const comandos = {
  albumCrear,
  albumAbrir,
  albumCerrar,
  albumCompactar,
  albumesRecientes,
  albumRecalcular,
  albumCopiar,
  albumEmpacar,
  albumDesempacar,
  plantillasListar,
  plantillaGuardar,
  plantillaEliminar,
  camposListar,
  campoCrear,
  campoEditar,
  campoEliminar,
  camposReordenar,
  formulaProbar,
  registrosQuery,
  registroObtener,
  registroCrear,
  registroEditar,
  registrosEliminar,
  registroImagenSet,
  registrosEditarLote,
  registrosSetAuxiliar,
  registrosTotalizar,
  registrosEstadisticas,
  registrosActualizarMasivo,
  registrosCrearDesdeCarpeta,
  variantesListar,
  categoriasSugerir,
  categoriasListar,
  categoriasActualizar,
  gruposListar,
  grupoGuardar,
  grupoEliminar,
  grupoArbol,
  filtrosListar,
  filtroObtener,
  filtroGuardar,
  filtroEliminar,
  thumbInvalidar,
  migracionVerificarMdbtools,
  migracionInspeccionar,
  migracionEjecutar,
} as const;
