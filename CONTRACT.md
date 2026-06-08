# MIC 3.0 — Contrato de API entre backend (Rust/Tauri) y frontend (Svelte)

**Este archivo es la fuente de verdad.** Los tipos Rust viven en `crates/mic-core/src/model.rs`
(serde `camelCase`). El frontend replica estas formas en `src/lib/domain/types.ts`.
Errores: todos los comandos devuelven `Result<T, String>` — el `String` es un mensaje en español listo para mostrar.

Convención invoke: nombres de comando en snake_case; argumentos desde JS en camelCase
(Tauri los convierte a snake_case automáticamente).

## Comandos Tauri

### Álbum
| Comando | Args (JS) | Retorna |
|---|---|---|
| `album_crear` | `{ ruta: string, nombre: string, campos: CampoNuevo[] }` | `AlbumInfo` |
| `album_abrir` | `{ ruta: string }` | `AlbumInfo` |
| `album_cerrar` | `{ albumId: number }` | `void` |
| `album_compactar` | `{ albumId: number }` | `void` |
| `albumes_recientes` | `{}` | `{ ruta: string, nombre: string }[]` |
| `album_recalcular` | `{ albumId: number }` | `number` (registros recalculados; ex "Act. Calculados") |
| `album_copiar` | `{ albumId, rutaDestino: string, soloEstructura: boolean }` | `number` (imágenes copiadas; ex frmCopAlbum) |
| `album_empacar` | `{ albumId, rutaZip: string }` | `number` (archivos empacados: .micdb + imagenes/; ex EmpacarAlbum) |
| `album_desempacar` | `{ rutaZip: string, dirDestino: string }` | `string` (ruta del .micdb extraído; no lo abre) |
| `plantillas_listar` | `{}` | `Plantilla[]` |
| `plantilla_guardar` | `{ nombre, campos: CampoNuevo[] }` | `void` (upsert por nombre; JSON en app_config_dir/plantillas.json) |
| `plantilla_eliminar` | `{ nombre }` | `void` |

`Plantilla = { nombre: string, campos: CampoNuevo[] }` (ex plantillas .xms del frmNuevo)

`AlbumInfo = { albumId, ruta, nombre, totalRegistros, tieneVariantes, campos: CampoDef[] }`
`CampoDef = { id, nombre, colFisica, tabla: 'principal'|'variantes', tipo: 'texto'|'numerico'|'moneda'|'fecha'|'calculado'|'multidato', decimales, totalizable, formula: string|null, visible, modificable, ordenVisible, formato: 'moneda'|'porcentaje'|null }`

`formato` es SOLO presentación (aplica a número y calculado; el dato sigue REAL): `'porcentaje'` muestra `12.50 %`, `'moneda'` muestra `$12.50`. `decimales` controla el redondeo de presentación (por defecto 2). Columna `campos.formato` añadida por migración idempotente al abrir álbumes previos.

### Campos (estructura del álbum)
| Comando | Args | Retorna |
|---|---|---|
| `campos_listar` | `{ albumId }` | `CampoDef[]` |
| `campo_crear` | `{ albumId, def: CampoNuevo }` | `CampoDef` |
| `campo_editar` | `{ albumId, campoId, def: CampoNuevo }` | `CampoDef` |
| `campo_eliminar` | `{ albumId, campoId }` | `void` |
| `campos_reordenar` | `{ albumId, orden: number[] }` | `void` (ids en orden visible) |
| `formula_probar` | `{ albumId, formula: string, valores: Valores }` | `Valor` (vista previa editor fórmulas) |

### Registros (grilla virtual / tabla)
| Comando | Args | Retorna |
|---|---|---|
| `registros_query` | `{ albumId, req: QueryReq }` | `QueryPage` |
| `registro_obtener` | `{ albumId, id, tabla }` | `RegistroCompleto` |
| `registro_crear` | `{ albumId, tabla, valores: Valores, multidatos: Record<string,string[]>, imagenOrigen?: string, idPrincipal?: number }` | `number` (id) |
| `registro_editar` | `{ albumId, id, tabla, valores: Valores, multidatos?: Record<string,string[]> }` | `RegistroCompleto` (con calculados recalculados) |
| `registros_eliminar` | `{ albumId, ids: number[], tabla }` | `void` |
| `registro_imagen_set` | `{ albumId, id, tabla, rutaOrigen: string }` | `{ imagen: string, imagenVersion: number }` (copia a imagenes/) |
| `registros_editar_lote` | `{ albumId, ids: number[], tabla, valores: Valores }` | `void` (inspector multi-selección) |
| `registros_set_auxiliar` | `{ albumId, ids: number[], tabla, oculto: boolean }` | `void` (ocultar/mostrar sin eliminar) |
| `registros_totalizar` | `{ albumId, req: QueryReq }` | `Totales` (suma de campos totalizables del conjunto filtrado) |
| `registros_actualizar_masivo` | `{ albumId, req: QueryReq, valores: Valores }` | `number` (registros tocados; ex frmActGrlDat) |
| `registros_crear_desde_carpeta` | `{ albumId, carpeta: string }` | `number` (registros creados; ex "Imagenes de Dir"; progreso vía `carpeta-progreso`) |

