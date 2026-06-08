# Wiring: Empacar/Desempacar + Plantillas de álbum

Implementación de las dos últimas funciones de paridad VB6 en MIC 3.0.
Firmas listas para añadir a `CONTRACT.md` (sección Álbum).

## Comandos Tauri nuevos

| Comando | Args (JS) | Retorna |
|---|---|---|
| `album_empacar` | `{ albumId, rutaZip: string }` | `number` (archivos empacados; ex EmpacarAlbum/frm3Botones) |
| `album_desempacar` | `{ rutaZip: string, dirDestino: string }` | `string` (ruta del `.micdb` extraído; NO lo abre) |
| `plantillas_listar` | `{}` | `Plantilla[]` |
| `plantilla_guardar` | `{ nombre: string, campos: CampoNuevo[] }` | `void` (upsert por nombre) |
| `plantilla_eliminar` | `{ nombre: string }` | `void` |

`Plantilla = { nombre: string, campos: CampoNuevo[] }` (serde camelCase).

### Notas de implementación (backend)

- `album_empacar`: crea un `.zip` (Deflated) con una copia consistente de la base
  (`VACUUM INTO` a un tempfile en `temp_dir`) en la raíz del zip como `<nombre>.micdb`,
  más toda la carpeta `imagenes/` recursiva con rutas relativas `imagenes/...`.
  Limpia el tempfile siempre. Crea el directorio destino si no existe.
- `album_desempacar`: extrae a `dirDestino` (lo crea), rechaza zip-slip vía
  `ZipArchive::enclosed_name()` (descarta entradas con `..`/rutas absolutas),
  y devuelve la ruta del PRIMER `.micdb` encontrado. Error en español si no hay `.micdb`.
- Plantillas: persistidas como JSON en `app_config_dir()/plantillas.json` (mismo
  patrón que `recientes.json`: `cargar_plantillas`/`guardar_plantillas`). `plantilla_guardar`
  hace upsert por nombre; `plantilla_eliminar` no falla si no existe.

Archivo: `crates/mic-tauri/src/commands/album.rs` (añadido al final).
Registro: `crates/mic-tauri/src/lib.rs`, tras `commands::album::album_copiar,`.
Dependencia: `zip = "2"` en `crates/mic-tauri/Cargo.toml`.

## Frontend

- Wrappers tipados en `src/lib/ipc/commands.ts`: `albumEmpacar`, `albumDesempacar`,
  `plantillasListar`, `plantillaGuardar`, `plantillaEliminar` (tras `albumCopiar`,
  también añadidos al objeto `comandos`).
- Tipo `Plantilla` en `src/lib/domain/types.ts` (tras `CampoNuevo`).
- Acciones en `src/lib/acciones.ts`:
  - `empacarActivo()` → `save()` (filtro zip, defaultPath `${a.nombre}.zip`) → `albumEmpacar` → toast `t.mensaje.empacado`.
  - `desempacarDialogo()` → `open()` (zip) → `open({directory:true})` → `albumDesempacar` → `albumes.abrir(ruta)` → toast `t.mensaje.desempacado`.
  - Cases `"empacar"` (requiere a) y `"desempacar"` (global) tras `"copiar-album"`.
  - Entradas en `accionesPaleta()` (`empacar` con `requiereAlbum`, `desempacar` sin él).
- Menú `archivo` en `src/lib/components/shell/MenuBar.svelte`: items `empacar`
  (deshabilitado sin álbum) y `desempacar` (siempre) tras `exportar`.
- `NewAlbumWizard.svelte`: Select "Plantilla" al inicio del formulario con
  "(desde cero)" + plantillas existentes; al elegir reemplaza la lista de campos
  (editables). Botón secundario "Guardar como plantilla…" abre un input inline
  (NO `window.prompt`) que llama `plantillaGuardar` con los campos actuales.
- i18n `src/lib/i18n/es.ts`:
  - `archivo.empacar` / `archivo.desempacar`.
  - `mensaje.empacado` / `mensaje.desempacado`.
  - Sección `plantillas: { titulo, desdeCero, guardarComo, nombre, guardada }` (antes de `a11y`).
- Mock `src/lib/ipc/mock/index.ts`: `album_empacar` (espera 400, return 42),
  `album_desempacar` (espera 400, return "/demo/desempacado/Catálogo Demo.micdb"),
  y plantillas con `plantillasMock` (Map module-level, plantilla demo "Catálogo básico"
  con 3 campos texto/moneda/fecha).

## Verificación

- `cargo check --workspace` → exit 0 (sin errores ni warnings).
- `npm run check` → 0 ERRORS 0 WARNINGS (3487 files).
