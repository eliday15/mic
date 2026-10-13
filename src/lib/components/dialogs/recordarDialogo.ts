/**
 * Recuerda la última configuración USADA de un diálogo por álbum (campos y
 * formato de exportación, opciones de impresión…): se guarda al exportar o
 * imprimir, no al cancelar. Vive en `localStorage`
 * porque es una preferencia local de la máquina, no un dato del álbum; si el
 * almacenamiento no está disponible (modo privado, cuota llena) se ignora.
 */

const PREFIJO = "mic:dialogo";

/** Clave estable: sección del diálogo + identidad del álbum (ruta o nombre). */
function clave(seccion: string, album: string): string {
  return `${PREFIJO}:${seccion}:${album}`;
}

/** Lee la última configuración recordada (o null si no hay o no es legible). */
export function leerRecordado<T>(seccion: string, album: string): Partial<T> | null {
  try {
    const crudo = localStorage.getItem(clave(seccion, album));
    if (crudo === null) return null;
    const valor: unknown = JSON.parse(crudo);
    return valor !== null && typeof valor === "object" ? (valor as Partial<T>) : null;
  } catch {
    return null;
  }
}

/** Recuerda la configuración actual del diálogo para el álbum. */
export function guardarRecordado<T>(seccion: string, album: string, valor: T): void {
  try {
    localStorage.setItem(clave(seccion, album), JSON.stringify(valor));
  } catch {
    // Sin almacenamiento disponible: la preferencia simplemente no persiste.
  }
}

/** Selección de campos recordada: elegidos (en orden) + todos los que existían. */
export interface CamposRecordados {
  campos: string[];
  /** Todos los campos elegibles al guardar: distingue los creados después. */
  conocidos: string[];
}

/**
 * Restaura una selección recordada contra los campos actuales: descarta los
 * que ya no existen y añade al final, marcados, los creados después de
 * guardarla (no se excluyen en silencio). Null si no hay nada recordado.
 */
export function restaurarCampos(
  recordado: Partial<CamposRecordados> | null,
  existentes: string[],
): string[] | null {
  if (!recordado || !Array.isArray(recordado.campos)) return null;
  const validos = new Set(existentes);
  const elegidos = recordado.campos.filter(
    (n): n is string => typeof n === "string" && validos.has(n),
  );
  // Sin `conocidos` (guardado antiguo) no se puede saber qué es nuevo.
  const conocidos = new Set(
    Array.isArray(recordado.conocidos) ? recordado.conocidos : existentes,
  );
  const nuevos = existentes.filter((n) => !conocidos.has(n) && !elegidos.includes(n));
  return [...elegidos, ...nuevos];
}
