/**
 * Tipos del sistema de impresión / reportes (ex frmprint / frmprint2 /
 * frmPreliminar + clsReporteCI / clsReporteSI del VB6).
 *
 * Una `ConfigReporte` describe por completo cómo renderizar e imprimir un
 * reporte: con imágenes (catálogo, "ci") o sin ellas (tabla densa, "si"). Se
 * persiste como JSON en la tabla `reportes` del álbum.
 */

/** Tipo de reporte: con imágenes (catálogo) o sin imágenes (tabla). */
export type TipoReporte = "ci" | "si";

/** Imágenes por línea de la cuadrícula del reporte con imágenes. */
export type ImagenesPorLinea = 1 | 2 | 4 | 8;

/** Orientación de la hoja impresa. */
export type Orientacion = "vertical" | "horizontal";

/** Tamaño de papel (mapea a `@page size`). */
export type Papel = "carta" | "oficio" | "a4";

/** Configuración completa de un reporte imprimible. */
export interface ConfigReporte {
  /** Con imágenes ("ci") o sin imágenes ("si"). */
  tipo: TipoReporte;
  /** Título del documento (vacío = nombre del álbum). */
  titulo: string;
  /** Nombres visibles de los campos a imprimir, en orden. */
  campos: string[];
  /** Imágenes por línea (solo aplica al tipo "ci"). */
  imagenesPorLinea: ImagenesPorLinea;
  orientacion: Orientacion;
  papel: Papel;
  /** Imprime la fecha en la cabecera. */
  ponFecha: boolean;
  /** Imprime el número de página en el pie (vía CSS counters). */
  ponPagina: boolean;
  /** Imprime los totales (suma de los campos totalizables). */
  ponTotales: boolean;
  /** Campo de agrupación con subtotales (solo "si"); null = sin agrupar. */
  agrupacion: string | null;
}

/** Una configuración de reporte guardada (nombre + config). */
export interface ReporteGuardado {
  nombre: string;
  config: ConfigReporte;
}

/** Mapea el papel del dominio al valor CSS de `@page size`. */
export function papelCss(papel: Papel): string {
  switch (papel) {
    case "oficio":
      return "legal";
    case "a4":
      return "A4";
    default:
      return "letter";
  }
}

/** Configuración por defecto de un reporte nuevo. */
export function configPorDefecto(): ConfigReporte {
  return {
    tipo: "ci",
    titulo: "",
    campos: [],
    imagenesPorLinea: 4,
    orientacion: "vertical",
    papel: "carta",
    ponFecha: true,
    ponPagina: true,
    ponTotales: false,
    agrupacion: null,
  };
}
