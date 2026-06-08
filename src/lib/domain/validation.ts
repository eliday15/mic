/**
 * Constructores de esquemas Zod por tipo de campo configurable.
 *
 * Reglas (acordes al modelo: la longitud nunca restringe, solo el formato):
 *  - texto      → string sin límite de longitud.
 *  - numerico   → number con `decimales` de presentación (no restringe rango).
 *  - moneda     → number con `decimales` de presentación.
 *  - fecha      → string ISO `YYYY-MM-DD` con fecha de calendario válida.
 *  - calculado  → solo lectura: no valida entrada (el backend lo recalcula).
 *  - multidato  → conteo entero ≥ 0 en `valores`; los valores van por separado.
 *
 * Los campos no obligatorios admiten `null` (valor nulo). El frontend usa estos
 * esquemas para validar el editor de registros antes de invocar al backend.
 */

import { z, type ZodTypeAny } from "zod";
import type { CampoDef, TipoCampo, Valor } from "$lib/domain/types";

/** Expresión de una fecha ISO `YYYY-MM-DD` bien formada. */
const RE_FECHA_ISO = /^\d{4}-\d{2}-\d{2}$/;

/** Comprueba que una cadena ISO corresponde a una fecha de calendario real. */
function esFechaCalendarioValida(iso: string): boolean {
  if (!RE_FECHA_ISO.test(iso)) return false;
  const [anio, mes, dia] = iso.split("-").map(Number);
  if (mes < 1 || mes > 12 || dia < 1 || dia > 31) return false;
  const d = new Date(Date.UTC(anio, mes - 1, dia));
  return (
    d.getUTCFullYear() === anio &&
    d.getUTCMonth() === mes - 1 &&
    d.getUTCDate() === dia
  );
}

/** Acepta un valor que puede venir como número o como cadena numérica. */
function aNumero(valor: unknown): number | typeof valor {
  if (typeof valor === "number") return valor;
  if (typeof valor === "string") {
    const limpio = valor.trim().replace(/,/g, "");
    if (limpio === "") return valor;
    const n = Number(limpio);
    return Number.isNaN(n) ? valor : n;
  }
  return valor;
}

/**
 * Redondea un número a la cantidad de decimales de presentación del campo.
 * No restringe el valor: solo lo normaliza al formato esperado.
 */
function redondear(n: number, decimales: number): number {
  if (decimales <= 0) return Math.round(n);
  const factor = 10 ** decimales;
  return Math.round((n + Number.EPSILON) * factor) / factor;
}

// ---------------------------------------------------------------------------
// Constructores por tipo
// ---------------------------------------------------------------------------

/** Esquema de un campo de texto: string sin límite de longitud. */
export function esquemaTexto(): ZodTypeAny {
  return z.string();
}

/**
 * Esquema de un campo numérico (o moneda): number normalizado a `decimales`
 * de presentación. Acepta cadenas numéricas y las coacciona.
 */
export function esquemaNumero(decimales: number): ZodTypeAny {
  return z.preprocess(
    aNumero,
    z
      .number({ message: "Debe ser un número" })
      .finite("Número fuera de rango")
      .transform((n) => redondear(n, decimales)),
  );
}

/** Esquema de un campo fecha: ISO `YYYY-MM-DD` válida del calendario. */
export function esquemaFecha(): ZodTypeAny {
  return z
    .string()
    .regex(RE_FECHA_ISO, "Use el formato AAAA-MM-DD")
    .refine(esFechaCalendarioValida, "Fecha inexistente");
}

/**
 * Esquema de un campo calculado: solo lectura. No valida la entrada del usuario
 * (el backend lo recalcula), por lo que admite cualquier valor o `null`.
 */
export function esquemaCalculado(): ZodTypeAny {
  return z.any();
}

/** Esquema del conteo de un campo multidato dentro de `valores`. */
export function esquemaMultidato(): ZodTypeAny {
  return z.preprocess(
    aNumero,
    z.number().int("Conteo inválido").nonnegative("Conteo inválido"),
  );
}

// ---------------------------------------------------------------------------
// Despachador por definición de campo
// ---------------------------------------------------------------------------

/** Devuelve el esquema base (sin opcionalidad) para un tipo de campo. */
function esquemaBasePorTipo(tipo: TipoCampo, decimales: number): ZodTypeAny {
  switch (tipo) {
    case "texto":
      return esquemaTexto();
    case "numerico":
    case "moneda":
      return esquemaNumero(decimales);
    case "fecha":
      return esquemaFecha();
    case "calculado":
      return esquemaCalculado();
    case "multidato":
      return esquemaMultidato();
  }
}

/**
 * Construye el esquema de validación de un campo a partir de su definición.
 * Todo campo admite `null` (valor vacío); los calculados nunca validan.
 *
 * @param campo Definición del campo.
 * @returns Esquema Zod que valida el `Valor` de ese campo.
 */
export function esquemaCampo(campo: CampoDef): ZodTypeAny {
  if (campo.tipo === "calculado") return esquemaCalculado();
  return esquemaBasePorTipo(campo.tipo, campo.decimales).nullable();
}

/**
 * Construye un esquema de objeto para validar todos los `valores` editables de
 * un registro. Excluye los campos calculados (solo lectura).
 *
 * @param campos Definiciones de campos del álbum.
 * @returns Esquema Zod de objeto `Record<nombre, Valor>`.
 */
export function esquemaRegistro(campos: CampoDef[]): ZodTypeAny {
  const forma: Record<string, ZodTypeAny> = {};
  for (const campo of campos) {
    if (campo.tipo === "calculado") continue;
    forma[campo.nombre] = esquemaCampo(campo).optional();
  }
  return z.object(forma).passthrough();
}

/** Resultado de validar un único valor. */
export interface ResultadoValidacion {
  ok: boolean;
  /** Valor normalizado (redondeado / coaccionado) si `ok`. */
  valor?: Valor;
  /** Mensaje de error en español si `!ok`. */
  error?: string;
}

/**
 * Valida y normaliza un único valor contra la definición de su campo.
 *
 * @param campo Definición del campo.
 * @param valor Valor a validar.
 * @returns Resultado con el valor normalizado o un mensaje de error.
 */
export function validarValor(campo: CampoDef, valor: Valor): ResultadoValidacion {
  if (campo.tipo === "calculado") return { ok: true, valor };
  if (valor === null || valor === "") return { ok: true, valor: null };

  const res = esquemaCampo(campo).safeParse(valor);
  if (res.success) return { ok: true, valor: res.data as Valor };
  return { ok: false, error: res.error.issues[0]?.message ?? "Valor inválido" };
}
