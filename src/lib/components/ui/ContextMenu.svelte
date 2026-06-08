<!--
  ContextMenu — menú contextual posicionado en coordenadas de pantalla. Se abre
  con `abrir(x, y)` (expuesto) y se cierra al elegir una opción, pulsar Escape o
  hacer clic fuera. Las opciones son una lista declarativa con separadores.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export interface OpcionMenu {
    id: string;
    etiqueta?: string;
    /** Si true, se renderiza un separador (ignora etiqueta/acción). */
    separador?: boolean;
    /** Icono opcional como snippet. */
    icono?: Snippet;
    /** Estilo de peligro (rojo). */
    peligro?: boolean;
    deshabilitada?: boolean;
  }

  interface Props {
    opciones: OpcionMenu[];
    onElegir: (id: string) => void;
  }

  let { opciones, onElegir }: Props = $props();

  let abierto = $state(false);
  let x = $state(0);
  let y = $state(0);
  let menu = $state<HTMLDivElement | null>(null);

  /** Abre el menú en las coordenadas indicadas (de cliente). */
  export function abrir(px: number, py: number): void {
    x = px;
    y = py;
    abierto = true;
    // Reajusta dentro del viewport tras renderizar.
    queueMicrotask(() => {
      if (!menu) return;
      const r = menu.getBoundingClientRect();
      if (x + r.width > window.innerWidth) x = window.innerWidth - r.width - 4;
      if (y + r.height > window.innerHeight)
        y = window.innerHeight - r.height - 4;
    });
  }

  /** Cierra el menú. */
  export function cerrar(): void {
    abierto = false;
  }

  function elegir(op: OpcionMenu): void {
    if (op.deshabilitada || op.separador) return;
    abierto = false;
    onElegir(op.id);
  }

  $effect(() => {
    if (!abierto) return;
    function onDoc(e: MouseEvent): void {
      if (menu && !menu.contains(e.target as Node)) abierto = false;
    }
    function onKey(e: KeyboardEvent): void {
      if (e.key === "Escape") abierto = false;
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    window.addEventListener("blur", cerrar);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", cerrar);
    };
  });
</script>

{#if abierto}
  <div
    bind:this={menu}
    class="ctx"
    role="menu"
    style="left:{x}px; top:{y}px;"
  >
    {#each opciones as op (op.id)}
      {#if op.separador}
        <div class="ctx__sep" role="separator"></div>
      {:else}
        <button
          type="button"
          class="ctx__op"
          class:ctx__op--peligro={op.peligro}
          role="menuitem"
          disabled={op.deshabilitada}
          onclick={() => elegir(op)}
        >
          {#if op.icono}
            <span class="ctx__icono">{@render op.icono()}</span>
          {/if}
          <span class="ctx__txt">{op.etiqueta}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ctx {
    position: fixed;
    z-index: var(--z-menu);
    min-width: 180px;
    padding: var(--esp-1);
    background: var(--color-elevado);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    box-shadow: var(--sombra-2);
    animation: aparecer var(--transicion-rapida) ease-out;
  }

  .ctx__op {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    width: 100%;
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto);
    text-align: left;
    cursor: pointer;
  }
  .ctx__op:hover:not(:disabled) {
    background: var(--color-hover);
  }
  .ctx__op:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .ctx__op--peligro {
    color: var(--color-peligro);
  }

  .ctx__icono {
    display: inline-flex;
    color: var(--color-texto-secundario);
  }
  .ctx__op--peligro .ctx__icono {
    color: var(--color-peligro);
  }

  .ctx__txt {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ctx__sep {
    height: 1px;
    margin: var(--esp-1) var(--esp-1);
    background: var(--color-borde);
  }

  @keyframes aparecer {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
