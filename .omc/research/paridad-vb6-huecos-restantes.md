# MIC 3.0 — Huecos restantes vs VB6 (auditoría 2026-06-05, post-fase 2)

Doble auditoría: (a) relectura profunda del código VB6 (`/Users/eliasdayan/Downloads/micNOV2007`)
buscando detalles omitidos por el inventario, (b) verificación E2E del código nuevo.

**Veredicto fase 2:** los 14 ítems están cableados de punta a punta (Rust → IPC → mock → UI →
menú/paleta) y compila limpio (svelte-check 0E/0W, cargo check OK). Pero quedan huecos finos
y 4 bugs abiertos del QA.

## A. Bugs — estado real verificado en código (2026-06-05)

Los BUG-1..4 venían de `qa-recorrido.md` pero ese reporte quedó OBSOLETO: al verificar el
código actual los cuatro ya estaban arreglados (los fixes entraron durante la fase 2 sin
actualizar el doc).

| Bug | Sev. | Estado | Evidencia |
|---|---|---|---|
| BUG-1 columnas de variantes en vista principal | ALTA | ✅ ya estaba arreglado | `albumState.svelte.ts` filtra `c.visible && c.tabla === tabla` (con comentario); TableView usa `estado.camposVisibles` |
| BUG-2 fórmulas `{Campo}` rotas | ALTA | ✅ ya estaba arreglado | `FormulaEditor.svelte:85-89` inserta nombres planos (espacios→`_`, "Sin llaves: el lexer no las reconoce"); `motor.ts` acepta ambas formas |
| BUG-3 Escape en lightbox cierra el editor | MEDIA | ✅ ya estaba arreglado | `RecordEditor.svelte:70-82`: listener window en captura + `stopImmediatePropagation` cierra solo el lightbox |
| BUG-4 `getCurrentWebview()` en navegador | BAJA | ✅ ya estaba arreglado | `AlbumView.svelte`: guard `window.__MIC_MOCK__` (lo setea `instalarMock`, mock/index.ts:776) |
| BUG-5 **pérdida de datos en ligas** | **ALTA** | 🔧 arreglado 2026-06-05 | `sincronizar_liga` pasaba `Valores` parcial a `editar` → NULL en llave y columnas sin homónimo (repo_registros.rs:479). Fix: merge sobre registro completo + `id_por_llave` sin `.ok()` traga-errores + `ORDER BY _id_`, con tests de regresión en ligados.rs |
| BUG-6 Escape muerto con foco en `<body>` | BAJA | 🔧 arreglado 2026-06-05 | Pila global `utils/capasEscape.ts`; `Modal.svelte` registra capa (prop `cerrarEscape`); Combobox/CommandPalette con `stopPropagation`; afectaba a todos los diálogos con fases |

## B. Funcionalidad del VB6 que aún falta (por prioridad)

### B1. Reportes — configuración granular (el hueco más grande)
El VB6 (tablas `reportes`/`reportessi`, db.bas:1066-1199; clsReporteCI/SI) guardaba por reporte:
- `LineasXCampo` — alto/líneas POR CAMPO (no global; CSS fijo en el nuevo)
- `FontSize` + `FontSizeTitle` — tamaños de fuente configurables y diferenciados
- `caractXCampo` — ancho de columna por campo en reporte tabla
- `Encabezado()` — títulos de columna personalizados (renombrar columnas del reporte)
- `totalizable()` — qué columnas totalizar SE ELIGE POR REPORTE, no solo por flag de campo
- `CamposV` / `LineasXCampoV` / `ImagenesXLineaV` — sección de VARIANTES dentro del reporte,
  con configuración independiente. **El nuevo no imprime variantes en absoluto.**

`ConfigReporte` es JSON opaco para el backend → extender no requiere migración de esquema.

### B2. Categorías con valor "Default" (frmAutofill.frm:204-228, db.bas:447)
Botón "Default" marcaba un valor de categoría como predeterminado por campo (persistido en
`Categorias.Default`); la captura lo proponía automáticamente en registros nuevos.
El nuevo no tiene `valorDefault` ni en `CategoriaVal` ni en `CampoDef`.

