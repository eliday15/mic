# MIC 3.0 — Matriz de paridad VB6 (micNOV2007) → fase 2 "completar 2026"

Fuentes: plan original (`~/.claude/plans/fluttering-chasing-reddy.md`), inventario
funcional VB6 (`.omc/research/inventario-vb6.md`), código actual (CONTRACT.md).

## Núcleo v1 — YA CUBIERTO ✅

Crear/abrir álbum (.micdb) · campos dinámicos de 6 tipos · captura/edición
(RecordEditor + DynamicForm) · variantes · multidatos + categorías (=frmAutofill,
CategoryManager) · búsqueda FTS sin acentos (=frmBuscar, modernizada como filtro
global) · filtros avanzados Y/O con guardado (=frmFA/frmAbreFA) · orden 3 niveles
(=frmOrdenar) · grupos jerárquicos con árbol (=modo grupo) · selección múltiple ·
zoom continuo (= escalas 8/4/2) · scroll virtual (= paginación clsPaginas) ·
campos a la vista/estructura (=frmDALV/frmEdCmps/frmnewp/frmauxcmp/frmCaptForm) ·
edición en lote vía inspector (≈frmActGrlDat parcial) · migración .mdb ·
compactar · vista tabla · paleta ⌘K · drag&drop (Tauri).

## HUECOS A IMPLEMENTAR (fase 2)

| # | Función | VB6 | Estado | Diseño 2026 |
|---|---|---|---|---|
| 1 | **Totalizar** | frmTotalizar | ❌ | Comando `registros_totalizar(QueryReq)` → suma de campos `totalizable` sobre los registros visibles (respeta filtro/grupo/búsqueda). Diálogo tabla Campo→Total + conteo. Menú Herramientas + ⌘K. |
| 2 | **Exportar** | frmExp | ❌ | Comando `exportar_registros(QueryReq, campos[], formato, ruta)` — CSV y XLSX (`rust_xlsxwriter`). Diálogo dual-list (Disponibles ⇄ Exportar, ↑/↓), respeta filtro y orden activos. Multidatos unidos con `|` (SEP_V original). |
| 3 | **Imprimir catálogo con imágenes** | frmprint + clsReporteCI | ❌ | Vista de impresión HTML (CSS `@media print`): N imágenes por línea (1/2/4/8), campos seleccionables con líneas por campo, título, fecha, nº página, totales. Configs guardadas en tabla `reportes` (config_json) vía `reportes_listar/guardar/eliminar`. Imprimir = print del webview (PDF nativo del SO). |
| 4 | **Reporte sin imágenes** | frmprint2 + clsReporteSI | ❌ | Mismo sistema, modo "tabla": columnas con ancho, agrupación por campo con subtotales, totales generales. |
| 5 | **Vista preliminar** | frmPreliminar | ❌ | Modal de previsualización (render del reporte a escala, paginado) antes de imprimir. |
| 6 | **Álbumes ligados** | frmAlbumsL/frmEdligado/frmstligas | ❌ (tabla `ligados` reservada) | Comandos `ligados_listar/guardar/eliminar` + `liga_actualizar(una|todas)` con evento de progreso `liga-progreso`. Sincroniza campos por llave desde otro `.micdb`; opción "crear registro si no existe". Diálogo de gestión + reporte de resultado. |
| 7 | **Copiar álbum** | frmCopAlbum/frmCopiar | ❌ | Comando `album_copiar(albumId, rutaDestino, soloEstructura)` — copia completa (VACUUM INTO + carpeta imagenes/) o solo estructura (campos+categorías+grupos+filtros+reportes). Diálogo con destino. |
| 8 | **Visor de imagen 100%** | MuestraImagen / visor.exe | ❌ (helper `imagenOriginalUrl` listo) | Modal visor: imagen original (size=0), zoom rueda/botones (ajustar/100%), pan arrastrando, ←/→ navega entre registros de la selección/grilla. Doble-clic en imagen del editor o menú contextual. |
| 9 | **Ocultar imagen** | `_auxiliar_` + menú contextual | ❌ (columna existe y se migra) | Comando `registros_set_auxiliar(ids, oculto)`; `QueryReq.incluirOcultos`; toggle "Mostrar ocultos" en Ver; badge visual en miniatura oculta. |
| 10 | **Act. Gral. de Datos** | frmActGrlDat | Parcial (lote del inspector exige selección) | Diálogo "Actualización masiva": campo + nuevo valor + alcance (seleccionados / filtrados / todos). Nuevo comando `registros_actualizar_masivo(QueryReq, campo, valor)`. |
| 11 | **Imágenes de Dir** | menú Herramientas | Parcial (drag&drop solo Tauri) | Comando `registros_crear_desde_carpeta(albumId, carpeta)` con evento de progreso; crea un registro por imagen. Menú Herramientas. |
| 12 | **Empacar / Desempacar** | EmpacarAlbum/frm3Botones | ❌ | `album_empacar(albumId, rutaZip)` / `album_desempacar(rutaZip, dirDestino)` (zip: .micdb + imagenes/). Para respaldo y compartir. |
| 13 | **Act. Calculados** | menú Herramientas | Implícito al editar | Comando `album_recalcular(albumId)` (mantenimiento tras migrar/ligar). Menú Herramientas. |
| 14 | **Plantillas de álbum** | frmNuevo (.xms) | ❌ (nota "fuera de v1" en wizard) | Paso de plantilla en el wizard: guardar estructura actual como plantilla JSON y crear desde plantilla. Importar `.xms` legado en el mismo paso. |

## Descartes deliberados (obsoletos en 2026)

- **Guardar / Usar respaldo** → SQLite WAL persiste al instante; respaldo = Copiar/Empacar.
- **Asociar .mdb / registro de Windows** → asociación `.micdb` la hace el instalador Tauri.
- **Protección anticopia / Desinstalar / frmkey / Registrar** → eliminado a propósito.
- **MDI / Cascade / Tile** → tabs por álbum.
- **Ayuda .hlp / Web Browser / Options vacíos** → no aplican.
- **Buscar siguiente (navegación por coincidencia)** → la búsqueda moderna filtra el universo (superior); no se replica el stepping.
- **Empaquetado EmpImp legacy** → sustituido por zip estándar (#12).

## Orden de implementación propuesto

1. Visor 100% (#8) + Ocultar (#9) — chicos, alto valor diario.
2. Totalizar (#1) + Act. masiva (#10) — chicos, completan herramientas de datos.
3. Exportar (#2) — mediano.
4. Impresión/reportes (#3+#4+#5) — grande, el hueco más visible vs VB6.
5. Copiar álbum (#7) + Empacar (#12) + Act. calculados (#13) — backend simple.
6. Álbumes ligados (#6) — mediano-grande (cross-álbum).
7. Imágenes de Dir (#11) + Plantillas (#14) — cierre.

Cada función: comando Rust (+CONTRACT.md) → wrapper `commands.ts` → mock para
navegador → UI Svelte → acción en menú/paleta → svelte-check + cargo test +
revisión agent-browser.
