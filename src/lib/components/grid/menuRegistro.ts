/**
 * Opciones del menú contextual de un registro, compartidas por la grilla de
 * miniaturas y la vista de tabla (ambas despachan los mismos ids de acción a
 * `AlbumView`).
 */

import { t } from "$lib/i18n/es";
import type { OpcionMenu } from "$lib/components/ui";

/** Menú del registro; "Ocultar" pasa a "Mostrar" si el registro está oculto. */
export function opcionesMenuRegistro(oculto: boolean): OpcionMenu[] {
  return [
    { id: "editar", etiqueta: t.editar.editarRegistro },
    { id: "ver100", etiqueta: t.ver.visor },
    { id: "variantes", etiqueta: t.registro.variantes },
    { id: "sep", separador: true },
    { id: "excluir", etiqueta: t.filtros.quitarDelFiltro },
    oculto
      ? { id: "mostrar", etiqueta: t.editar.mostrar }
      : { id: "ocultar", etiqueta: t.editar.ocultar },
    { id: "eliminar", etiqueta: t.accion.eliminar, peligro: true },
  ];
}
