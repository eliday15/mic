/**
 * Formato de números, moneda y fechas con `Intl` en locale es-MX.
 *
 * Todos los formateadores producen cifras tabulares (alineables en columnas) y
 * usan caché de instancias `Intl.*` por configuración para evitar recrearlas.
 */

import type { Valor } from "$lib/domain/types";

const LOCALE = "es-MX";

/** Caché de formateadores numéricos por nº de decimales. */
const cacheNumero = new Map<number, Intl.NumberFormat>();
/** Caché de formateadores de moneda por nº de decimales. */
const cacheMoneda = new Map<number, Intl.NumberFormat>();

/** Formateador de número con `decimales` fijos de presentación. */
function fmtNumero(decimales: number): Intl.NumberFormat {
  let f = cacheNumero.get(decimales);
  if (!f) {
    f = new Intl.NumberFormat(LOCALE, {
      minimumFractionDigits: decimales,
      maximumFractionDigits: decimales,
      useGrouping: true,
    });
    cacheNumero.set(decimales, f);
  }
  return f;
}

/** Formateador de moneda (MXN) con `decimales` fijos. */
function fmtMoneda(decimales: number): Intl.NumberFormat {
  let f = cacheMoneda.get(decimales);
  if (!f) {
    f = new Intl.NumberFormat(LOCALE, {
      style: "currency",
      currency: "MXN",
      minimumFractionDigits: decimales,
      maximumFractionDigits: decimales,
    });
    cacheMoneda.set(decimales, f);
  }
  return f;
}

/** Convierte un `Valor` a número, o `null` si no es numérico. */
function aNumeroOpc(valor: Valor): number | null {
  if (valor === null || valor === "") return null;
  if (typeof valor === "number") return Number.isFinite(valor) ? valor : null;
  if (typeof valor === "boolean") return valor ? 1 : 0;
  const limpio = String(valor).trim().replace(/,/g, "");
  const n = Number(limpio);
  return Number.isNaN(n) ? null : n;
}

/**
 * Formatea un número con `decimales` de presentación. Devuelve cadena vacía
 * para valores no numéricos o nulos.
 */
export function formatearNumero(valor: Valor, decimales = 2): string {
  const n = aNumeroOpc(valor);
  return n === null ? "" : fmtNumero(decimales).format(n);
}

/**
 * Formatea un valor monetario en MXN con `decimales` de presentación.
 */
export function formatearMoneda(valor: Valor, decimales = 2): string {
  const n = aNumeroOpc(valor);
  return n === null ? "" : fmtMoneda(decimales).format(n);
}

/**
 * Formatea una fecha ISO `YYYY-MM-DD` al formato corto es-MX (dd/mm/aaaa).
 * Devuelve cadena vacía si el valor no es una fecha ISO válida.
 */
export function formatearFecha(valor: Valor): string {
  if (typeof valor !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(valor)) {
    return "";
  }
  const [anio, mes, dia] = valor.split("-").map(Number);
  const d = new Date(Date.UTC(anio, mes - 1, dia));
  if (Number.isNaN(d.getTime())) return "";
  return new Intl.DateTimeFormat(LOCALE, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    timeZone: "UTC",
  }).format(d);
}

/**
 * Formatea un valor según el tipo de campo, para mostrar en grilla/tabla.
 *
 * @param valor     Valor crudo del registro.
 * @param tipo      Tipo del campo.
 * @param decimales Decimales de presentación/redondeo (por defecto 2).
 * @param formato   Formato de presentación: 'moneda' | 'porcentaje' | null.
 */
export function formatearPorTipo(
  valor: Valor,
  tipo: string,
  decimales = 2,
  formato: string | null = null,
): string {
  if (valor === null) return "";
  // El formato de presentación manda sobre el tipo, pero solo aplica a tipos
  // numéricos (número/moneda/calculado): un texto jamás se "porcentajiza".
  const esNumericoTipo =
    tipo === "numerico" || tipo === "moneda" || tipo === "calculado";
  if (formato === "porcentaje" && esNumericoTipo) {
    const n = formatearNumero(valor, decimales);
    return n === "" ? "" : `${n} %`;
  }
  if (formato === "moneda" && esNumericoTipo) {
    return formatearMoneda(valor, decimales);
  }
  switch (tipo) {
    case "numerico":
    case "calculado":
      return formatearNumero(valor, decimales);
    case "moneda":
      return formatearMoneda(valor, decimales);
    case "fecha":
      return formatearFecha(valor);
    default:
      return String(valor);
  }
}

/** Formatea un entero (conteos, totales de registros) con separador de miles. */
export function formatearEntero(n: number): string {
  return fmtNumero(0).format(n);
}

/** Fecha actual en formato ISO `YYYY-MM-DD` (hora local). */
export function hoyIso(): string {
  const d = new Date();
  const mes = String(d.getMonth() + 1).padStart(2, "0");
  const dia = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mes}-${dia}`;
}
