/**
 * Lectura y normalización de CSV para el mock de importación (modo navegador).
 *
 * Espejo de `crates/mic-tauri/src/commands/importar.rs`: cascada de encoding
 * BOM → UTF-8 estricto → Windows-1252, autodetección de separador sobre el
 * encabezado, números es-MX / en-US, fechas ISO o dd/MM/aaaa, inferencia del
 * tipo de las columnas extra. El XLSX no se lee en el navegador.
 */

import type { TipoCampo } from "$lib/domain/types";

/** Un archivo "subido" al mock: texto ya decodificado + metadatos. */
export interface ArchivoMock {
  nombre: string;
  encabezados: string[];
  filas: string[][];
  encoding: "utf-8" | "utf-8-bom" | "windows-1252";
  separador: string;
  huella: string;
}

/** Decodifica los bytes en cascada (igual que `decodificar` del backend). */
function decodificar(bytes: Uint8Array): { texto: string; encoding: ArchivoMock["encoding"] } {
  if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
    return { texto: new TextDecoder("utf-8").decode(bytes.subarray(3)), encoding: "utf-8-bom" };
  }
  try {
    return { texto: new TextDecoder("utf-8", { fatal: true }).decode(bytes), encoding: "utf-8" };
  } catch {
    return { texto: new TextDecoder("windows-1252").decode(bytes), encoding: "windows-1252" };
  }
}

/** Separador de mayor frecuencia fuera de comillas en el encabezado (empate → coma). */
function detectarSeparador(texto: string): string {
  const primera = texto.split(/\r?\n/, 1)[0] ?? "";
  let enComillas = false;
  let comas = 0;
  let puntosComa = 0;
  let tabs = 0;
  for (const ch of primera) {
    if (ch === '"') enComillas = !enComillas;
    else if (!enComillas && ch === ",") comas++;
    else if (!enComillas && ch === ";") puntosComa++;
    else if (!enComillas && ch === "\t") tabs++;
  }
  const max = Math.max(comas, puntosComa, tabs);
  if (max === 0 || (comas >= puntosComa && comas >= tabs)) return ",";
  return puntosComa >= tabs ? ";" : "\t";
}

/** Parser CSV mínimo (RFC 4180: comillas, `""` escapado, saltos dentro de comillas). */
function parsearCsv(texto: string, sep: string): string[][] {
  const filas: string[][] = [];
  let fila: string[] = [];
  let celda = "";
  let enComillas = false;
  for (let i = 0; i < texto.length; i++) {
    const ch = texto[i];
    if (enComillas) {
      if (ch === '"' && texto[i + 1] === '"') {
        celda += '"';
        i++;
      } else if (ch === '"') enComillas = false;
      else celda += ch;
    } else if (ch === '"') enComillas = true;
    else if (ch === sep) {
      fila.push(celda);
      celda = "";
    } else if (ch === "\n" || ch === "\r") {
      if (ch === "\r" && texto[i + 1] === "\n") i++;
      fila.push(celda);
      filas.push(fila);
      fila = [];
      celda = "";
    } else celda += ch;
  }
  if (celda !== "" || fila.length > 0) {
    fila.push(celda);
    filas.push(fila);
  }
  return filas;
}

/** Lee un `File` del navegador como lo haría `leer_csv` del backend. */
export async function leerArchivoCsv(archivo: File): Promise<ArchivoMock> {
  if (!archivo.name.toLowerCase().endsWith(".csv")) {
    throw "En modo navegador solo se pueden importar archivos .csv";
  }
  const bytes = new Uint8Array(await archivo.arrayBuffer());
  const { texto: crudo, encoding } = decodificar(bytes);
  const texto = crudo.replace(/^﻿/, "");
  const separador = detectarSeparador(texto);
  const filas = parsearCsv(texto, separador);
  if (filas.length === 0) throw "el archivo está vacío (sin encabezados)";
  const encabezados = filas.shift()!.map((s) => s.replace(/^﻿/, "").trim());
  for (const f of filas) while (f.length < encabezados.length) f.push("");
  return {
    nombre: archivo.name,
    encabezados,
    filas,
    encoding,
    separador,
    huella: `${archivo.size}:${Math.floor(archivo.lastModified / 1000)}`,
  };
}

