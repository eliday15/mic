<!--
  SplitPane — dos paneles con un divisor arrastrable. Orientación horizontal
  (paneles lado a lado) o vertical (apilados). El tamaño del primer panel se
  enlaza en `tamano` (px) y se acota a [min, max].
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  type Orientacion = "horizontal" | "vertical";

  interface Props {
    orientacion?: Orientacion;
    /** Tamaño del primer panel en píxeles (enlazado). */
    tamano?: number;
    min?: number;
    max?: number;
    primero: Snippet;
    segundo: Snippet;
  }

  let {
    orientacion = "horizontal",
    tamano = $bindable(280),
    min = 160,
    max = 640,
    primero,
    segundo,
  }: Props = $props();

  let raiz = $state<HTMLDivElement | null>(null);
  let arrastrando = $state(false);

  const esHorizontal = $derived(orientacion === "horizontal");

  function acotar(v: number): number {
    return Math.max(min, Math.min(max, v));
  }

  function onPointerDown(e: PointerEvent): void {
    arrastrando = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onPointerMove(e: PointerEvent): void {
    if (!arrastrando || !raiz) return;
    const rect = raiz.getBoundingClientRect();
    const nuevo = esHorizontal ? e.clientX - rect.left : e.clientY - rect.top;
    tamano = acotar(nuevo);
  }

  function onPointerUp(e: PointerEvent): void {
    arrastrando = false;
    (e.currentTarget as HTMLElement).releasePointerCapture?.(e.pointerId);
  }

  function onKeydown(e: KeyboardEvent): void {
    const paso = e.shiftKey ? 32 : 8;
    if (esHorizontal) {
      if (e.key === "ArrowLeft") tamano = acotar(tamano - paso);
      else if (e.key === "ArrowRight") tamano = acotar(tamano + paso);
      else return;
    } else {
      if (e.key === "ArrowUp") tamano = acotar(tamano - paso);
      else if (e.key === "ArrowDown") tamano = acotar(tamano + paso);
      else return;
    }
    e.preventDefault();
  }
</script>

<div
  bind:this={raiz}
  class="split"
  class:split--vertical={!esHorizontal}
  class:split--arrastrando={arrastrando}
>
  <div
    class="split__panel"
    style={esHorizontal
      ? `width:${tamano}px`
      : `height:${tamano}px`}
  >
    {@render primero()}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="split__divisor"
    role="separator"
    aria-orientation={esHorizontal ? "vertical" : "horizontal"}
    aria-valuenow={tamano}
    aria-valuemin={min}
    aria-valuemax={max}
    tabindex="0"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onkeydown={onKeydown}
  ></div>

  <div class="split__panel split__panel--flex">
    {@render segundo()}
  </div>
</div>

<style>
  .split {
    display: flex;
    flex-direction: row;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .split--vertical {
    flex-direction: column;
  }

  .split--arrastrando {
    cursor: col-resize;
    user-select: none;
  }
  .split--vertical.split--arrastrando {
    cursor: row-resize;
  }

  .split__panel {
    overflow: auto;
    min-width: 0;
    min-height: 0;
  }
  .split__panel--flex {
    flex: 1;
  }

  .split__divisor {
    flex: 0 0 auto;
    width: 5px;
    background: transparent;
    cursor: col-resize;
    position: relative;
    transition: background var(--transicion);
  }
  .split--vertical .split__divisor {
    width: auto;
    height: 5px;
    cursor: row-resize;
  }

  .split__divisor::before {
    content: "";
    position: absolute;
    inset: 0;
    margin: auto;
    background: var(--color-borde);
  }
  .split:not(.split--vertical) .split__divisor::before {
    width: 1px;
  }
  .split--vertical .split__divisor::before {
    height: 1px;
  }

  .split__divisor:hover::before,
  .split--arrastrando .split__divisor::before {
    background: var(--color-acento);
  }

  .split__divisor:focus-visible {
    box-shadow: var(--foco-ring);
  }
</style>
