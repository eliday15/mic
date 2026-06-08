/**
 * Mock IPC para desarrollo en navegador (sin Tauri).
 *
 * Instala `mockIPC` de @tauri-apps/api e implementa TODOS los comandos del
 * CONTRACT.md contra un álbum demo en memoria (ver `datos.ts` / `motor.ts`),
 * además del diálogo de archivos (`plugin:dialog|open`) y los eventos de
 * migración. Permite revisar la UI completa con un navegador normal.
 *
 * NUNCA se importa en producción: `main.ts` solo lo carga cuando corre en DEV
 * y no existe `window.__TAURI_INTERNALS__`.
 */

import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import type {
  AlbumInfo,
  CampoDef,
  CampoNuevo,
  CategoriaVal,
  CondicionFiltro,
  Grupo,
  QueryReq,
  RegistroCompleto,
  RegistroLigero,
  Tabla,
  Valores,
} from "$lib/domain/types";
import { crearAlbumDemo, type AlbumMock, type RegistroMock } from "./datos";
import {
  construirArbol,
  ejecutarQuery,
  evaluarFormula,
  normalizar,
  recalcular,
} from "./motor";

const albunes = new Map<number, AlbumMock>();
let proximoAlbumId = 1;

/** Reportes guardados por álbum (mock del sistema de impresión). */
const reportesMock = new Map<number, { nombre: string; config: unknown }[]>();

/** Una liga del mock de álbumes ligados. */
interface LigaMock {
  id: number;
  rutaAlbum: string;
  llave: string;
  crearFaltantes: boolean;
}

/** Ligas del mock (compartidas entre álbumes), con una liga demo inicial. */
const ligasMock = new Map<number, LigaMock>([
  [1, { id: 1, rutaAlbum: "/demo/Bodegas.micdb", llave: "Clave", crearFaltantes: false }],
]);
let proximaLigaId = 2;

/** Una plantilla de álbum del mock (nombre + campos iniciales). */
interface PlantillaMock {
  nombre: string;
  campos: CampoNuevo[];
}

/** Campo de plantilla con los valores por defecto del mock. */
function campoPlantilla(nombre: string, tipo: CampoNuevo["tipo"]): CampoNuevo {
  return {
    nombre,
    tabla: "principal",
    tipo,
    decimales: 2,
    totalizable: false,
    formula: null,
    visible: true,
    modificable: true,
    ordenVisible: 0,
  };
}

/** Plantillas del mock, con una plantilla demo inicial. */
const plantillasMock = new Map<string, PlantillaMock>([
  [
    "Catálogo básico",
    {
      nombre: "Catálogo básico",
      campos: [
        campoPlantilla("Nombre", "texto"),
        campoPlantilla("Precio", "moneda"),
        campoPlantilla("Fecha Alta", "fecha"),
      ],
    },
  ],
]);

function obtenerAlbum(albumId: number): AlbumMock {
  const a = albunes.get(albumId);
  if (!a) throw `El álbum ${albumId} no está abierto`;
  return a;
}

function tablaDe(album: AlbumMock, tabla: Tabla): RegistroMock[] {
  return tabla === "variantes" ? album.variantes : album.principal;
}

function buscarRegistro(album: AlbumMock, tabla: Tabla, id: number): RegistroMock {
  const r = tablaDe(album, tabla).find((x) => x.id === id);
  if (!r) throw `No existe el registro ${id} en ${tabla}`;
  return r;
}

function infoDe(album: AlbumMock): AlbumInfo {
  return {
    albumId: album.albumId,
    ruta: album.ruta,
    nombre: album.nombre,
    totalRegistros: album.principal.length,
    tieneVariantes: album.campos.some((c) => c.tabla === "variantes"),
    campos: album.campos,
  };
}