/** `eq_ignore_ascii_case` de Rust tras `trim` (no pliega acentos). */
export function igualAscii(a: string, b: string): boolean {
  const bajar = (s: string) => s.trim().replace(/[A-Z]/g, (c) => c.toLowerCase());
  return bajar(a) === bajar(b);
}

/** Literal que acepta `str::parse::<f64>()` (sin `inf`/`NaN`, que no aparecen en catálogos). */
const RE_F64 = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/;

/** Parsea a número con la misma limpieza que `parsear_numero` del backend. */
export function parsearNumero(texto: string, sep: string): number | null {
  let s = texto.replace(/[$€%  ]/g, "");
  if (s === "") return null;
  const punto = s.includes(".");
  const coma = s.includes(",");
  if (punto && coma) {
    s = sep === ";" ? s.replace(/\./g, "").replace(/,/g, ".") : s.replace(/,/g, "");
  } else if (coma) {
    s = (s.match(/,/g) ?? []).length > 1 ? s.replace(/,/g, "") : s.replace(",", ".");
  }
  s = s.trim();
  return RE_F64.test(s) ? Number(s) : null;
}

/** Fecha válida → ISO `AAAA-MM-DD`; `null` si no existe en el calendario. */
function isoSiValida(a: number, m: number, d: number): string | null {
  const f = new Date(Date.UTC(a, m - 1, d));
  if (f.getUTCFullYear() !== a || f.getUTCMonth() !== m - 1 || f.getUTCDate() !== d) return null;
  return `${String(a).padStart(4, "0")}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

/** Fecha ISO (con o sin hora) o dd/MM/aaaa, dd-MM-aaaa, dd/MM/aa → ISO (como `parsear_fecha`). */
export function parsearFecha(texto: string): string | null {
  const t = texto.trim();
  const iso = /^(\d{4})-(\d{1,2})-(\d{1,2})/.exec(t.slice(0, 10));
  if (iso && t.slice(0, 10).length === 10) {
    const r = isoSiValida(+iso[1], +iso[2], +iso[3]);
    if (r) return r;
  }
  const dmy = /^(\d{1,2})[/-](\d{1,2})[/-](\d{2}|\d{4})$/.exec(t);
  if (!dmy) return null;
  let anio = +dmy[3];
  // `%y` de chrono: 00–68 → 2000–2068, 69–99 → 1969–1999.
  if (dmy[3].length === 2) anio += anio <= 68 ? 2000 : 1900;
  return isoSiValida(anio, +dmy[2], +dmy[1]);
}

/**
 * Tipo del campo a crear para una columna extra (espejo de `inferir_tipo`):
 * numérico si todos los valores no vacíos son números (0 decimales si todos son
 * enteros), fecha si todos son fechas, texto si no. Cero a la izquierda → texto.
 */
export function inferirTipo(valores: string[], sep: string): { tipo: TipoCampo; decimales: number } {
  let hay = false;
  let numerico = true;
  let enteros = true;
  let fecha = true;
  for (const v of valores) {
    const t = v.trim();
    if (t === "") continue;
    hay = true;
    if (numerico) {
      const ceroIzq = t.length > 1 && t[0] === "0" && /\d/.test(t[1]);
      const n = parsearNumero(t, sep);
      if (n === null || !Number.isFinite(n) || ceroIzq) numerico = false;
      else if (!Number.isInteger(n)) enteros = false;
    }
    if (fecha && parsearFecha(t) === null) fecha = false;
    if (!numerico && !fecha) break;
  }
  if (hay && numerico) return { tipo: "numerico", decimales: enteros ? 0 : 2 };
  if (hay && fecha) return { tipo: "fecha", decimales: 0 };
  return { tipo: "texto", decimales: 0 };
}