`QueryReq = { tabla, idPrincipal?, grupo?: SeleccionGrupo, filtroRapido?: {campo,valor}, condiciones: CondicionFiltro[], busqueda?, orden: {campo,direccion:'asc'|'desc'}[], incluirOcultos?: boolean, offset, limit }`
`QueryPage = { total, offset, registros: RegistroLigero[] }`
`RegistroLigero = { id, imagen, imagenVersion, tieneVariantes, oculto: boolean, valores: Record<string, null|string|number|boolean> }`
`Totales = { registros: number, totales: { campo: string, suma: number }[] }`
`CondicionFiltro = { opRel: 'y'|'o'|null, campo, opComp: 'igual'|'distinto'|'mayor'|'menor'|'mayor_igual'|'menor_igual'|'contiene'|'empieza', valor }`

### Variantes
| Comando | Args | Retorna |
|---|---|---|
| `variantes_listar` | `{ albumId, idPrincipal }` | `RegistroLigero[]` |

(crear/editar variantes = `registro_crear`/`registro_editar` con `tabla: 'variantes'`)

### Multidatos y categorías
| Comando | Args | Retorna |
|---|---|---|
| `categorias_sugerir` | `{ albumId, campoId, principal: boolean, prefijo: string }` | `string[]` |
| `categorias_listar` | `{ albumId, campoId, principal }` | `CategoriaVal[]` |
| `categorias_actualizar` | `{ albumId, campoId, principal, valores: CategoriaVal[] }` | `void` |

### Grupos
| Comando | Args | Retorna |
|---|---|---|
| `grupos_listar` | `{ albumId }` | `Grupo[]` |
| `grupo_guardar` | `{ albumId, grupo: Grupo }` | `number` (id; id=0 → crear) |
| `grupo_eliminar` | `{ albumId, grupoId }` | `void` |
| `grupo_arbol` | `{ albumId, grupoId }` | `NodoGrupo[]` (valores distintos por nivel, con conteos) |

### Filtros avanzados guardados
| Comando | Args | Retorna |
|---|---|---|
| `filtros_listar` | `{ albumId }` | `string[]` |
| `filtro_obtener` | `{ albumId, nombre }` | `CondicionFiltro[]` |
| `filtro_guardar` | `{ albumId, nombre, condiciones }` | `void` |
| `filtro_eliminar` | `{ albumId, nombre }` | `void` |

### Exportación
| Comando | Args | Retorna |
|---|---|---|
| `exportar_registros` | `{ albumId, req: QueryReq, campos: string[], formato: 'csv'\|'xlsx'\|'csv-mic', rutaDestino }` | `number` (registros exportados; ex frmExp) |

Respeta filtro/orden de `req` (ignora offset/limit). Multidatos unidos con `" | "`. CSV = UTF-8 con BOM; XLSX = encabezados en negrita.
`csv-mic` = CSV legible por el "Importar..." del MIC clásico (VB6): Windows-1252 sin BOM, coma sin comillas (comas/tabs/saltos dentro de valores → espacio), CRLF; la primera columna de `campos` es el campo llave con el que el MIC 2007 actualiza su álbum.

### Importación de registros
| Comando | Args (JS) | Retorna |
|---|---|---|
| `importar_inspeccionar` | `{ albumId, rutaArchivo: string }` | `InspeccionImport` |
| `importar_registros` | `{ albumId, rutaArchivo: string, campoLlave: string, politica: 'sustituir'\|'mantener'\|'rellenar_vacios', crearFaltantes: boolean, dryRun: boolean, huella?: string }` | `ResultadoImportacion` |

`InspeccionImport = { columnas: string[], totalFilas: number, encoding: 'utf-8'|'utf-8-bom'|'windows-1252', formato: 'csv'|'xlsx', columnasReconocidas: string[], columnasNoReconocidas: string[], camposLlaveSugeridos: string[], huella: string }`
`ResultadoImportacion = { actualizados: number, creados: number, sinCambio: number, errores: string[], avisos: string[], dryRun: boolean }`

Lee CSV (UTF-8 con/sin BOM o Windows-1252 autodetectado; también el `csv-mic` propio) o XLSX (primera hoja, crate calamine). Casa columnas con campos del álbum por nombre case-insensitive e independiente del orden; columnas no reconocidas se ignoran (aviso). El `campoLlave` (default: primera columna) nunca se actualiza y distingue mayúsculas/acentos; los campos calculados se recalculan (no se importan); multidato se parte por "|". Duplicados de llave DENTRO del archivo → se usa la primera fila y se avisa. `dryRun:true` analiza sin escribir y devuelve el resumen previo; `dryRun:false` aplica todo. `politica` decide en colisión de llave: `sustituir` (sobrescribe), `mantener` (no toca el registro), `rellenar_vacios` (solo escribe campos hoy vacíos/null). `crearFaltantes` da de alta llaves no encontradas (sin imagen). `huella` (largo+mtime de la inspección) se valida al aplicar: si el archivo cambió, devuelve error. Emite `importacion-progreso`. Solo tabla principal.

