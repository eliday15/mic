/**
 * Mock de `importar_inspeccionar` / `importar_registros` (modo navegador).
 *
 * Espejo de `crates/mic-tauri/src/commands/importar.rs`: llave de importación
 * campo ↔ columna, filas sin coincidencia / llave vacía / llave repetida con
 * número de fila, políticas sustituir/mantener/rellenar_vacios, alta de
 * faltantes, columnas extra (tipo inferido; con `importarExtras` se crean los
 * campos ANTES de aplicar) y dry-run que recorre exactamente la misma lógica.
 *
 * El "diálogo de archivos" del mock abre un `<input type=file>` real
 * (`#mic-mock-archivo`) y guarda el CSV leído en memoria bajo `/subidos/<nombre>`.
 */

import { emit } from "@tauri-apps/api/event";
import type { CampoDef, CampoNuevo, Valor, Valores } from "$lib/domain/types";
import type { AlbumMock, RegistroMock } from "./datos";
import {
  igualAscii,
  inferirTipo,
  leerArchivoCsv,
  parsearFecha,
  parsearNumero,
  type ArchivoMock,
} from "./importarCsv";

/** Operaciones del mock general que la importación reutiliza. */
export interface OpsImport {
  crearCampo(def: CampoNuevo): CampoDef;
  crearRegistro(valores: Valores, multidatos: Record<string, string[]>): void;
  editarRegistro(r: RegistroMock, valores: Valores, multidatos: Record<string, string[]> | null): void;
}

interface FilaLlave {
  fila: number;
  llave: string;
}
interface ListaFilas {
  total: number;
  filas: FilaLlave[];
}
interface ColumnaExtra {
  columna: string;
  tipo: CampoDef["tipo"];
  decimales: number;
}

const LIMITE_DETALLE = 500;
const archivos = new Map<string, ArchivoMock>();
const espera = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Resolución pendiente del selector anterior (se cierra con `null` al reabrir). */
let pendiente: ((ruta: string | null) => void) | null = null;

/**
 * Abre el selector real del navegador y resuelve la ruta virtual del CSV leído.
 * No se resuelve con el evento `cancel`: Chrome headless lo dispara al instante
 * (antes de que la automatización suba el archivo); un selector abandonado se
 * resuelve con `null` al abrir el siguiente.
 */
export function elegirArchivoImportacion(): Promise<string | null> {
  pendiente?.(null);
  pendiente = null;
  document.getElementById("mic-mock-archivo")?.remove();
  const input = document.createElement("input");
  input.type = "file";
  input.id = "mic-mock-archivo";
  input.accept = ".csv,.xlsx";
  input.style.display = "none";
  document.body.appendChild(input);
  return new Promise((resolve, reject) => {
    pendiente = resolve;
    input.addEventListener("change", async () => {
      pendiente = null;
      const f = input.files?.[0];
      if (!f) return resolve(null);
      try {
        const leido = await leerArchivoCsv(f);
        archivos.set(`/subidos/${f.name}`, leido);
        resolve(`/subidos/${f.name}`);
      } catch (e) {
        reject(e);
      }
    });
    input.click();
  });
}

function obtenerArchivo(ruta: string): ArchivoMock {
  if (ruta.trim() === "") throw "no se indicó ningún archivo";
  const a = archivos.get(ruta);
  if (!a) throw `no se encontró el archivo '${ruta}'`;
  return a;
}

const principales = (album: AlbumMock) => album.campos.filter((c) => c.tabla === "principal");
const esLlaveElegible = (c: CampoDef) =>
  c.tabla === "principal" && c.tipo !== "calculado" && c.tipo !== "multidato";
const campoParaColumna = (album: AlbumMock, col: string) =>
  principales(album).find((c) => igualAscii(c.nombre, col));

function agregar(lista: ListaFilas, fila: number, llave: string): void {
  lista.total++;
  if (lista.filas.length < LIMITE_DETALLE) lista.filas.push({ fila, llave });
}

