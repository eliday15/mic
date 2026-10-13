/**
 * Evaluador de fórmulas del mock: espejo del motor Rust (`mic-core::calc`).
 *
 * - Nombres de campo con letras ASCII, dígitos y `_` (espacios → `_`); sin
 *   llaves. Otros calculados se evalúan en línea (como la expansión de
 *   Module5.bas), con detección de ciclos.
 * - Las fechas se reducen a días desde 1900-01-01: `Fin - Inicio` da días,
 *   `Fecha + 30` da días que un calculado con `formato: "fecha"` devuelve como
 *   fecha ISO. `@FECHA(campo)` y `@HOY` (paréntesis opcionales).
 * - Campo numérico vacío = 0 (VB); campo fecha vacío → resultado vacío (null).
 * - Errores (sintaxis, división por cero) se lanzan como `string`.
 */

import type { CampoDef, Valor, Valores } from "$lib/domain/types";
import { hoyIso } from "$lib/utils/format";

type Ast =
  | { k: "num"; n: number }
  | { k: "campo"; nombre: string }
  | { k: "neg"; arg: Ast }
  | { k: "bin"; op: string; izq: Ast; der: Ast }
  | { k: "fecha"; arg: Ast }
  | { k: "hoy" };

const MS_DIA = 86_400_000;
const EPOCH = Date.UTC(1900, 0, 1);

/** Días desde 1900-01-01 de una fecha (ISO, dd/mm/aaaa, mm/dd/aaaa...). */
function fechaADias(texto: string): number | null {
  const s = texto.trim();
  const formatos: [RegExp, (m: RegExpMatchArray) => [number, number, number]][] = [
    [/^(\d{4})-(\d{2})-(\d{2})$/, (m) => [+m[1], +m[2], +m[3]]],
    [/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/, (m) => [+m[3], +m[2], +m[1]]],
    [/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/, (m) => [+m[3], +m[1], +m[2]]],
    [/^(\d{4})\/(\d{1,2})\/(\d{1,2})$/, (m) => [+m[1], +m[2], +m[3]]],
    [/^(\d{1,2})-(\d{1,2})-(\d{4})$/, (m) => [+m[3], +m[2], +m[1]]],
  ];
  for (const [re, partes] of formatos) {
    const m = s.match(re);
    if (!m) continue;
    const [a, mes, d] = partes(m);
    const t = Date.UTC(a, mes - 1, d);
    const f = new Date(t);
    if (f.getUTCFullYear() === a && f.getUTCMonth() === mes - 1 && f.getUTCDate() === d) {
      return Math.round((t - EPOCH) / MS_DIA);
    }
  }
  return null;
}

/** Fecha ISO `YYYY-MM-DD` a partir de días desde 1900-01-01. */
export function diasAFecha(dias: number): string {
  return new Date(EPOCH + Math.round(dias) * MS_DIA).toISOString().slice(0, 10);
}

/** Tokeniza como `ObtenToken`: nombres, números, `@` y operadores sueltos. */
function tokenizar(formula: string): string[] {
  const tokens: string[] = [];
  const re = /\s*([A-Za-z][A-Za-z0-9_]*|[\d.]+|\S)/gy;
  let m: RegExpExecArray | null;
  while ((m = re.exec(formula)) !== null) {
    if (/^[\d.]+$/.test(m[1]) && /[A-Za-z]/.test(formula[re.lastIndex] ?? "")) {
      throw `token inválido en la fórmula: '${m[1]}${formula[re.lastIndex]}'`;
    }
    tokens.push(m[1]);
  }
  return tokens;
}

/** Parser de precedencia (`* /` antes que `+ -`, asociativo por la izquierda). */
function compilar(formula: string): Ast {
  const tk = tokenizar(formula);
  if (tk.length === 0) throw "la fórmula está vacía";
  let pos = 0;
  const esperar = (s: string): void => {
    const t = tk[pos++];
    if (t !== s) throw t === undefined ? `se esperaba '${s}' al final de la fórmula` : `se esperaba '${s}', se encontró '${t}'`;
  };
  const prefijo = (): Ast => {
    const t = tk[pos++];
    if (t === undefined) throw "se esperaba una expresión";
    if (/^[\d.]+$/.test(t)) {
      const n = Number(t);
      if (Number.isNaN(n)) throw `número inválido: '${t}'`;
      return { k: "num", n };
    }
    if (/^[A-Za-z]/.test(t)) return { k: "campo", nombre: t };
    if (t === "(") {
      const e = expr(0);
      esperar(")");
      return e;
    }
    if (t === "-") return { k: "neg", arg: prefijo() };
    if (t === "+") return prefijo();
    if (t === "@") {
      const fn = (tk[pos++] ?? "").toUpperCase();
      if (fn === "HOY") {
        if (tk[pos] === "(") {
          pos++;
          esperar(")");
        }
        return { k: "hoy" };
      }
      if (fn === "FECHA") {
        esperar("(");
        const arg = expr(0);
        esperar(")");
        return { k: "fecha", arg };
      }
      throw `función desconocida: '@${fn}'`;
    }
    throw `operador inesperado: '${t}'`;
  };
  const potencia = (op: string | undefined): number =>
    op === "+" || op === "-" ? 1 : op === "*" || op === "/" ? 2 : 0;
  const expr = (min: number): Ast => {
    let izq = prefijo();
    while (potencia(tk[pos]) > 0 && potencia(tk[pos]) >= min) {
      const op = tk[pos++];
      izq = { k: "bin", op, izq, der: expr(potencia(op) + 1) };
    }
    return izq;
  };
  const ast = expr(0);
  if (pos !== tk.length) throw `token inesperado tras la expresión: '${tk[pos]}'`;
  return ast;
}

