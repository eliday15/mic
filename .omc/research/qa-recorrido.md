# Reporte QA — MIC 3.0 (catálogo de inventario)

Entorno: `http://localhost:1420` (Vite dev + mock IPC en memoria, imágenes placeholder SVG).
Driver: `agent-browser --session qa`. Datos demo: 240 registros principales, de los cuales **6 están marcados `oculto`** (IDs 8, 48, 88, 128, 168, 208 → `i % 40 === 7` en `datos.ts`), por lo que la grilla muestra **234 visibles** — esto es esperado, no es bug.

Nota de método: durante el recorrido el código fuente se editó en vivo varias veces (se agregaron Totalizar, Act. Masiva, Mostrar Ocultos, Visor 100%, Copiar Álbum, etc.), por lo que algunas pruebas se repitieron tras recargar para evaluar el bundle vigente. Se descartan los fallos de carga de la ventana (~1 min) en que el servidor :1420 se cayó/reinició.

---

## (a) BUGS

### BUG-1 — La vista Tabla muestra columnas de la tabla `variantes` en la vista `principal` (severidad: ALTA)
La grilla en modo Tabla deriva sus columnas de `estado.camposVisibles`, que filtra solo por `c.visible` y **no por la tabla actual** (`src/lib/stores/albumState.svelte.ts:76-81`). Resultado: al ver la tabla principal aparecen también las columnas de variantes (**Talla, Color, Existencia, Precio Var**) siempre vacías para los 234 registros principales.
Repro: abrir Catálogo Demo → toolbar "Tabla" → scroll horizontal / leer cabeceras. Cabeceras observadas: Clave, Nombre, Marca, Línea, Cantidad, Precio, Fecha Alta, Importe, Etiquetas, Descripción, **Talla, Color, Existencia, Precio Var** (estas 4 con celdas en blanco).
Fix sugerido: filtrar `camposVisibles` (o las columnas de TableView) por `c.tabla === estado.tabla`.

### BUG-2 — El editor de fórmulas usa sintaxis con llaves `{Campo}` que el evaluador no entiende → vista previa siempre "—" (severidad: ALTA)
El `FormulaEditor` inserta y documenta tokens con llaves (placeholder `{Precio} * {Cantidad}`, los chips insertan `{Nombre}` vía `insertar()`), pero el evaluador `evaluarFormula` (`src/lib/ipc/mock/motor.ts:208-231`) reemplaza por **substring del nombre sin llaves** y luego valida con la regex `/^[\d\s+\-*/().,]+$/`, que rechaza `{` y `}`. Por eso toda fórmula construida desde la UI queda con llaves residuales y devuelve `null`.
Prueba directa de `formula_probar` con `{Cantidad:3, Precio:10}`:
- `"{Cantidad} * {Precio}"` → **null** (la sintaxis que la UI promueve)
- `"Cantidad * Precio"` → **30** (correcto)
Consecuencia: la vista previa del editor nunca calcula (siempre "—"), y un campo calculado creado con los chips tendría fórmula rota. El campo demo "Importe" funciona solo porque su fórmula es `"Cantidad * Precio"` (sin llaves). Hay incoherencia entre la UI y el evaluador.
Fix sugerido: que el evaluador reconozca `{Campo}` (o que la UI inserte el nombre sin llaves), unificando ambos.

### BUG-3 — Escape sobre el visor 100% (lightbox) cierra el RecordEditor completo en vez de solo el lightbox (severidad: MEDIA)
El lightbox (`RecordEditor.svelte:300-318`) se renderiza como hermano del `Modal`, tiene su propio overlay pero **no maneja la tecla Escape**; el `Modal` sí escucha Escape globalmente y se cierra. Al abrir el visor 100% dentro del editor y pulsar Escape, se cierran **ambos** (lightbox + editor), descartando cambios no guardados.
Repro: doble click en una miniatura → botón "100 %" → Escape. Observado: editor cerrado por completo, `editorOpen:false`.
Fix sugerido: handler de Escape propio del lightbox con `stopPropagation`, o un manejo de "overlay más alto cierra primero".

