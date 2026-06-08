# Cableado: Sistema de impresión / reportes (frmprint / frmprint2 / frmPreliminar)

Migración del módulo de impresión del VB6 (frmprint = CI con imágenes,
frmprint2 = SI sin imágenes, frmPreliminar = vista preliminar, clsReporteCI /
clsReporteSI) a MIC 3.0. Diseño 2026: reportes renderizados como HTML +
`window.print()`; configuraciones guardadas como JSON en la tabla `reportes`.

## Comandos Tauri (firmas exactas, para CONTRACT.md)

Convención: nombre snake_case; args en camelCase; `Result<T, String>`.

| Comando | Args (JS) | Retorna |
|---|---|---|
| `reportes_listar` | `{ albumId }` | `ReporteGuardado[]` |
| `reporte_guardar` | `{ albumId, nombre: string, config: ConfigReporte }` | `void` |
| `reporte_eliminar` | `{ albumId, nombre: string }` | `void` |

`ReporteGuardado = { nombre: string, config: ConfigReporte }` (serde camelCase;
en Rust `config` es `serde_json::Value` opaco — lo interpreta el frontend).

`reporte_guardar` hace upsert por nombre. `reportes_listar` ordena alfabético
por nombre y devuelve `config: null` para filas con JSON inválido/nulo.

### ConfigReporte (frontend, JSON guardado)

```ts
interface ConfigReporte {
  tipo: "ci" | "si";               // con imágenes / sin imágenes
  titulo: string;                   // vacío => nombre del álbum
  campos: string[];                 // nombres visibles, en orden
  imagenesPorLinea: 1 | 2 | 4 | 8;  // solo ci
  orientacion: "vertical" | "horizontal";
  papel: "carta" | "oficio" | "a4"; // letter / legal / A4 en @page
  ponFecha: boolean;
  ponPagina: boolean;               // counter(page) en @media print
  ponTotales: boolean;              // suma de campos totalizables
  agrupacion: string | null;        // solo si: subtotales por grupo
}
```

## Persistencia

Tabla `reportes` ya reservada en `crates/mic-db/src/schema.rs`:
`reportes(id INTEGER PK, nombre TEXT, config_json TEXT)`. El repo guarda el
JSON en `config_json` (upsert por `nombre`).

## Archivos

Backend:
- `crates/mic-db/src/repo_reportes.rs` — CRUD (`ReporteGuardado`, `listar`,
  `guardar`, `eliminar`). Registrado en `lib.rs` (`pub mod repo_reportes;`).
- `crates/mic-tauri/src/commands/reportes.rs` — 3 comandos patrón `en_db`.
  Registrado en `commands/mod.rs` y en el `invoke_handler` de `lib.rs`.

Frontend:
- `src/lib/components/print/tipos.ts` — `ConfigReporte`, `ReporteGuardado`,
  `configPorDefecto()`.
- `src/lib/components/print/printIpc.ts` — wrappers `reportesListar` /
  `reporteGuardar` / `reporteEliminar` (independiente de `$lib/ipc/commands`).
- `src/lib/components/print/PrintSheet.svelte` — render de la hoja (ci/si),
  CSS de impresión (`@page`, `@media print`) inyectado en `<svelte:head>` solo
  al imprimir; oculta el resto de la app vía `body:has(.print-sheet--imprimiendo)`
  sin tocar `src/app.css`. Exporta `papelCss(papel)`.
- `src/lib/components/dialogs/PrintDialog.svelte` — formulario + selector de
  reportes guardados + vista previa con zoom (50/75/100 %) + `window.print()`.
  Trae todos los registros filtrados con `registrosQuery` +
  `estado.construirQuery(0, 100000)`.

## Integración (acción `imprimir`, modal `imprimir`)

- `src/lib/acciones.ts`: case `"imprimir"` (abre modal) + entrada en
  `accionesPaleta()`.
- `src/lib/components/shell/MenuBar.svelte`: item Archivo > Imprimir.
- `src/lib/components/album/AlbumView.svelte`: `dlgImprimir` + `<PrintDialog>`.
- `src/lib/ipc/mock/index.ts`: cases mock con `reportesMock` (Map por álbum).

## Notas de diseño

- La vista preliminar escala la hoja con `transform: scale()` (zoom CSS), sin
  re-render. La hoja usa unidades `mm` para coincidir con el papel físico.
- Totalizables = campos impresos con `totalizable` y tipo numérico/moneda/
  calculado. Subtotales por grupo (tipo si + agrupación) y total general.
- Números de página: `counter(page)` en `@media print` cuando `ponPagina`; si el
  motor lo ignora, degrada a sin número (sin romper el resto).
- i18n: todas las cadenas en `t.reportes.*` / `t.archivo.imprimir` (ya existían).
