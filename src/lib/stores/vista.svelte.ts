/**
 * Estado de presentación de los paneles del álbum y bus de acciones de la barra
 * de herramientas / menú hacia la vista activa (`AlbumView`).
 *
 * La barra superior (Toolbar/MenuBar) vive en el shell, por encima de la vista
 * del álbum; para que sus botones (nueva imagen, eliminar, alternar paneles)
 * lleguen a la vista sin acoplarse a su instancia, se publican aquí como
 * señales: un contador por acción que `AlbumView` observa con `$effect`.
 *
 * La visibilidad de los paneles laterales (sidebar izquierdo e inspector
 * derecho) es estado compartido directo, ya que tanto el shell (iconos toggle)
 * como la vista lo consultan.
 */

class StoreVista {
  /** Sidebar izquierdo (Grupos/Filtros) visible. */
  sidebarVisible = $state(true);
  /** Inspector derecho visible. */
  inspectorVisible = $state(true);

  // --- Señales de acción (contadores incrementales) ----------------------
  /** Solicitud de "nueva imagen / registro". */
  senalNuevaImagen = $state(0);
  /** Solicitud de "nueva variante" del registro seleccionado. */
  senalNuevaVariante = $state(0);
  /** Solicitud de "editar registro" seleccionado. */
  senalEditar = $state(0);
  /** Solicitud de "eliminar selección". */
  senalEliminar = $state(0);
  /** Solicitud de "seleccionar todo". */
  senalSeleccionarTodo = $state(0);
  /** Solicitud de "deseleccionar todo". */
  senalDeseleccionar = $state(0);
  /** Solicitud de "invertir selección". */
  senalInvertirSeleccion = $state(0);
  /** Solicitud de "ocultar selección" (`_auxiliar_`). */
  senalOcultar = $state(0);
  /** Solicitud de "mostrar selección" (quitar oculto). */
  senalMostrar = $state(0);
  /** Solicitud de abrir el visor de imagen al 100 %. */
  senalVisor = $state(0);

  alternarSidebar(): void {
    this.sidebarVisible = !this.sidebarVisible;
  }
  alternarInspector(): void {
    this.inspectorVisible = !this.inspectorVisible;
  }

  nuevaImagen(): void {
    this.senalNuevaImagen++;
  }
  nuevaVariante(): void {
    this.senalNuevaVariante++;
  }
  editar(): void {
    this.senalEditar++;
  }
  eliminar(): void {
    this.senalEliminar++;
  }
  seleccionarTodo(): void {
    this.senalSeleccionarTodo++;
  }
  deseleccionar(): void {
    this.senalDeseleccionar++;
  }
  invertirSeleccion(): void {
    this.senalInvertirSeleccion++;
  }
  ocultar(): void {
    this.senalOcultar++;
  }
  mostrar(): void {
    this.senalMostrar++;
  }
  abrirVisor(): void {
    this.senalVisor++;
  }
}

/** Instancia única del store de presentación. */
export const vista = new StoreVista();