### BUG-4 — Rechazo de promesa no capturado en cada montaje de álbum: `getCurrentWebview()` (severidad: BAJA en mock; revisar en Tauri real)
En `AlbumView.svelte:86` el `$effect` que registra el drag-drop del SO llama `getCurrentWebview()` sin guardas; en el navegador (sin runtime Tauri) lanza `TypeError: Cannot read properties of undefined (reading 'currentWindow')` como `unhandledrejection` en **cada montaje de álbum**. Es el único error JS recurrente de la sesión.
Fix sugerido: try/catch o detección de entorno Tauri antes de `getCurrentWebview()`.

---

## (b) Controles / menús muertos o sin handler claro

- **Botón de "Filtros avanzados" en el sidebar (panel) sin `aria-label`** (`FiltersPanel.svelte`, el botón `SlidersHorizontal`): en el árbol de accesibilidad aparece como `button` sin nombre. A11y. (No está muerto: abre el AdvancedFiltersDialog.)
- **Sin botón de eliminar filtros guardados en el sidebar**: la lista "Filtros guardados" del panel solo permite *aplicar*; para *eliminar/abrir* hay que ir al diálogo "Filtros avanzados" (ahí sí hay iconos carpeta + papelera). Inconsistencia menor de descubribilidad.
- No se detectaron menús con item completamente sin handler en el bundle final: Totalizar, Act. Masiva, Recalcular, Campos del Álbum, etc. todos respondieron tras recargar al bundle vigente. (Un click temprano a "Totalizar" no hizo nada, pero fue por bundle desactualizado mid-edición, no por falta de handler.)

---

## (c) Funcionalidades probadas que SÍ existen (corrige supuestos del brief)

- **Visor de imagen 100%**: existe (lightbox desde el botón "100 %" del RecordEditor y opción "Ver 100 %" del menú contextual de la grilla). Funciona (ver BUG-3 por el Escape).
- **Totalizar**: existe y funciona — diálogo con Registros (234), Cantidad (14.717), Precio ($285.747,24), Importe (17.632.031,18). Suma los campos `totalizable`, respeta ocultos.
- **Actualización Masiva de Datos**: existe — diálogo Campo / Nuevo valor / Aplicar a (filtro actual) con advertencia.
- **Recalcular Campos Calculados**: existe — toast "422 campos recalculados".
- **Mostrar/Ocultar registros**: comandos "Ocultar" y "Mostrar (quitar oculto)" en la paleta ⌘K.
- **Copiar Álbum**: existe en la paleta de comandos (ARCHIVO → "Copiar Álbum…").
- (No se evaluó "Exportar" — añadido tarde; se omite.)

### Funcionalidades aún ausentes / no observadas
- **Imprimir**: no se vio comando/menú de impresión.
- **Rango visible en la barra de estado** ("1–30 de 234"): la StatusBar (`StatusBar.svelte`) muestra total + chips de filtro/orden/búsqueda/grupo + conteo de selección, pero **no** un rango de la ventana visible. El brief lo daba por presente; no existe.

---

## Funcionalidades verificadas OK (resumen)