### B3. Captura múltiple con comodines `@` (Module1.bas:181-262, CapturaCampos)
Al capturar N>1 imágenes de golpe: pregunta "¿aplicar datos a todas?", y los campos texto cuyo
valor empieza con `@` (ej. `@NOMBRE`) se sustituyen por la propiedad del archivo de imagen
(nombre, fecha, tamaño, tipo) vía `ObtenValCom`. Cancelar una imagen cancela todo el lote.
El nuevo tiene `registros_crear_desde_carpeta` pero crea registros con solo la imagen, sin
plantilla de valores ni comodines.

### B4. Validaciones de captura (Module2.bas:127-194 Esvalido, Module1.bas:135-157 Aplicafmto)
- Multidato no puede quedar vacío
- Texto no puede iniciar con `@` (reservado para comodines)
- Numérico/moneda: validación matemática de pérdida de precisión al aplicar formato
  (si `|antes-después| > 0.01` → error al usuario, no truncado silencioso)

### B5. Búsqueda
- Toggle "Mayúsculas/Minúsculas" por búsqueda (frmBuscar.frm:15-22, 296-300) — el nuevo
  siempre normaliza (NFD + toLower)
- Búsqueda transparente EN VARIANTES: si no halla en principal, busca en sus variantes y
  etiqueta el hallazgo "(imagen N de variantes)" (frmBuscar.frm:341-350) — verificar si la
  FTS nueva indexa variantes; la UI solo opera sobre la tabla activa

### B6. Importar CSV/XLS — ✅ IMPLEMENTADO (2026-06-05)
Modernizado como `importar_inspeccionar` + `importar_registros` (CONTRACT.md): CSV
(UTF-8/BOM/Windows-1252 + separador autodetectados) y XLSX (calamine), campo llave
seleccionable, políticas **sustituir / mantener / rellenar vacíos**, checkbox crear
faltantes, resumen previo en seco (dry-run) con huella anti-cambio-de-archivo, progreso
por evento, errores por fila sin abortar. `ImportarDialog.svelte` + mock para navegador.
25 tests Rust. Fuera de alcance v1 (deliberado): variantes `llave|variante` e import .mdb
(lo cubre el migrador).

### B7. Exportación de variantes
El VB6 codificaba variantes en CSV/XML con separador `|` preservando jerarquía reimportable.
El nuevo exporta una tabla a la vez (principal O variantes), sin estructura combinada.

### B8. Menores
- Badge visual "oculto" en miniaturas (existe toggle Ver→ocultos y badge de variantes, falta
  distintivo del registro oculto; ThumbnailGrid.svelte:430)
- Act. masiva: alcance "propagar a registros con el mismo valor anterior" (Check2 de
  frmActGrlDat) — emulable con filtro + "filtrados", pero no es 1 clic
- Act. masiva sobre variantes: el comando lo soporta (`req.tabla`), la UI asume tabla activa
- Importar plantillas `.xms` legado (XML) al sistema de plantillas JSON
- Convención VB6: al crear variantes, solo el 1er campo de usuario queda `visible=true`
  (db.bas:64-67) — revisar default del editor de estructura

## C. Confirmado código muerto en VB6 (no portar — valida los descartes)
testok()/anticopia (Module1.bas:666-734, comentado), Undo/Paste Special (no existen),
Web Browser (no existe), respaldo manual (no existe), lectura EmpImp legacy (solo existe
escritura CrearAME), "Act. Multidatos"/ActualizaEstructura (db.bas:1241-1307 — solo migración
de álbumes viejos; el migrador nuevo ya calcula conteos de multidatos).

## Orden sugerido
1. Bugs A (BUG-1, BUG-2 críticos; BUG-3, BUG-4 rápidos)
2. B1 reportes granulares + variantes en reportes (hueco más visible al imprimir)
3. B2 default de categorías + B4 validaciones (calidad de captura diaria)
4. B5 búsqueda (case-sensitive + variantes)
5. B3 captura múltiple con comodines
6. B6 importar CSV (decidir si se quiere; el VB6 lo tenía oculto)
7. B7/B8 cierre
