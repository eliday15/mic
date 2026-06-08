/**
 * Store del tema visual de la aplicación.
 *
 * Tema oscuro por defecto. Persiste la elección del usuario en `localStorage`
 * y aplica el atributo `data-theme` en el `<html>` para activar los tokens CSS.
 */

export type Tema = "dark" | "light";

const CLAVE_LS = "mic.tema";
const TEMA_DEFECTO: Tema = "dark";

/** Lee el tema guardado, con respaldo al predeterminado. */
function leerTemaGuardado(): Tema {
  if (typeof localStorage === "undefined") return TEMA_DEFECTO;
  const guardado = localStorage.getItem(CLAVE_LS);
  return guardado === "light" || guardado === "dark" ? guardado : TEMA_DEFECTO;
}

/** Aplica el tema al documento (atributo `data-theme` en `<html>`). */
function aplicarAlDocumento(tema: Tema): void {
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("data-theme", tema);
  }
}

class StoreTema {
  /** Tema activo (reactivo). */
  tema = $state<Tema>(leerTemaGuardado());

  /** True si el tema activo es oscuro. */
  get esOscuro(): boolean {
    return this.tema === "dark";
  }

  /** Aplica el tema actual al documento. Llamar al montar la app. */
  inicializar(): void {
    aplicarAlDocumento(this.tema);
  }

  /** Fija un tema concreto y lo persiste. */
  fijar(tema: Tema): void {
    this.tema = tema;
    aplicarAlDocumento(tema);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(CLAVE_LS, tema);
    }
  }

  /** Alterna entre oscuro y claro. */
  alternar(): void {
    this.fijar(this.tema === "dark" ? "light" : "dark");
  }
}

/** Instancia única del store de tema. */
export const tema = new StoreTema();