function aLigero(album: AlbumMock, tabla: Tabla, r: RegistroMock): RegistroLigero {
  // Etiquetas (multidato): la grilla recibe los valores reales unidos con
  // " · " (igual que el GROUP_CONCAT del backend), no el conteo interno.
  const valores = { ...r.valores };
  for (const c of album.campos) {
    if (c.tabla === tabla && c.tipo === "multidato") {
      const lista = r.multidatos[c.nombre] ?? [];
      valores[c.nombre] = lista.length > 0 ? lista.join(" · ") : null;
    }
  }
  return {
    id: r.id,
    imagen: r.imagen,
    imagenVersion: r.imagen ? r.imagenVersion : null,
    tieneVariantes:
      tabla === "principal" &&
      album.variantes.some((v) => v.idPrincipal === r.id),
    oculto: r.oculto,
    valores,
  };
}

function aCompleto(tabla: Tabla, r: RegistroMock): RegistroCompleto {
  return {
    id: r.id,
    tabla,
    imagen: r.imagen,
    imagenVersion: r.imagen ? r.imagenVersion : null,
    valores: r.valores,
    multidatos: r.multidatos,
  };
}

/** Sincroniza los conteos de campos multidato en `valores`. */
function sincronizarConteos(album: AlbumMock, tabla: Tabla, r: RegistroMock): void {
  for (const c of album.campos) {
    if (c.tabla === tabla && c.tipo === "multidato") {
      r.valores[c.nombre] = (r.multidatos[c.nombre] ?? []).length;
    }
  }
}

function defCompleta(album: AlbumMock, def: CampoNuevo, id: number): CampoDef {
  return {
    id,
    nombre: def.nombre,
    colFisica: `f_${id}`,
    tabla: def.tabla,
    tipo: def.tipo,
    decimales: def.decimales ?? 0,
    totalizable: def.totalizable ?? false,
    formula: def.formula ?? null,
    visible: def.visible ?? true,
    modificable: def.modificable ?? true,
    ordenVisible: def.ordenVisible ?? album.campos.length,
    formato: def.formato ?? null,
  };
}

const espera = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Diálogo de archivos simulado: decide la ruta según los filtros pedidos. */
function dialogoAbrir(options: unknown): string {
  const exts: string[] =
    (options as { filters?: { extensions: string[] }[] })?.filters?.flatMap(
      (f) => f.extensions,
    ) ?? [];
  if (exts.includes("micdb")) return "/demo/Catálogo Demo.micdb";
  if (exts.includes("mdb")) return "/demo/viejo.mdb";
  if (exts.includes("csv")) return "/demo/importar-demo.csv";
  if (exts.includes("xlsx")) return "/demo/importar-demo.xlsx";
  return "/demo/imagenes/nueva-imagen.jpg";
}

type Payload = Record<string, never> & Record<string, unknown>;