| Área | Resultado |
|---|---|
| Pantalla inicio (Nuevo/Abrir/Importar, Recientes) | OK; toolbar correctamente deshabilitada sin álbum |
| Abrir "Catálogo Demo" | OK |
| Grilla virtual (scroll a 240/visible 234, llega a SKU-1240) | OK |
| Zoom slider min(90)/max(420) | OK en ambos extremos |
| Selección click / ⌘+click / shift+click (rango) | OK (5 seleccionados, status bar refleja) |
| Badge de variantes (punto azul) | OK, visible en todos los zooms |
| RecordEditor: texto/numérico/moneda/fecha/calculado(RO)/multidato(chips+autocomplete) | OK; Importe = Cantidad×Precio correcto (9×1377,27=12.395,43) |
| Multidato autocomplete ("of"→"oferta") + chips | OK |
| Editar Nombre + Guardar → reflejo en grilla/tabla/inspector | OK (persiste) |
| Strip de variantes (3 thumbs incl. una sin imagen + botón "+") | OK render |
| Vista Tabla: orden por cabecera (asc→desc→quitar) | OK |
| Edición inline de celda (doble click, Enter confirma) | OK (persiste) |
| Formato en celdas: $1.377,27 / fecha 13/12/2019 / Importe calc | OK |
| Sidebar Grupos: combo, árbol "Por Marca" (conteos suman 240/234) | OK |
| Grupo 2 niveles "Marca y Línea" (expandir, leaf filtra a 13) | OK |
| Nuevo grupo (Marca/Línea) | OK (aparece en combo) |
| Filtros: filtro guardado "Caros (>$1,000)" → 149 registros | OK |
| Diálogo Filtros avanzados (agregar/quitar cond., guardar, eliminar) | UI presente y correcta |
| Buscar ⌘F "lampara" (sin acento) → 19 (encuentra "Lámpara") | OK acento-insensible |
| Ordenar 3 niveles (Marca desc, Cantidad asc) | OK (Vértice→3,11,11,14,15) |
| Campos del Álbum: lista con tipo+tabla, edit/eliminar, "+ Nuevo campo", drag | OK (formula editor con preview, ver BUG-2) |
| Inspector: 1 sel. edición inline; N sel. edición en lote | OK (lote aplicó "ZZTest" a 5) |
| Paleta de comandos ⌘K (búsqueda + Enter ejecuta) | OK |
| Nuevo Álbum wizard (nombre, ubicación, 3 campos de tipos distintos) | OK → abre tab nuevo "Prueba" (0 registros), toast "Álbum creado" |
| Validación wizard (bloquea sin Ubicación, toast ⚠) | OK |
| Importar desde Access: inspección (total 1850, variantes Sí, 4 campos TC_TEXTO/TC_MONEDA/TC_FECHA) | OK hasta inspección (no se llegó a ejecutar progreso por tiempo) |
| Tabs: 2 álbumes, cambiar entre tabs, cerrar uno | OK |
| Status bar: total, chips de filtro/orden/búsqueda/grupo, conteo selección | OK (salvo rango visible, ver (c)) |

No probados por tiempo: tema claro/oscuro (toggle), Compactar, Cerrar álbum, paneles ocultar/mostrar (Ver), atajos ⌘O/⌘N/Escape genéricos, ejecución completa de la migración con barra de progreso y reporte final, crear/editar variante concreta, Mostrar Ocultos, Copiar Álbum, Exportar.

---

## (d) Observaciones de diseño / UX

1. **Estado vacío del Inspector usa el subtítulo de la app**: sin selección, el panel derecho muestra "Catálogo de inventario" (`t.app.subtitulo`, `InspectorPanel.svelte:170`). Parece texto suelto/placeholder; convendría un empty-state explícito ("Selecciona un registro").
2. **Toolbar con estado `[pressed]` en botones deshabilitados**: sin álbum, "Panel de Grupos" e "Inspector" aparecen `[disabled] [pressed]` a la vez — confuso semánticamente.
3. **Álbum nuevo vacío sin empty-state en la grilla**: el área central queda en blanco (solo el sidebar muestra "No hay grupos definidos"). Faltaría un mensaje tipo "Arrastra imágenes para empezar".
4. **Dos sintaxis de fórmula coexisten** (`{Campo}` en la UI vs `Campo` en datos demo) — además del BUG-2, es inconsistente de cara al usuario.
5. **Grupo demo "Línea / Marca / Año"** agrupa el 3er nivel por `Fecha Alta` (fecha completa), no por año; el nombre sugiere año.
6. **Descripción no se recalcula** al cambiar Marca por edición en lote (es texto almacenado): SKU-1004 quedó con Marca "ZZTest" pero Descripción seguía "…marca Acme." — comportamiento esperado, pero puede confundir.
7. **Refresco/HMR**: el `album_abrir` del mock reutiliza el álbum cacheado por ruta (no regenera); en navegador esto hace que reapariciones "soft" mantengan ediciones previas. Solo afecta al modo mock de desarrollo.
8. **Accesibilidad**: varios botones-icono sin `aria-label` (filtros avanzados del sidebar); los `Select` del sidebar son `<select>` nativos (bien) pero los chips de fórmula y algunos toggles dependen solo de iconos.