function detectarExtras(album: AlbumMock, a: ArchivoMock, indiceLlave: number | null): ColumnaExtra[] {
  const vistos = new Set<string>();
  const extras: ColumnaExtra[] = [];
  a.encabezados.forEach((col, i) => {
    const nombre = col.trim();
    const clave = nombre.toLowerCase();
    if (nombre === "" || vistos.has(clave)) return;
    vistos.add(clave);
    if (i === indiceLlave || album.campos.some((c) => igualAscii(c.nombre, nombre))) return;
    const { tipo, decimales } = inferirTipo(a.filas.map((f) => f[i] ?? ""), a.separador);
    extras.push({ columna: nombre, tipo, decimales });
  });
  return extras;
}

export async function inspeccionar(album: AlbumMock, ruta: string) {
  await espera(250);
  const a = obtenerArchivo(ruta);
  const reconocidas: string[] = [];
  const noReconocidas: string[] = [];
  const sugeridas: string[] = [];
  for (const col of a.encabezados) {
    const campo = campoParaColumna(album, col);
    if (!campo) noReconocidas.push(col);
    else {
      reconocidas.push(col);
      if (esLlaveElegible(campo)) sugeridas.push(col);
    }
  }
  return {
    columnas: a.encabezados,
    totalFilas: a.filas.length,
    encoding: a.encoding,
    formato: "csv",
    columnasReconocidas: reconocidas,
    columnasNoReconocidas: noReconocidas,
    camposLlaveSugeridos: sugeridas,
    camposLlaveAlbum: album.campos.filter(esLlaveElegible).map((c) => c.nombre),
    columnasExtra: detectarExtras(album, a, null),
    huella: a.huella,
  };
}

/** Clave canónica (numérica sin `.0`), como `clave_canonica` del backend. */
function claveCanonica(v: Valor | string | undefined, numerica: boolean): string | null {
  if (v === null || v === undefined) return null;
  const t = String(v).trim();
  if (t === "") return null;
  if (numerica) {
    const n = typeof v === "number" ? v : /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(t) ? Number(t) : NaN;
    if (Number.isFinite(n)) return String(n);
    if (typeof v === "number") return null;
  }
  return t;
}

/** Celda normalizada: `undefined` = omitir; string[] = multidato. */
function normalizar(texto: string, campo: CampoDef, sep: string): Valor | string[] | undefined {
  const limpio = texto.replace(/^[\r\n]+|[\r\n]+$/g, "");
  if (campo.tipo === "calculado") return undefined;
  if (campo.tipo === "multidato") {
    const vals = limpio.split("|").map((s) => s.trim()).filter((s) => s !== "");
    return vals.length > 0 ? vals : undefined;
  }
  if (limpio.trim() === "") return undefined;
  if (campo.tipo === "numerico" || campo.tipo === "moneda") {
    const n = parsearNumero(limpio, sep);
    if (n === null) throw `'${limpio}' no es un número válido para '${campo.nombre}'`;
    return n;
  }
  if (campo.tipo === "fecha") {
    const iso = parsearFecha(limpio);
    if (!iso) throw `fecha inválida '${limpio}' en '${campo.nombre}' (use AAAA-MM-DD o dd/MM/aaaa)`;
    return iso;
  }
  return limpio.trim();
}

function estaVacio(v: Valor | undefined, campo: CampoDef, multi: string[] | undefined): boolean {
  if (campo.tipo === "multidato") return (multi ?? []).length === 0;
  if (campo.tipo === "numerico" || campo.tipo === "moneda") return v === null || v === undefined;
  return v === null || v === undefined || String(v).trim() === "";
}

interface OpcionesImport {
  campoLlave: string;
  columnaLlave?: string | null;
  politica: "sustituir" | "mantener" | "rellenar_vacios";
  crearFaltantes: boolean;
  importarExtras?: boolean;
  dryRun: boolean;
  huella?: string;
}