/* eslint-disable @typescript-eslint/no-explicit-any */
async function manejar(cmd: string, p: any): Promise<unknown> {
  switch (cmd) {
    // --- Plugins de Tauri ---
    case "plugin:dialog|open":
      return dialogoAbrir(p.options);
    case "plugin:dialog|save": {
      const exts: string[] =
        (p.options as { filters?: { extensions: string[] }[] })?.filters?.flatMap(
          (f: { extensions: string[] }) => f.extensions,
        ) ?? [];
      const ext = exts[0] ?? "micdb";
      return `/demo/salida-demo.${ext}`;
    }
    case "plugin:opener|open_url":
    case "plugin:opener|open_path":
      return null;
    // BaseDirectory (Desktop=18, Documents=7…): el mock responde una ruta fija.
    case "plugin:path|resolve_directory":
      return "/demo/Escritorio";

    // --- Álbum ---
    case "album_crear": {
      const album = crearAlbumDemo(proximoAlbumId++, p.ruta);
      album.nombre = p.nombre;
      album.principal = [];
      album.variantes = [];
      album.grupos = [];
      album.filtros = new Map();
      album.categorias = new Map();
      album.campos = (p.campos as CampoNuevo[]).map((d, i) =>
        defCompleta(album, { ...d, ordenVisible: i }, i + 1),
      );
      album.proximoCampoId = album.campos.length + 1;
      album.proximoId = 1;
      albunes.set(album.albumId, album);
      return infoDe(album);
    }
    case "album_abrir": {
      const existente = [...albunes.values()].find((a) => a.ruta === p.ruta);
      if (existente) return infoDe(existente);
      const album = crearAlbumDemo(proximoAlbumId++, p.ruta);
      const base = String(p.ruta).split("/").pop()?.replace(/\.micdb$/, "");
      if (base && base !== "demo") album.nombre = base;
      albunes.set(album.albumId, album);
      return infoDe(album);
    }
    case "album_cerrar":
      albunes.delete(p.albumId);
      return null;
    case "album_compactar":
      await espera(300);
      return null;
    case "albumes_recientes":
      return [
        { ruta: "/demo/Catálogo Demo.micdb", nombre: "Catálogo Demo" },
        { ruta: "/demo/Bodegas.micdb", nombre: "Bodegas" },
      ];

    // --- Campos ---
    case "campos_listar":
      return obtenerAlbum(p.albumId).campos;
    case "campo_crear": {
      const a = obtenerAlbum(p.albumId);
      const def = defCompleta(a, p.def, a.proximoCampoId++);
      a.campos.push(def);
      for (const r of tablaDe(a, def.tabla)) {
        r.valores[def.nombre] = def.tipo === "multidato" ? 0 : null;
        if (def.tipo === "multidato") r.multidatos[def.nombre] = [];
      }
      return def;
    }
    case "campo_editar": {
      const a = obtenerAlbum(p.albumId);
      const def = a.campos.find((c) => c.id === p.campoId);
      if (!def) throw `No existe el campo ${p.campoId}`;
      const anterior = def.nombre;
      Object.assign(def, p.def as CampoNuevo);
      if (anterior !== def.nombre) {
        for (const r of tablaDe(a, def.tabla)) {
          if (anterior in r.valores) {
            r.valores[def.nombre] = r.valores[anterior];
            delete r.valores[anterior];
          }
          if (anterior in r.multidatos) {
            r.multidatos[def.nombre] = r.multidatos[anterior];
            delete r.multidatos[anterior];
          }
        }
      }
      for (const r of tablaDe(a, def.tabla)) recalcular(r, a.campos, def.tabla);
      return def;
    }
    case "campo_eliminar": {
      const a = obtenerAlbum(p.albumId);
      const idx = a.campos.findIndex((c) => c.id === p.campoId);
      if (idx < 0) throw `No existe el campo ${p.campoId}`;
      const [def] = a.campos.splice(idx, 1);
      for (const r of tablaDe(a, def.tabla)) {
        delete r.valores[def.nombre];
        delete r.multidatos[def.nombre];
      }
      return null;
    }
    case "campos_reordenar": {
      const a = obtenerAlbum(p.albumId);
      (p.orden as number[]).forEach((id, i) => {
        const def = a.campos.find((c) => c.id === id);
        if (def) def.ordenVisible = i;
      });
      a.campos.sort((x, y) => x.ordenVisible - y.ordenVisible);
      return null;
    }
    case "formula_probar": {
      const a = obtenerAlbum(p.albumId);
      return evaluarFormula(p.formula, p.valores as Valores, a.campos);
    }

    // --- Registros ---
    case "registros_query": {
      const a = obtenerAlbum(p.albumId);
      const req = p.req as QueryReq;
      const todos = ejecutarQuery(a, req);
      const pagina = todos.slice(req.offset, req.offset + req.limit);
      return {
        total: todos.length,
        offset: req.offset,
        registros: pagina.map((r) => aLigero(a, req.tabla, r)),
      };
    }
    case "registro_obtener":
      return aCompleto(p.tabla, buscarRegistro(obtenerAlbum(p.albumId), p.tabla, p.id));
    case "registro_crear": {
      const a = obtenerAlbum(p.albumId);
      const tabla = p.tabla as Tabla;
      const r: RegistroMock = {
        id: a.proximoId++,
        idPrincipal: tabla === "variantes" ? (p.idPrincipal ?? null) : null,
        imagen: p.imagenOrigen
          ? `imagenes/${String(p.imagenOrigen).split("/").pop()}`
          : null,
        imagenVersion: 1,
        oculto: false,
        valores: { ...(p.valores as Valores) },
        multidatos: { ...((p.multidatos ?? {}) as Record<string, string[]>) },
      };
      sincronizarConteos(a, tabla, r);
      recalcular(r, a.campos, tabla);
      tablaDe(a, tabla).push(r);
      return r.id;
    }
    case "registro_editar": {
      const a = obtenerAlbum(p.albumId);
      const r = buscarRegistro(a, p.tabla, p.id);
      Object.assign(r.valores, p.valores as Valores);
      if (p.multidatos) {
        r.multidatos = { ...(p.multidatos as Record<string, string[]>) };
      }
      sincronizarConteos(a, p.tabla, r);
      recalcular(r, a.campos, p.tabla);
      return aCompleto(p.tabla, r);
    }
    case "registros_eliminar": {
      const a = obtenerAlbum(p.albumId);
      const ids = new Set(p.ids as number[]);
      if (p.tabla === "principal") {
        a.principal = a.principal.filter((r) => !ids.has(r.id));
        a.variantes = a.variantes.filter((v) => !ids.has(v.idPrincipal ?? -1));
      } else {
        a.variantes = a.variantes.filter((r) => !ids.has(r.id));
      }
      return null;
    }
    case "registro_imagen_set": {
      const a = obtenerAlbum(p.albumId);
      const r = buscarRegistro(a, p.tabla, p.id);
      r.imagen = `imagenes/${String(p.rutaOrigen).split("/").pop()}`;
      r.imagenVersion++;
      return { imagen: r.imagen, imagenVersion: r.imagenVersion };
    }
    case "registros_editar_lote": {
      const a = obtenerAlbum(p.albumId);
      for (const id of p.ids as number[]) {
        const r = buscarRegistro(a, p.tabla, id);
        Object.assign(r.valores, p.valores as Valores);
        recalcular(r, a.campos, p.tabla);
      }
      return null;
    }

    // --- Ocultar / totalizar / actualización masiva ---
    case "registros_set_auxiliar": {
      const a = obtenerAlbum(p.albumId);
      for (const id of p.ids as number[]) {
        buscarRegistro(a, p.tabla, id).oculto = p.oculto as boolean;
      }
      return null;
    }
    case "registros_totalizar": {
      const a = obtenerAlbum(p.albumId);
      const req = p.req as QueryReq;
      const regs = ejecutarQuery(a, req);
      const totalizables = a.campos.filter(
        (c) =>
          c.tabla === req.tabla &&
          c.totalizable &&
          c.tipo !== "multidato" &&
          c.tipo !== "texto",
      );
      return {
        registros: regs.length,
        totales: totalizables.map((c) => ({
          campo: c.nombre,
          suma:
            Math.round(
              regs.reduce((acc, r) => {
                const v = r.valores[c.nombre];
                return acc + (typeof v === "number" ? v : Number(v) || 0);
              }, 0) * 100,
            ) / 100,
        })),
      };
    }
    case "registros_estadisticas": {
      const a = obtenerAlbum(p.albumId);
      const req = p.req as QueryReq;
      const regs = ejecutarQuery(a, req);
      const defs = (p.campos as string[])
        .map((n) => a.campos.find((c) => c.nombre === n))
        .filter(
          (c): c is CampoDef =>
            !!c &&
            c.tabla === req.tabla &&
            (c.tipo === "numerico" ||
              c.tipo === "moneda" ||
              c.tipo === "calculado"),
        );
      const stats = defs.map((c) => {
        const vals = regs
          .map((r) => r.valores[c.nombre])
          .filter((v): v is number => typeof v === "number")
          .sort((x, y) => x - y);
        const n = vals.length;
        const suma = vals.reduce((s, v) => s + v, 0);
        const frec = new Map<number, number>();
        for (const v of vals) frec.set(v, (frec.get(v) ?? 0) + 1);
        const [moda, modaConteo] = [...frec.entries()].sort(
          (x, y) => y[1] - x[1] || x[0] - y[0],
        )[0] ?? [null, 0];
        return {
          campo: c.nombre,
          cuenta: n,
          suma,
          media: n ? suma / n : null,
          mediana: n
            ? n % 2
              ? vals[(n - 1) / 2]
              : (vals[n / 2 - 1] + vals[n / 2]) / 2
            : null,
          moda,
          modaConteo,
          minimo: n ? vals[0] : null,
          maximo: n ? vals[n - 1] : null,
        };
      });
      return { registros: regs.length, campos: stats };
    }
    case "registros_actualizar_masivo": {
      const a = obtenerAlbum(p.albumId);
      const req = p.req as QueryReq;
      const regs = ejecutarQuery(a, req);
      for (const r of regs) {
        Object.assign(r.valores, p.valores as Valores);
        recalcular(r, a.campos, req.tabla);
      }
      return regs.length;
    }

    case "registros_crear_desde_carpeta": {
      const a = obtenerAlbum(p.albumId);
      // Simula 5 imágenes encontradas en la carpeta elegida.
      for (let i = 0; i < 5; i++) {
        const id = a.proximoId++;
        a.principal.push({
          id,
          idPrincipal: null,
          imagen: `imagenes/carpeta_${id}.jpg`,
          imagenVersion: 1,
          oculto: false,
          valores: Object.fromEntries(
            a.campos
              .filter((c) => c.tabla === "principal")
              .map((c) => [c.nombre, c.tipo === "multidato" ? 0 : null]),
          ),
          multidatos: {},
        });
      }
      await espera(400);
      return 5;
    }

    // --- Empacar / desempacar álbum ---
    case "album_empacar":
      await espera(400);
      return 42;
    case "album_desempacar":
      await espera(400);
      return "/demo/desempacado/Catálogo Demo.micdb";

    // --- Plantillas de álbum ---
    case "plantillas_listar":
      return [...plantillasMock.values()];
    case "plantilla_guardar": {
      plantillasMock.set(p.nombre, {
        nombre: p.nombre,
        campos: p.campos as CampoNuevo[],
      });
      return null;
    }
    case "plantilla_eliminar":
      plantillasMock.delete(p.nombre);
      return null;

    // --- Recalcular / copiar álbum ---
    case "album_recalcular": {
      const a = obtenerAlbum(p.albumId);
      for (const r of a.principal) recalcular(r, a.campos, "principal");
      for (const r of a.variantes) recalcular(r, a.campos, "variantes");
      return a.principal.length + a.variantes.length;
    }
    case "album_copiar": {
      const a = obtenerAlbum(p.albumId);
      await espera(500);
      return p.soloEstructura ? 0 : a.principal.filter((r) => r.imagen).length;
    }

    // --- Variantes ---
    case "variantes_listar": {
      const a = obtenerAlbum(p.albumId);
      return a.variantes
        .filter((v) => v.idPrincipal === p.idPrincipal)
        .map((v) => aLigero(a, "variantes", v));
    }

    // --- Multidatos y categorías ---
    case "categorias_sugerir": {
      const a = obtenerAlbum(p.albumId);
      const def = a.campos.find((c) => c.id === p.campoId);
      const pre = normalizar(p.prefijo ?? "");
      const set = new Set<string>();
      for (const c of a.categorias.get(`${p.campoId}:${p.principal}`) ?? []) {
        set.add(c.valor);
      }
      if (def) {
        const regs = p.principal ? a.principal : a.variantes;
        for (const r of regs) {
          for (const v of r.multidatos[def.nombre] ?? []) set.add(v);
        }
      }
      return [...set]
        .filter((v) => normalizar(v).startsWith(pre))
        .sort((x, y) => x.localeCompare(y, "es"))
        .slice(0, 20);
    }
    case "categorias_listar":
      return (
        obtenerAlbum(p.albumId).categorias.get(`${p.campoId}:${p.principal}`) ??
        []
      );
    case "categorias_actualizar":
      obtenerAlbum(p.albumId).categorias.set(
        `${p.campoId}:${p.principal}`,
        p.valores as CategoriaVal[],
      );
      return null;

    // --- Grupos ---
    case "grupos_listar":
      return obtenerAlbum(p.albumId).grupos;
    case "grupo_guardar": {
      const a = obtenerAlbum(p.albumId);
      const g = p.grupo as Grupo;
      if (g.id === 0) {
        const nuevo = { ...g, id: a.proximoGrupoId++ };
        a.grupos.push(nuevo);
        return nuevo.id;
      }
      const idx = a.grupos.findIndex((x) => x.id === g.id);
      if (idx < 0) throw `No existe el grupo ${g.id}`;
      a.grupos[idx] = g;
      return g.id;
    }
    case "grupo_eliminar": {
      const a = obtenerAlbum(p.albumId);
      a.grupos = a.grupos.filter((g) => g.id !== p.grupoId);
      return null;
    }
    case "grupo_arbol":
      return construirArbol(obtenerAlbum(p.albumId), p.grupoId);

    // --- Álbumes ligados ---
    case "ligados_listar":
      return [...ligasMock.values()];
    case "liga_guardar": {
      const liga = p.liga as LigaMock;
      if (liga.id === 0) {
        const nueva = { ...liga, id: proximaLigaId++ };
        ligasMock.set(nueva.id, nueva);
        return nueva.id;
      }
      ligasMock.set(liga.id, liga);
      return liga.id;
    }
    case "liga_eliminar":
      ligasMock.delete(p.ligaId);
      return null;
    case "liga_actualizar": {
      for (let hechas = 0; hechas <= 50; hechas += 25) {
        await emit("liga-progreso", { hechas, total: 50 });
        await espera(266);
      }
      return { actualizados: 42, creados: 3, sinCoincidencia: 5 };
    }
    case "ligas_actualizar_todas":
      return [{ actualizados: 42, creados: 3, sinCoincidencia: 5 }];

    // --- Filtros guardados ---
    case "filtros_listar":
      return [...obtenerAlbum(p.albumId).filtros.keys()].sort();
    case "filtro_obtener": {
      const f = obtenerAlbum(p.albumId).filtros.get(p.nombre);
      if (!f) throw `No existe el filtro "${p.nombre}"`;
      return f;
    }
    case "filtro_guardar":
      obtenerAlbum(p.albumId).filtros.set(
        p.nombre,
        p.condiciones as CondicionFiltro[],
      );
      return null;
    case "filtro_eliminar":
      obtenerAlbum(p.albumId).filtros.delete(p.nombre);
      return null;

    // --- Exportación ---
    case "exportar_registros": {
      await espera(400);
      return ejecutarQuery(obtenerAlbum(p.albumId), p.req as QueryReq).length;
    }

    // --- Importación de registros ---
    case "importar_inspeccionar": {
      const a = obtenerAlbum(p.albumId);
      await espera(250);
      const esXlsx = String(p.rutaArchivo).endsWith(".xlsx");
      // Toma las primeras columnas principales del álbum demo y añade una
      // columna de ruido para ejercitar el aviso de "ignoradas".
      const cols = a.campos
        .filter((c) => c.tabla === "principal")
        .slice(0, 4)
        .map((c) => c.nombre);
      const elegibles = a.campos
        .filter(
          (c) =>
            c.tabla === "principal" &&
            c.tipo !== "calculado" &&
            c.tipo !== "multidato",
        )
        .map((c) => c.nombre);
      return {
        columnas: [...cols, "ColumnaDesconocida"],
        totalFilas: 120,
        encoding: esXlsx ? "utf-8" : "utf-8-bom",
        formato: esXlsx ? "xlsx" : "csv",
        columnasReconocidas: cols,
        columnasNoReconocidas: ["ColumnaDesconocida"],
        camposLlaveSugeridos: elegibles,
        huella: "12345:1717545600",
      };
    }
    case "importar_registros": {
      const fase = p.dryRun ? "analizando" : "aplicando";
      for (let hechas = 0; hechas <= 120; hechas += 40) {
        await emit("importacion-progreso", { fase, hechas, total: 120 });
        await espera(180);
      }
      return p.dryRun
        ? {
            actualizados: 80,
            creados: 25,
            sinCambio: 15,
            errores: [],
            avisos: [
              "llave duplicada en el archivo: 'A-1' (se usó la primera fila)",
            ],
            dryRun: true,
          }
        : {
            actualizados: 80,
            creados: 25,
            sinCambio: 15,
            errores: [],
            avisos: [],
            dryRun: false,
          };
    }

    // --- Miniaturas ---
    case "thumb_invalidar":
      return null;

    // --- Reportes (impresión) ---
    case "reportes_listar":
      return [...(reportesMock.get(p.albumId) ?? [])].sort((a, b) =>
        a.nombre.localeCompare(b.nombre, "es"),
      );
    case "reporte_guardar": {
      const lista = reportesMock.get(p.albumId) ?? [];
      const idx = lista.findIndex((r) => r.nombre === p.nombre);
      if (idx >= 0) lista[idx] = { nombre: p.nombre, config: p.config };
      else lista.push({ nombre: p.nombre, config: p.config });
      reportesMock.set(p.albumId, lista);
      return null;
    }
    case "reporte_eliminar":
      reportesMock.set(
        p.albumId,
        (reportesMock.get(p.albumId) ?? []).filter((r) => r.nombre !== p.nombre),
      );
      return null;

    // --- Migración ---
    case "migracion_verificar_mdbtools":
      return true;
    case "migracion_inspeccionar":
      await espera(400);
      return {
        tablas: ["propiedades", "Principal", "Variantes", "Multidatos"],
        campos: [
          { nombre: "Clave", tipo: "TC_TEXTO" },
          { nombre: "Nombre", tipo: "TC_TEXTO" },
          { nombre: "Precio", tipo: "TC_MONEDA" },
          { nombre: "Fecha Alta", tipo: "TC_FECHA" },
        ],
        totalEstimado: 1850,
        tieneVariantes: true,
      };
    case "migracion_ejecutar": {
      const total = 1850;
      for (const fase of ["Leyendo estructura", "Copiando registros", "Imágenes"]) {
        for (let hechas = 0; hechas <= total; hechas += 370) {
          await emit("migracion-progreso", { fase, hechas, total });
          await espera(120);
        }
      }
      return {
        filasPrincipal: 1850,
        filasVariantes: 320,
        filasMultidatos: 940,
        imagenesEncontradas: 1714,
        imagenesFaltantes: ["G:\\MIC\\imagenes\\perdida_034.jpg"],
        advertencias: ["2 fórmulas recalculadas con diferencias de redondeo"],
      };
    }

    default:
      throw `Comando no soportado por el mock: ${cmd}`;
  }
}
/* eslint-enable @typescript-eslint/no-explicit-any */

/** Marca global que consultan otros módulos (p. ej. miniaturas). */
declare global {
  interface Window {
    __MIC_MOCK__?: boolean;
  }
}

/** Instala el mock IPC. Llamar antes de montar la app. */
export function instalarMock(): void {
  window.__MIC_MOCK__ = true;
  mockIPC((cmd, payload) => manejar(cmd, payload as Payload), {
    shouldMockEvents: true,
  });
  console.info("[MIC] Mock IPC instalado — modo navegador de desarrollo");
}