### Reportes (impresión)
| Comando | Args | Retorna |
|---|---|---|
| `reportes_listar` | `{ albumId }` | `ReporteGuardado[]` |
| `reporte_guardar` | `{ albumId, nombre, config: ConfigReporte }` | `void` (upsert por nombre) |
| `reporte_eliminar` | `{ albumId, nombre }` | `void` |

`ReporteGuardado = { nombre, config: ConfigReporte }` — `config` es JSON opaco para el backend (tabla `reportes.config_json`).
`ConfigReporte = { tipo: 'ci'|'si', titulo, campos: string[], imagenesPorLinea: 1|2|4|8, orientacion: 'vertical'|'horizontal', papel: 'carta'|'oficio'|'a4', ponFecha, ponPagina, ponTotales, agrupacion: string|null }`
El render y la impresión son del frontend (HTML + `@media print` + `window.print()`); ex frmprint/frmprint2/frmPreliminar.

### Álbumes ligados
| Comando | Args | Retorna |
|---|---|---|
| `ligados_listar` | `{ albumId }` | `Liga[]` |
| `liga_guardar` | `{ albumId, liga: Liga }` | `number` (id; id=0 → crear) |
| `liga_eliminar` | `{ albumId, ligaId }` | `void` |
| `liga_actualizar` | `{ albumId, ligaId }` | `ResultadoLiga` |
| `ligas_actualizar_todas` | `{ albumId }` | `ResultadoLiga[]` |

`Liga = { id, rutaAlbum, llave, crearFaltantes }` · `ResultadoLiga = { actualizados, creados, sinCoincidencia }`
Sincroniza DESDE otro `.micdb` HACIA el actual por campo llave (tabla principal); copia campos con nombre común (salvo llave/calculados, que se recalculan); `crearFaltantes` da de alta llaves nuevas. Persistido en `ligados.config_json`. Ex frmAlbumsL/frmEdligado/frmstligas.

### Migración
| Comando | Args | Retorna |
|---|---|---|
| `migracion_verificar_mdbtools` | `{}` | `boolean` |
| `migracion_inspeccionar` | `{ rutaMdb }` | `MdbInspeccion` |
| `migracion_ejecutar` | `{ rutaMdb, rutaDestino }` | `MigracionReporte` |

`MdbInspeccion = { tablas: string[], campos: {nombre,tipo}[], totalEstimado: number, tieneVariantes: boolean }`
`MigracionReporte = { filasPrincipal, filasVariantes, filasMultidatos, imagenesEncontradas, imagenesFaltantes: string[], advertencias: string[] }`

### Miniaturas
- **No hay comando de fetch**: las miniaturas se sirven por el protocolo custom `thumb`.
- URL: `thumb://localhost/{albumId}/{tabla}/{id}?size={128|256|512}&v={imagenVersion}`
  (en Windows: `http://thumb.localhost/...` — usar helper `thumbUrl()` en `src/lib/ipc/thumbnails.ts`
  que detecta plataforma igual que hace `convertFileSrc`).
- El handler Rust: cache-hit en `.thumbs/{size}/{hash}.jpg` → bytes con `Cache-Control: max-age=31536000, immutable`;
  miss → genera (semáforo de N=cores), guarda y responde. 404 si la imagen original no existe → frontend muestra placeholder.
- `thumb_invalidar` | `{ albumId, id, tabla }` | `void` — borra los thumbs cacheados del registro.
- La imagen original a tamaño completo (visor 100%) se sirve igual: `size=0` = original.

## Eventos Tauri (backend → frontend)
| Evento | Payload |
|---|---|
| `migracion-progreso` | `{ fase: string, hechas: number, total: number }` |
| `album-cambiado` | `{ albumId, ids: number[] }` (registros modificados fuera del flujo normal) |
| `liga-progreso` | `{ hechas: number, total: number }` (sincronización de álbumes ligados) |
| `carpeta-progreso` | `{ hechas: number, total: number }` (alta masiva de imágenes desde carpeta) |
| `importacion-progreso` | `{ fase: 'analizando'\|'aplicando', hechas: number, total: number }` (análisis/aplicación de importación de registros) |

## Notas de implementación
- `Valor` es JSON plano untagged: `null | string | number | boolean`. Fechas = string ISO `YYYY-MM-DD`.
- `valores` siempre se indexa por `CampoDef.nombre` (el visible), nunca por `colFisica`.
- Campos `calculado` son de solo lectura: el backend los recalcula en crear/editar y los devuelve.
- Campo `multidato` ("Etiquetas" en la UI): en `RegistroLigero.valores` lleva el RESUMEN de texto de sus valores (`"a · b"`, o null sin valores) para mostrarse en grilla/tabla; en `RegistroCompleto.valores` lleva el conteo (number) y los valores reales van por `multidatos`. El orden por multidato sigue usando el conteo (columna física).
- En la UI, `moneda` no es un tipo aparte: se presenta como "Número" con formato moneda ($). El dominio conserva `numerico`/`moneda` sin cambios.
