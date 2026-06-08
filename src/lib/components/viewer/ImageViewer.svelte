<!--
  ImageViewer — visor de imagen a tamaño completo (equivale a "Ver Imagen 100%"
  del original). Overlay a pantalla completa con:
    - zoom: rueda del ratón, botones acercar/alejar, ajustar a ventana y 100 %
    - pan: arrastre con el ratón cuando la imagen excede la ventana
    - navegación: ←/→ entre los registros recibidos en `ids`
    - cierre con Escape o clic en el fondo
-->
<script lang="ts">
  import {
    X,
    ZoomIn,
    ZoomOut,
    Maximize,
    Scan,
    ChevronLeft,
    ChevronRight,
  } from "lucide-svelte";
  import { IconButton } from "$lib/components/ui";
  import { imagenOriginalUrl } from "$lib/ipc/thumbnails";
  import { t } from "$lib/i18n/es";
  import type { Tabla } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    albumId: number;
    tabla: Tabla;
    /** Ids navegables (en el orden de la grilla). */
    ids: number[];
    /** Id inicial a mostrar. */
    inicial: number;
    onCerrar?: () => void;
  }

  let {
    abierto = $bindable(true),
    albumId,
    tabla,
    ids,
    inicial,
    onCerrar,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let indice = $state(Math.max(0, ids.indexOf(inicial)));
  const id = $derived(ids[indice]);
  const url = $derived(imagenOriginalUrl(albumId, tabla, id, 0));

  // --- Zoom / pan ---------------------------------------------------------
  /** Escala actual (1 = 100 %). 0 = "ajustar" (calculada al cargar). */
  let escala = $state(0);
  let panX = $state(0);
  let panY = $state(0);
  let natW = $state(0);
  let natH = $state(0);
  let visor = $state<HTMLDivElement | null>(null);
  let cargada = $state(false);
  let fallo = $state(false);

  const ZOOM_TOPE = 8;

  function escalaAjuste(): number {
    if (!visor || natW === 0 || natH === 0) return 1;
    const r = visor.getBoundingClientRect();
    return Math.min((r.width - 32) / natW, (r.height - 32) / natH, 1);
  }

  const escalaEfectiva = $derived(escala === 0 ? escalaAjuste() : escala);

  function onCargada(e: Event): void {
    const img = e.currentTarget as HTMLImageElement;
    natW = img.naturalWidth;
    natH = img.naturalHeight;
    cargada = true;
    fallo = false;
  }

  function ajustar(): void {
    escala = 0;
    panX = 0;
    panY = 0;
  }

  function tamanoReal(): void {
    escala = 1;
    panX = 0;
    panY = 0;
  }

  function zoom(factor: number, cx = 0, cy = 0): void {
    const previa = escalaEfectiva;
    const nueva = Math.min(ZOOM_TOPE, Math.max(0.05, previa * factor));
    // Mantiene el punto bajo el cursor estable al hacer zoom.
    panX = cx - ((cx - panX) * nueva) / previa;
    panY = cy - ((cy - panY) * nueva) / previa;
    escala = nueva;
  }

  function onRueda(e: WheelEvent): void {
    e.preventDefault();
    const r = visor?.getBoundingClientRect();
    const cx = r ? e.clientX - r.left - r.width / 2 : 0;
    const cy = r ? e.clientY - r.top - r.height / 2 : 0;
    zoom(e.deltaY < 0 ? 1.15 : 1 / 1.15, cx, cy);
  }

  // Pan por arrastre.
  let arrastrando = $state(false);
  let inicioX = 0;
  let inicioY = 0;

  function onPointerDown(e: PointerEvent): void {
    arrastrando = true;
    inicioX = e.clientX - panX;
    inicioY = e.clientY - panY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: PointerEvent): void {
    if (!arrastrando) return;
    panX = e.clientX - inicioX;
    panY = e.clientY - inicioY;
  }
  function onPointerUp(): void {
    arrastrando = false;
  }

  // --- Navegación ---------------------------------------------------------
  function ir(delta: number): void {
    const n = indice + delta;
    if (n < 0 || n >= ids.length) return;
    indice = n;
    cargada = false;
    fallo = false;
    ajustar();
  }

  function onKey(e: KeyboardEvent): void {
    switch (e.key) {
      case "Escape":
        e.preventDefault();
        cerrar();
        break;
      case "ArrowLeft":
        e.preventDefault();
        ir(-1);
        break;
      case "ArrowRight":
        e.preventDefault();
        ir(1);
        break;
      case "+":
      case "=":
        zoom(1.15);
        break;
      case "-":
        zoom(1 / 1.15);
        break;
      case "0":
        ajustar();
        break;
      case "1":
        tamanoReal();
        break;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if abierto}
  <div
    class="iv"
    role="dialog"
    aria-modal="true"
    aria-label={t.ver.visor}
  >
    <!-- Barra superior -->
    <header class="iv__barra">
      <span class="iv__contador">{indice + 1} / {ids.length} · #{id}</span>
      <div class="iv__acciones">
        <IconButton etiqueta={t.visor.alejar} onclick={() => zoom(1 / 1.3)}>
          <ZoomOut size={18} />
        </IconButton>
        <span class="iv__zoom">{Math.round(escalaEfectiva * 100)} %</span>
        <IconButton etiqueta={t.visor.acercar} onclick={() => zoom(1.3)}>
          <ZoomIn size={18} />
        </IconButton>
        <IconButton etiqueta={t.visor.ajustar} onclick={ajustar}>
          <Maximize size={18} />
        </IconButton>
        <IconButton etiqueta={t.visor.tamanoReal} onclick={tamanoReal}>
          <Scan size={18} />
        </IconButton>
        <IconButton etiqueta={t.accion.cerrar} onclick={cerrar}>
          <X size={18} />
        </IconButton>
      </div>
    </header>

    <!-- Lienzo -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="iv__lienzo"
      class:iv__lienzo--arrastrando={arrastrando}
      bind:this={visor}
      role="presentation"
      onwheel={onRueda}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
    >
      {#if fallo}
        <p class="iv__sin">{t.visor.sinImagen}</p>
      {:else}
        <img
          class="iv__img"
          class:iv__img--oculta={!cargada}
          src={url}
          alt={`#${id}`}
          draggable="false"
          style={`transform: translate(calc(-50% + ${panX}px), calc(-50% + ${panY}px)) scale(${escalaEfectiva});`}
          onload={onCargada}
          onerror={() => (fallo = true)}
        />
      {/if}
    </div>

    <!-- Navegación lateral -->
    {#if indice > 0}
      <button class="iv__nav iv__nav--izq" onclick={() => ir(-1)} aria-label={t.visor.anterior}>
        <ChevronLeft size={32} />
      </button>
    {/if}
    {#if indice < ids.length - 1}
      <button class="iv__nav iv__nav--der" onclick={() => ir(1)} aria-label={t.visor.siguiente}>
        <ChevronRight size={32} />
      </button>
    {/if}
  </div>
{/if}

<style>
  .iv {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal, 100);
    display: flex;
    flex-direction: column;
    background: rgba(10, 11, 13, 0.96);
  }
  .iv__barra {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--esp-2) var(--esp-3);
    color: var(--color-texto-secundario);
    background: rgba(0, 0, 0, 0.35);
  }
  .iv__contador {
    font-size: var(--tam-fuente-sm);
    font-variant-numeric: tabular-nums;
  }
  .iv__acciones {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
  }
  .iv__zoom {
    min-width: 52px;
    text-align: center;
    font-size: var(--tam-fuente-sm);
    font-variant-numeric: tabular-nums;
  }
  .iv__lienzo {
    position: relative;
    flex: 1;
    overflow: hidden;
    cursor: grab;
    touch-action: none;
  }
  .iv__lienzo--arrastrando {
    cursor: grabbing;
  }
  .iv__img {
    position: absolute;
    top: 50%;
    left: 50%;
    transform-origin: center;
    user-select: none;
    will-change: transform;
    transition: opacity 140ms ease;
  }
  .iv__img--oculta {
    opacity: 0;
  }
  .iv__sin {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: var(--color-texto-secundario);
  }
  .iv__nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 72px;
    border: none;
    border-radius: var(--radio-md);
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity 120ms ease;
  }
  .iv__nav:hover {
    opacity: 1;
  }
  .iv__nav--izq {
    left: var(--esp-3);
  }
  .iv__nav--der {
    right: var(--esp-3);
  }
</style>
