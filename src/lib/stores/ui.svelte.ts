/**
 * Store de estado de interfaz global: modal activo, toasts y ocupación global.
 *
 * Mantiene una pila lógica simple (un modal a la vez) y una lista de toasts con
 * autodescarte. El contador `busy` permite mostrar un indicador global cuando
 * hay operaciones de fondo en curso (varias pueden solaparse).
 */

/** Severidad visual de un toast. */
export type TipoToast = "info" | "exito" | "error" | "aviso";

/** Un mensaje efímero mostrado en la esquina. */
export interface Toast {
  id: number;
  tipo: TipoToast;
  mensaje: string;
  /** Milisegundos antes de autodescartarse (0 = persistente). */
  duracion: number;
}

/** Duración por defecto de un toast, en milisegundos. */
const DURACION_DEFECTO = 4000;

class StoreUI {
  /** Identificador del modal activo (`null` = ninguno). */
  modal = $state<string | null>(null);
  /** Datos opcionales asociados al modal activo. */
  modalProps = $state<Record<string, unknown> | null>(null);
  /** Toasts visibles. */
  toasts = $state<Toast[]>([]);
  /** Contador de operaciones de fondo en curso. */
  private contadorBusy = $state(0);

  private siguienteId = 1;
  private temporizadores = new Map<number, ReturnType<typeof setTimeout>>();

  /** True si hay al menos una operación global en curso. */
  get busy(): boolean {
    return this.contadorBusy > 0;
  }

  // -- Modales -----------------------------------------------------------

  /** Abre un modal por su identificador, con props opcionales. */
  abrirModal(id: string, props?: Record<string, unknown>): void {
    this.modal = id;
    this.modalProps = props ?? null;
  }

  /** Cierra el modal activo. */
  cerrarModal(): void {
    this.modal = null;
    this.modalProps = null;
  }

  /** True si el modal indicado es el activo. */
  esModalActivo(id: string): boolean {
    return this.modal === id;
  }

  // -- Toasts ------------------------------------------------------------

  /**
   * Empuja un toast a la lista. Devuelve su id (útil para descartarlo manual).
   */
  push(
    mensaje: string,
    tipo: TipoToast = "info",
    duracion = DURACION_DEFECTO,
  ): number {
    const id = this.siguienteId++;
    this.toasts = [...this.toasts, { id, tipo, mensaje, duracion }];
    if (duracion > 0) {
      const tmr = setTimeout(() => this.dismiss(id), duracion);
      this.temporizadores.set(id, tmr);
    }
    return id;
  }

  /** Atajo: toast de éxito. */
  exito(mensaje: string, duracion = DURACION_DEFECTO): number {
    return this.push(mensaje, "exito", duracion);
  }

  /** Atajo: toast de error (persiste algo más por defecto). */
  error(mensaje: string, duracion = 6000): number {
    return this.push(mensaje, "error", duracion);
  }

  /** Atajo: toast de aviso. */
  aviso(mensaje: string, duracion = DURACION_DEFECTO): number {
    return this.push(mensaje, "aviso", duracion);
  }

  /** Descarta un toast por id. */
  dismiss(id: number): void {
    const tmr = this.temporizadores.get(id);
    if (tmr) {
      clearTimeout(tmr);
      this.temporizadores.delete(id);
    }
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  /** Descarta todos los toasts. */
  limpiarToasts(): void {
    for (const tmr of this.temporizadores.values()) clearTimeout(tmr);
    this.temporizadores.clear();
    this.toasts = [];
  }

  // -- Ocupación global --------------------------------------------------

  /** Marca el inicio de una operación de fondo. */
  iniciarBusy(): void {
    this.contadorBusy++;
  }

  /** Marca el fin de una operación de fondo. */
  terminarBusy(): void {
    if (this.contadorBusy > 0) this.contadorBusy--;
  }

  /**
   * Ejecuta una promesa marcando ocupación global durante su transcurso.
   * Reemite el error tras restaurar el contador.
   */
  async conBusy<T>(tarea: () => Promise<T>): Promise<T> {
    this.iniciarBusy();
    try {
      return await tarea();
    } finally {
      this.terminarBusy();
    }
  }
}

/** Instancia única del store de UI. */
export const ui = new StoreUI();
