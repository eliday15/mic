# QA integral del sistema — 2026-06-07 (navegador + agent-browser, IPC mock)

3 agentes secuenciales sobre el álbum demo (240 registros, 234 visibles). 41 casos.
Resultado final tras fixes: **41/41 funcionales**, 0 errores de consola.

## Cobertura

- **Núcleo (14 casos ✅)**: bienvenida/recientes, tabs, menús habilitado/deshabilitado,
  grilla (zoom, columnas A/2/4/6/8, badges), selección (simple/Shift/Cmd/todos), scroll
  virtual hasta el final sin huecos, vista tabla (solo columnas principales, orden por
  encabezado), búsqueda insensible a acentos, orden 3 niveles coherente, grupos (árbol,
  crear/navegar/eliminar), filtros avanzados Y/O + guardar/reabrir/eliminar, filtro rápido,
  ocultar/mostrar con estilo `cel--oculta`, paleta ⌘K, totalizar con estadísticas.
- **Captura (15 casos ✅)**: editor (texto/número/fecha), calculado solo-lectura con preview
  en vivo, multidato + categorías (sugerencias, gestor, default), lightbox/Escape, nuevo
  registro (234→235), eliminar con confirmación, lote desde inspector (3 reg.), totalizar
  respeta filtro, act. masiva por alcance, recalcular, visor 100% (zoom/pan/←→), imágenes
  de carpeta (+5).
- **Diálogos (13 casos ✅)**: exportar (dual-list, 3 formatos), importar (smoke), imprimir
  CI 2/línea + SI agrupado con subtotales + guardar/eliminar config, ligados (alta, progreso,
  resultado, eliminar), copiar álbum, empacar/desempacar, estructura de campos (crear/editar/
  fórmula con chips y preview/reordenar drag/eliminar), campos a la vista, wizard + plantillas,
  migración (smoke), cerrar/reabrir álbum, atajos ⌘F/⌘N/⌘K.

## Defectos encontrados → todos arreglados el mismo día

| Sev. | Defecto | Fix |
|---|---|---|
| ALTO | Variantes inaccesibles: clic en strip no abría, "+" no creaba, menú abría el principal | RecordEditor: navegación interna `tablaActual`/`idPrincipalActual` + breadcrumb "Volver al principal"; AlbumView `señalNuevaVariante` → tabla variantes/id null. **Causa raíz fina**: el `$effect` de sync de props rastreaba `tablaActual` (leído síncronamente por `cargar()`) y revertía la navegación → `untrack()` en el cuerpo (RecordEditor.svelte:99). ⚠️ Patrón Svelte 5 a recordar. |
| MEDIO | Sidebar "Filtros guardados" no refrescaba al guardar/eliminar desde el diálogo | `versionFiltros`/`marcarFiltros()` en albumState; el panel la observa |
| BAJO | Faltaba "Invertir selección" (VB6: InvierteSeleccion) | Acción + menú Editar + paleta; misma semántica que Seleccionar Todo (ids cargados) |
| BAJO | Toast "seleccionados" ambiguo en selección parcial | "Seleccionados N de M (solo los registros ya cargados)" |
| BAJO | Mock de miniaturas ignoraba `version` (cambio de imagen invisible en navegador) | `thumbMock` deriva también de version |
| BAJO | Sin UI para eliminar plantillas (IPC existía; VB6 tenía botón) | Botón con confirmación en NewAlbumWizard |

Re-test posterior: 9/9 ✅ (incl. flujo completo de variantes: abrir desde strip → volver →
nueva variante → guardar → conteo 3→4; regresión de Escape intacta).

## Notas (no defectos)
- "Seleccionar Todo"/"Invertir" operan sobre los registros CARGADOS del scroll virtual
  (por diseño v1; el toast ahora lo comunica).
- Filtro rápido y filtros avanzados son sistemas independientes con limpieza propia.
- Warnings benignos en consola del mock: "[TAURI] Couldn't find callback id" (sin backend).
- Capturas: /tmp/qa-sys-*.png y /tmp/qa-retest-*.png.