/** Contexto de una evaluación: campos del álbum, valores y hoy en días. */
interface Ctx {
  campos: CampoDef[];
  valores: Valores;
  hoy: number;
  enCurso: Set<string>;
}

/** Busca la definición de un campo por su forma en fórmula (`_` = espacio). */
function defPorNombre(nombre: string, campos: CampoDef[]): CampoDef | undefined {
  const conEspacios = nombre.replace(/_/g, " ");
  return (
    campos.find((c) => c.nombre === conEspacios || c.nombre === nombre) ??
    // Los calculados se resuelven sin distinguir mayúsculas (indexar_calculados).
    campos.find(
      (c) => c.tipo === "calculado" && c.nombre.replace(/ /g, "_").toUpperCase() === nombre.toUpperCase(),
    )
  );
}

/** Valor numérico de un campo; `null` = fecha vacía (vacía el resultado). */
function valorCampo(nombre: string, ctx: Ctx): number | null {
  const def = defPorNombre(nombre, ctx.campos);
  if (def?.tipo === "calculado" && def.formula) {
    if (ctx.enCurso.has(def.nombre)) throw `ciclo en campos calculados: '${def.nombre}'`;
    ctx.enCurso.add(def.nombre);
    try {
      return evaluarAst(compilar(def.formula), ctx);
    } finally {
      ctx.enCurso.delete(def.nombre);
    }
  }
  const v: Valor = def ? (ctx.valores[def.nombre] ?? null) : (ctx.valores[nombre] ?? null);
  if (def?.tipo === "fecha") return typeof v === "string" ? fechaADias(v) : null;
  if (v === null || v === "") return 0;
  if (typeof v === "number") return v;
  if (typeof v === "boolean") return v ? 1 : 0;
  const dias = fechaADias(v);
  if (dias !== null) return dias;
  const n = Number(v.trim());
  return Number.isNaN(n) ? 0 : n;
}

function evaluarAst(ast: Ast, ctx: Ctx): number | null {
  switch (ast.k) {
    case "num":
      return ast.n;
    case "hoy":
      return ctx.hoy;
    case "campo":
      return valorCampo(ast.nombre, ctx);
    case "neg": {
      const v = evaluarAst(ast.arg, ctx);
      return v === null ? null : -v;
    }
    case "fecha": {
      if (ast.arg.k !== "campo") return evaluarAst(ast.arg, ctx);
      const def = defPorNombre(ast.arg.nombre, ctx.campos);
      const v = ctx.valores[def?.nombre ?? ast.arg.nombre] ?? null;
      if (v === null || (typeof v === "string" && v.trim() === "")) return null;
      if (typeof v !== "string") return typeof v === "number" ? v : 0;
      const dias = fechaADias(v);
      if (dias === null) throw `uso indebido de @FECHA: '${v}' no es una fecha`;
      return dias;
    }
    case "bin": {
      const a = evaluarAst(ast.izq, ctx);
      const b = evaluarAst(ast.der, ctx);
      if (a === null || b === null) return null;
      if (ast.op === "+") return a + b;
      if (ast.op === "-") return a - b;
      if (ast.op === "*") return a * b;
      if (b === 0) throw "división por cero";
      return a / b;
    }
  }
}

/** `true` si la fórmula usa `@HOY`, también a través de otros calculados
 *  (como `MotorCalculo::campos_con_hoy` sobre la fórmula expandida). */
export function usaHoy(
  formula: string,
  campos: CampoDef[],
  enCurso: Set<string> = new Set(),
): boolean {
  let ast: Ast;
  try {
    ast = compilar(formula);
  } catch {
    return false;
  }
  const recorre = (n: Ast): boolean => {
    switch (n.k) {
      case "hoy":
        return true;
      case "num":
        return false;
      case "neg":
      case "fecha":
        return recorre(n.arg);
      case "bin":
        return recorre(n.izq) || recorre(n.der);
      case "campo": {
        const def = defPorNombre(n.nombre, campos);
        if (def?.tipo !== "calculado" || !def.formula || enCurso.has(def.nombre)) return false;
        enCurso.add(def.nombre);
        const r = usaHoy(def.formula, campos, enCurso);
        enCurso.delete(def.nombre);
        return r;
      }
    }
  };
  return recorre(ast);
}

/**
 * Evalúa una fórmula con los valores de un registro. Con `formato: "fecha"`
 * el resultado (días) se devuelve como fecha ISO. Lanza `string` si falla.
 */
export function evaluarFormulaMock(
  formula: string,
  valores: Valores,
  campos: CampoDef[],
  formato: string | null = null,
): Valor {
  const ast = compilar(formula);
  const ctx: Ctx = { campos, valores, hoy: fechaADias(hoyIso()) ?? 0, enCurso: new Set() };
  const n = evaluarAst(ast, ctx);
  if (n === null || !Number.isFinite(n)) return null;
  // `@FECHA(x)` sola es una fecha (tipo de salida Fecha del original).
  if (formato === "fecha" || ast.k === "fecha") return diasAFecha(n);
  return n;
}
