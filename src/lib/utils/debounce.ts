/**
 * Utilidades de control de frecuencia: `debounce` y `throttle`.
 *
 * Pensadas para entradas de búsqueda, autocomplete y refrescos de scroll.
 */

/** Función con método `cancel` para abortar una llamada pendiente. */
export interface FuncionDebounced<A extends unknown[]> {
  (...args: A): void;
  /** Cancela cualquier ejecución pendiente. */
  cancel(): void;
  /** Ejecuta inmediatamente con los últimos argumentos, si los hay. */
  flush(): void;
}

/**
 * Devuelve una versión de `fn` que se ejecuta solo tras `ms` milisegundos sin
 * nuevas llamadas. La última invocación gana.
 *
 * @param fn Función a retrasar.
 * @param ms Tiempo de espera en milisegundos.
 */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  ms: number,
): FuncionDebounced<A> {
  let temporizador: ReturnType<typeof setTimeout> | null = null;
  let ultimos: A | null = null;

  const wrapped = ((...args: A) => {
    ultimos = args;
    if (temporizador !== null) clearTimeout(temporizador);
    temporizador = setTimeout(() => {
      temporizador = null;
      const a = ultimos;
      ultimos = null;
      if (a) fn(...a);
    }, ms);
  }) as FuncionDebounced<A>;

  wrapped.cancel = () => {
    if (temporizador !== null) {
      clearTimeout(temporizador);
      temporizador = null;
    }
    ultimos = null;
  };

  wrapped.flush = () => {
    if (temporizador !== null) {
      clearTimeout(temporizador);
      temporizador = null;
    }
    const a = ultimos;
    ultimos = null;
    if (a) fn(...a);
  };

  return wrapped;
}

/**
 * Devuelve una versión de `fn` que se ejecuta como máximo una vez cada `ms`
 * milisegundos (primer disparo inmediato, resto descartado en la ventana).
 *
 * @param fn Función a limitar.
 * @param ms Ventana mínima entre ejecuciones, en milisegundos.
 */
export function throttle<A extends unknown[]>(
  fn: (...args: A) => void,
  ms: number,
): (...args: A) => void {
  let ultimo = 0;
  return (...args: A) => {
    const ahora = Date.now();
    if (ahora - ultimo >= ms) {
      ultimo = ahora;
      fn(...args);
    }
  };
}
