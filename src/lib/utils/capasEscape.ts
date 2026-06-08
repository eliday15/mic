/**
 * capasEscape — pila global de "capas" que responden a Escape (modales,
 * lightbox, etc.) con un único listener en window (fase de burbuja).
 *
 * Resuelve el caso en que un cambio de fase interno de un diálogo deja el
 * foco en <body>: el keydown ya no pasa por el div del modal y Escape queda
 * muerto. Con la pila, Escape siempre cierra SOLO la capa superior, y los
 * handlers de elementos (Combobox, lightbox en captura…) siguen teniendo
 * prioridad: si detienen la propagación, la capa no se entera.
 */

type CapaEscape = () => void;

const pila: CapaEscape[] = [];
let escuchando = false;

function onKeydown(e: KeyboardEvent): void {
  if (e.key !== "Escape" || pila.length === 0) return;
  e.preventDefault();
  pila[pila.length - 1]();
}

/**
 * Registra una capa que se cierra con Escape; la última registrada es la que
 * responde. Devuelve la función para des-registrarla (usar en el cleanup del
 * `$effect`).
 */
export function registrarCapaEscape(cerrar: CapaEscape): () => void {
  pila.push(cerrar);
  if (!escuchando) {
    window.addEventListener("keydown", onKeydown);
    escuchando = true;
  }
  return () => {
    const i = pila.lastIndexOf(cerrar);
    if (i >= 0) pila.splice(i, 1);
    if (pila.length === 0 && escuchando) {
      window.removeEventListener("keydown", onKeydown);
      escuchando = false;
    }
  };
}
