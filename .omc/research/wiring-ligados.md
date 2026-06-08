# Wiring — Álbumes Ligados (ex-frmAlbumsL / frmEdligado / frmstligas)

Firmas exactas para incorporar a `CONTRACT.md`. Una "liga" sincroniza datos
DESDE otro álbum `.micdb` HACIA el actual usando un campo llave común; copia los
valores de los campos cuyo NOMBRE coincide en ambos (salvo la llave y los
calculados del destino, que se recalculan). Solo tabla principal.

## Comandos Tauri

| Comando | Args (JS) | Retorna |
|---|---|---|
| `ligados_listar` | `{ albumId }` | `Liga[]` |
| `liga_guardar` | `{ albumId, liga: Liga }` | `number` (id; id=0 → crear) |
| `liga_eliminar` | `{ albumId, ligaId }` | `void` |
| `liga_actualizar` | `{ albumId, ligaId }` | `ResultadoLiga` |
| `ligas_actualizar_todas` | `{ albumId }` | `ResultadoLiga[]` |

## Tipos (serde camelCase)

```
Liga = { id: number, rutaAlbum: string, llave: string, crearFaltantes: boolean }
ResultadoLiga = { actualizados: number, creados: number, sinCoincidencia: number }
```

- `Liga.id`: id en la tabla `ligados`; `0` al crear (lo asigna el backend).
- `Liga.rutaAlbum`: ruta absoluta del `.micdb` del que se copian datos.
- `Liga.llave`: nombre visible del campo llave (debe existir en ambos álbumes,
  tabla principal). Debe ser un campo no calculado y no multidato.
- `Liga.crearFaltantes`: si la llave del ligado no existe en el actual, da de
  alta el registro ("dar de alta si no existe").

## Eventos Tauri (backend → frontend)

| Evento | Payload |
|---|---|
| `liga-progreso` | `{ hechas: number, total: number }` |

Se emite cada ~50 registros procesados durante `liga_actualizar` /
`ligas_actualizar_todas` (patrón de `migracion-progreso`).

## Persistencia

Tabla `ligados` del esquema base (`id`, `nombre`, `config_json`). El DDL es
genérico, así que `config_json` guarda `{ rutaAlbum, llave, crearFaltantes }`
serializado (JSON) y `nombre` replica `rutaAlbum` para listados rápidos. No se
necesitó `ALTER TABLE` ni bump de `schema_version`.

## Semántica de `liga_actualizar`

1. Abre el álbum ligado con `mic_db::AlbumDb::abrir(ruta)` (lectura lógica) y
   lee sus campos (`repo_campos::listar`) y todos sus registros principales.
2. Indexa los registros del ligado por el valor textual de la llave.
3. Recorre los registros del álbum actual; por cada uno con coincidencia de
   llave, copia los campos de nombre común (salvo llave y calculados) vía
   `repo_registros::editar` (recalcula calculados con el motor del handle).
4. Si `crearFaltantes`, da de alta con `repo_registros::crear` las llaves del
   ligado que no existen en el actual.
5. Si NO `crearFaltantes`, cuenta esas llaves como `sinCoincidencia`.

## Archivos

- Backend: `crates/mic-db/src/repo_ligados.rs`,
  `crates/mic-tauri/src/commands/ligados.rs`.
- Registro: `crates/mic-db/src/lib.rs`,
  `crates/mic-tauri/src/commands/mod.rs`, `crates/mic-tauri/src/lib.rs`.
- Frontend: `src/lib/components/dialogs/ligadosIpc.ts`,
  `src/lib/components/dialogs/LinkedAlbumsDialog.svelte`.
- Integración: `src/lib/acciones.ts`,
  `src/lib/components/shell/MenuBar.svelte`,
  `src/lib/components/album/AlbumView.svelte` (modal id `ligados`),
  `src/lib/ipc/mock/index.ts`.
