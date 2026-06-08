<!--
  Modal — diálogo modal con overlay, trampa de foco y cierre por Escape o clic
  fuera. El contenido va en el slot por defecto; el título y el pie son snippets
  opcionales.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { X } from "lucide-svelte";
  import { registrarCapaEscape } from "$lib/utils/capasEscape";

  type Ancho = "sm" | "md" | "lg" | "xl";

  interface Props {
    /** Controla la visibilidad (enlazado). */
    abierto?: boolean;
    titulo?: string;
    ancho?: Ancho;
    /** Permite cerrar al hacer clic en el overlay. */
    cerrarFuera?: boolean;
    /** Permite cerrar con Escape (apagar durante operaciones en curso). */
    cerrarEscape?: boolean;
    /** Muestra el botón de cierre en la cabecera. */
    botonCerrar?: boolean;
    onCerrar?: () => void;
    children: Snippet;
    pie?: Snippet;
  }

  let {
    abierto = $bindable(true),
    titulo,
    ancho = "md",
    cerrarFuera = true,
    cerrarEscape = true,
    botonCerrar = true,
    onCerrar,
    children,
    pie,
  }: Props = $props();

  let dialogo = $state<HTMLDivElement | null>(null);
  let previoFoco: HTMLElement | null = null;

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }

  /** Elementos enfocables dentro del diálogo. */
  function enfocables(): HTMLElement[] {
    if (!dialogo) return [];
    const sel =
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';
    return Array.from(dialogo.querySelectorAll<HTMLElement>(sel)).filter(
      (el) => el.offsetParent !== null,
    );
  }

  // Escape se maneja por la pila global de capas (ver $effect abajo): si el
  // foco queda en <body> tras un cambio de fase interno, el keydown nunca
  // pasaría por este div y Escape quedaría muerto. Aquí solo va el ciclo Tab.
  function onKeydown(e: KeyboardEvent): void {
    if (e.key !== "Tab") return;
    const items = enfocables();
    if (items.length === 0) {
      e.preventDefault();
      return;
    }
    const primero = items[0];
    const ultimo = items[items.length - 1];
    const activo = document.activeElement as HTMLElement | null;
    if (e.shiftKey && activo === primero) {
      e.preventDefault();
      ultimo.focus();
    } else if (!e.shiftKey && activo === ultimo) {
      e.preventDefault();
      primero.focus();
    }
  }

  // Trampa de foco: al abrir guarda el foco previo y enfoca el diálogo; al
  // cerrar lo restaura.
  $effect(() => {
    if (abierto) {
      previoFoco = document.activeElement as HTMLElement | null;
      queueMicrotask(() => {
        const items = enfocables();
        (items[0] ?? dialogo)?.focus();
      });
      return () => {
        previoFoco?.focus?.();
      };
    }
  });

  // Cierre por Escape vía pila global: la capa superior (este modal, o un
  // lightbox abierto encima que se registre después) es la única que responde.
  $effect(() => {
    if (abierto && cerrarEscape) {
      return registrarCapaEscape(cerrar);
    }
  });
</script>

{#if abierto}
  <div
    class="overlay"
    transition:fade={{ duration: 120 }}
    onclick={() => cerrarFuera && cerrar()}
    onkeydown={onKeydown}
    role="presentation"
  >
    <div
      bind:this={dialogo}
      class="modal modal--{ancho}"
      role="dialog"
      aria-modal="true"
      aria-label={titulo}
      tabindex="-1"
      transition:scale={{ duration: 140, start: 0.96 }}
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKeydown}
    >
      {#if titulo || botonCerrar}
        <header class="modal__head">
          {#if titulo}<h2 class="modal__titulo">{titulo}</h2>{/if}
          {#if botonCerrar}
            <button
              type="button"
              class="modal__cerrar"
              aria-label="Cerrar diálogo"
              onclick={cerrar}
            >
              <X size={16} />
            </button>
          {/if}
        </header>
      {/if}

      <div class="modal__cuerpo">{@render children()}</div>

      {#if pie}
        <footer class="modal__pie">{@render pie()}</footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    display: grid;
    place-items: center;
    padding: var(--esp-4);
    background: var(--color-overlay);
  }

  .modal {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - var(--esp-8));
    width: 100%;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-lg);
    box-shadow: var(--sombra-3);
    outline: none;
  }

  .modal--sm {
    max-width: 360px;
  }
  .modal--md {
    max-width: 520px;
  }
  .modal--lg {
    max-width: 720px;
  }
  .modal--xl {
    max-width: 960px;
  }

  .modal__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--esp-2);
    padding: var(--esp-3) var(--esp-4);
    border-bottom: 1px solid var(--color-borde);
  }

  .modal__titulo {
    margin: 0;
    font-size: var(--tam-fuente-lg);
    font-weight: 600;
  }

  .modal__cerrar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--alto-control-sm);
    height: var(--alto-control-sm);
    padding: 0;
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto-secundario);
    cursor: pointer;
  }
  .modal__cerrar:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }

  .modal__cuerpo {
    padding: var(--esp-4);
    overflow-y: auto;
  }

  .modal__pie {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--esp-2);
    padding: var(--esp-3) var(--esp-4);
    border-top: 1px solid var(--color-borde);
  }
</style>
