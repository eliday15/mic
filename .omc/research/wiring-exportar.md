# Wiring — Exportación de registros (ex-frmExp)

Implementación de la exportación de registros (CSV / XLSX) en MIC 3.0.

## Comando añadido (para CONTRACT.md)

### Exportación

| Comando | Args (JS) | Retorna |
|---|---|---|
| `exportar_registros` | `{ albumId, req: QueryReq, campos: string[], formato: 'csv'\|'xlsx', rutaDestino: string }` | `number` (registros exportados; ex-frmExp) |

Firma Rust exacta:

```rust
pub async fn exportar_registros(
    state: State<'_, AppState>,
    album_id: u64,
    req: QueryReq,
    campos: Vec<String>,
    formato: String,
    ruta_destino: String,
) -> Result<u64, String>
```

- `campos` = nombres visibles de los campos a exportar, en ese orden (las columnas
  del archivo). Los nombres desconocidos se ignoran.
- `formato`: `"csv"` o `"xlsx"`. Cualquier otro valor → error en español.
- Respeta filtro y orden de `req`; ignora `offset`/`limit` (consulta TODOS los
  registros vía `mic_db::repo_registros::query` con `limit = u32::MAX`, `offset = 0`).
- Valores: texto tal cual; números con punto decimal (enteros sin `.0`); fechas
  ISO (ya llegan como texto desde la consulta); multidatos → valores reales
  (`mic_db::repo_multidatos::listar`) unidos con `" | "`.
- CSV: UTF-8 con BOM (`EF BB BF`) + separador coma (crate `csv`).
- XLSX: `rust_xlsxwriter` 0.79; encabezados en negrita; auto-ancho acotado a
  `[8, 60]` caracteres.

## Notas

- **Dependencias** (`crates/mic-tauri/Cargo.toml`): `csv` se tomó de
  `workspace.dependencies` (ya existía); `rust_xlsxwriter = "0.79"` se fijó a
  versión de crate.io (resuelve a `0.79.4`; la última publicada es 0.95, fuera
  de alcance de la tarea).
- **`mic_db::pool` y `mic_db::pool::Conn` son públicos**, por eso el helper
  `celda()` puede tipar `conn: &mic_db::pool::Conn` directamente.
- **Errores**: `MicError::Invalido` (campos vacíos / formato no soportado) y
  `MicError::Io` (fallos de escritura). Se propagan en español vía `en_db`.
- **Frontend**: el wrapper tipado vive en
  `src/lib/components/dialogs/exportarIpc.ts` (NO en `src/lib/ipc/commands.ts`,
  por estar acoplado al diálogo). El diálogo `ExportDialog.svelte` es dual-list
  (disponibles / incluidos) con Agregar/Quitar/Subir/Bajar, selector de formato
  y destino con `save()` de `@tauri-apps/plugin-dialog`. Llama al wrapper con
  `estado.construirQuery(0, 1)` (offset/limit simbólicos, ignorados por el backend).
- **Mock**: `exportar_registros` simula con `espera(400)` y devuelve
  `ejecutarQuery(album, req).length`.
- **Concurrencia**: durante esta tarea, `mod.rs`/`lib.rs` recibieron en paralelo
  un módulo `reportes`. Las anclas de exportación (`pub mod exportar;` tras
  `pub mod campos;`; `commands::exportar::exportar_registros,` tras
  `commands::campos::formula_probar,`) quedaron intactas y coexisten sin conflicto.

## Verificación

- `cargo check --workspace` → OK (0 errores).
- `npm run check` → 0 errores, 0 warnings.