export async function importar(album: AlbumMock, ruta: string, op: OpcionesImport, ops: OpsImport) {
  const a = obtenerArchivo(ruta);
  if (op.huella && op.huella !== a.huella) {
    throw "el archivo cambió desde el análisis; vuelve a ver el resumen";
  }
  if (!["sustituir", "mantener", "rellenar_vacios"].includes(op.politica)) {
    throw `política de importación no soportada: '${op.politica}'`;
  }
  const campoLlave = principales(album).find((c) => igualAscii(c.nombre, op.campoLlave));
  if (!campoLlave) throw `el campo llave '${op.campoLlave}' no existe en el álbum`;
  if (!esLlaveElegible(campoLlave)) {
    throw `el campo llave '${campoLlave.nombre}' no puede ser calculado ni multidato`;
  }
  const columna = op.columnaLlave?.trim() ? op.columnaLlave : op.campoLlave;
  const indicesLlave = a.encabezados.flatMap((c, i) => (igualAscii(c, columna) ? [i] : []));
  if (indicesLlave.length === 0) throw `la columna llave '${columna}' no está presente en el archivo`;
  if (indicesLlave.length > 1) throw `la columna llave '${columna}' aparece duplicada en el archivo`;
  const iLlave = indicesLlave[0];

  // Extras: al aplicar se crean los campos primero (refresco implícito: el mock
  // lee `album.campos` en vivo); en dry-run se simulan.
  const columnasExtra = detectarExtras(album, a, iLlave);
  const camposCreados: string[] = [];
  let campos = [...album.campos];
  if (op.importarExtras) {
    columnasExtra.forEach((e, i) => {
      const def: CampoNuevo = { nombre: e.columna, tabla: "principal", tipo: e.tipo, decimales: e.decimales };
      if (op.dryRun) {
        campos.push({ ...def, id: -(i + 1), colFisica: "", decimales: e.decimales, totalizable: false,
          formula: null, visible: true, modificable: true, ordenVisible: 0, formato: null });
      } else {
        ops.crearCampo({ ...def, ordenVisible: album.campos.length });
      }
      camposCreados.push(e.columna);
    });
    if (!op.dryRun) campos = [...album.campos];
  }

  const avisos: string[] = [];
  const errores: string[] = [];
  const porColumna: (CampoDef | null)[] = [];
  const vistos = new Set<string>();
  let calculados = false;
  a.encabezados.forEach((col, i) => {
    const clave = col.trim().toLowerCase();
    if (vistos.has(clave)) {
      avisos.push(`columna '${col}' duplicada en el archivo: se ignoran las repetidas`);
      porColumna.push(null);
      return;
    }
    vistos.add(clave);
    const campo = i === iLlave ? null : campos.find((c) => c.tabla === "principal" && igualAscii(c.nombre, col));
    if (campo && campo.nombre === campoLlave.nombre) {
      avisos.push(`la columna '${col}' corresponde al campo llave y se ignora (la llave se toma de '${a.encabezados[iLlave]}')`);
      porColumna.push(null);
    } else if (campo?.tipo === "calculado") {
      calculados = true;
      porColumna.push(null);
    } else porColumna.push(campo ?? null);
  });
  const noReconocidas = a.encabezados.filter(
    (c, i) => i !== iLlave && !campos.some((x) => x.tabla === "principal" && igualAscii(x.nombre, c)),
  );
  if (noReconocidas.length > 0) {
    avisos.push(`columnas ignoradas (no coinciden con ningún campo): ${noReconocidas.join(", ")}`);
  }
  if (calculados) avisos.push("columnas calculadas ignoradas: se recalculan automáticamente");

  // Índice del álbum: clave → registro (el más antiguo gana).
  const numerica = campoLlave.tipo === "numerico" || campoLlave.tipo === "moneda";
  const indice = new Map<string, RegistroMock>();
  const duplicadasAlbum = new Set<string>();
  for (const r of [...album.principal].sort((x, y) => x.id - y.id)) {
    const k = claveCanonica(r.valores[campoLlave.nombre], numerica);
    if (k === null) continue;
    if (indice.has(k)) duplicadasAlbum.add(k);
    else indice.set(k, r);
  }

  const res = {
    actualizados: 0, creados: 0, sinCambio: 0, coincidentes: 0, omitidos: 0,
    noCoincidentes: { total: 0, filas: [] } as ListaFilas,
    llavesVacias: { total: 0, filas: [] } as ListaFilas,
    llavesDuplicadas: { total: 0, filas: [] } as ListaFilas,
    columnasExtra,
    camposCreados, errores, avisos, dryRun: op.dryRun,
  };
  for (const k of duplicadasAlbum) {
    avisos.push(`la llave '${k}' aparece varias veces en el álbum; solo se actualizó una`);
  }

  const total = a.filas.length;
  const fase = op.dryRun ? "analizando" : "aplicando";
  await emit("importacion-progreso", { fase, hechas: 0, total });
  const claves = new Set<string>();
  for (const [idx, fila] of a.filas.entries()) {
    const numFila = idx + 2;
    const clave = claveCanonica(fila[iLlave] ?? "", numerica);
    if (clave === null) {
      agregar(res.llavesVacias, numFila, "");
      continue;
    }
    if (claves.has(clave)) {
      agregar(res.llavesDuplicadas, numFila, clave);
      continue;
    }
    claves.add(clave);

    const escalares: Valores = {};
    const multis: Record<string, string[]> = {};
    porColumna.forEach((campo, i) => {
      if (!campo) return;
      try {
        const v = normalizar(fila[i] ?? "", campo, a.separador);
        if (Array.isArray(v)) multis[campo.nombre] = v;
        else if (v !== undefined) escalares[campo.nombre] = v;
      } catch (e) {
        errores.push(`fila ${numFila}: ${String(e)}`);
      }
    });

    const existente = indice.get(clave);
    if (existente) {
      res.coincidentes++;
      if (op.politica === "mantener") {
        res.sinCambio++;
        continue;
      }
      const nuevos: Valores = {};
      for (const [nombre, v] of Object.entries(escalares)) {
        const campo = campos.find((c) => c.nombre === nombre)!;
        const actual = existente.valores[nombre];
        if (op.politica === "rellenar_vacios" && !estaVacio(actual, campo, undefined)) continue;
        if (actual !== v) nuevos[nombre] = v;
      }
      let multiNuevos: Record<string, string[]> | null = null;
      for (const [nombre, v] of Object.entries(multis)) {
        const actuales = existente.multidatos[nombre] ?? [];
        if (op.politica === "rellenar_vacios" && actuales.length > 0) continue;
        if (actuales.join("|") !== v.join("|")) multiNuevos = { ...(multiNuevos ?? existente.multidatos), [nombre]: v };
      }
      if (Object.keys(nuevos).length === 0 && !multiNuevos) {
        res.sinCambio++;
        continue;
      }
      if (!op.dryRun) ops.editarRegistro(existente, nuevos, multiNuevos);
      res.actualizados++;
    } else {
      agregar(res.noCoincidentes, numFila, clave);
      if (!op.crearFaltantes) {
        res.omitidos++;
        continue;
      }
      if (!op.dryRun) {
        ops.crearRegistro({ ...escalares, [campoLlave.nombre]: numerica ? Number(clave) : clave }, multis);
      }
      res.creados++;
    }
    if ((idx + 1) % 50 === 0) await emit("importacion-progreso", { fase, hechas: idx + 1, total });
  }
  await espera(300);
  await emit("importacion-progreso", { fase, hechas: total, total });
  return res;
}
