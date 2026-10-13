<!--
  ActiveFiltersList — lista de los filtros libres activos (condiciones del
  álbum), apilados en el orden en que se evalúan. Cada fila se puede quitar,
  alternar su conector Y/O con la anterior y reordenar arrastrando el asa
  (pointer events, para no depender del drag & drop HTML5 que el manejador de
  archivos de Tauri intercepta en Windows) o con las flechas ↑ ↓ sobre el asa.
-->
<script lang="ts">
  import { GripVertical, X } from "lucide-svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
  }

  let { estado }: Props = $props();

  let lista = $state<HTMLOListElement | null>(null);
  /** Índice de la fila arrastrada (null = sin arrastre). */
  let arrastrada = $state<number | null>(null);
  /** Índice final que tendría la fila arrastrada si se soltara ahora. */
  let destino = $state<number | null>(null);
  let desplazamiento = $state(0);
  /** Alto de fila + separación: lo que se corren las demás para hacer hueco. */
  let paso = $state(0);
  let yInicial = 0;
  /** Centros verticales de las filas al empezar el arrastre (sin transformar). */
  let centros: number[] = [];

  function etiquetaOp(op: string): string {
    return (t.filtros.op as Record<string, string>)[op] ?? op;
  }

  /** Índice final: cuántas de las otras filas quedan por encima del puntero. */
  function destinoEn(y: number): number {
    return centros.filter((c, i) => i !== arrastrada && c < y).length;
  }

  /** Corrimiento de la fila `i` mientras otra se arrastra (hueco animado). */
  function corrimiento(i: number): number {
    if (arrastrada === null || destino === null || i === arrastrada) return 0;
    if (arrastrada < i && i <= destino) return -paso;
    if (destino <= i && i < arrastrada) return paso;
    return 0;
  }

  function onPointerDown(e: PointerEvent, i: number): void {
    if (e.button !== 0) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const filas = lista ? [...lista.querySelectorAll<HTMLElement>(".afl__fila")] : [];
    const rects = filas.map((f) => f.getBoundingClientRect());
    centros = rects.map((r) => r.top + r.height / 2);
    paso = rects.length > 1 ? rects[1].top - rects[0].top : 0;
    arrastrada = i;
    destino = i;
    yInicial = e.clientY;
    desplazamiento = 0;
  }

  function onPointerMove(e: PointerEvent): void {
    if (arrastrada === null) return;
    desplazamiento = e.clientY - yInicial;
    destino = destinoEn(e.clientY);
  }

  function onPointerUp(): void {
    if (arrastrada !== null && destino !== null) {
      estado.moverCondicion(arrastrada, destino);
    }
    arrastrada = null;
    destino = null;
    desplazamiento = 0;
  }

  function onKeyAsa(e: KeyboardEvent, i: number): void {
    // Con el foco en el asa, ↑/↓ (con o sin Alt) mueven la fila.
    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
    e.preventDefault();
    const hasta = e.key === "ArrowUp" ? i - 1 : i + 1;
    if (hasta < 0 || hasta >= estado.condiciones.length) return;
    estado.moverCondicion(i, hasta);
    // Mantener el foco en el asa de la fila movida.
    requestAnimationFrame(() =>
      lista?.querySelectorAll<HTMLElement>(".afl__asa")[hasta]?.focus(),
    );
  }

</script>

<ol class="afl" bind:this={lista} aria-label={t.filtros.libres}>
  {#each estado.condiciones as cond, i (i)}
    <li
      class="afl__fila"
      class:afl__fila--arrastre={arrastrada === i}
      style:transform={arrastrada === i
        ? `translateY(${desplazamiento}px)`
        : corrimiento(i) !== 0
          ? `translateY(${corrimiento(i)}px)`
          : null}
      data-testid="filtro-activo"
    >
      <button
        type="button"
        class="afl__asa"
        aria-label={t.filtros.arrastrar}
        title={t.filtros.arrastrar}
        onpointerdown={(e) => onPointerDown(e, i)}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        onkeydown={(e) => onKeyAsa(e, i)}
      >
        <GripVertical size={14} />
      </button>
      {#if i > 0}
        <button
          type="button"
          class="afl__rel"
          class:afl__rel--o={cond.opRel === "o"}
          title={t.filtros.alternarConector}
          onclick={() => estado.setConector(i, cond.opRel === "o" ? "y" : "o")}
        >
          {cond.opRel === "o" ? t.filtros.rel.o : t.filtros.rel.y}
        </button>
      {/if}
      <span class="afl__txt" title="{cond.campo} {etiquetaOp(cond.opComp)} {cond.valor}">
        <strong>{cond.campo}</strong>
        <span class="afl__op">{etiquetaOp(cond.opComp)}</span>
        {cond.valor}
      </span>
      <button
        type="button"
        class="afl__x"
        aria-label={t.filtros.quitarCondicion}
        title={t.filtros.quitarCondicion}
        onclick={() => estado.quitarCondicion(i)}
      >
        <X size={12} />
      </button>
    </li>
  {/each}
</ol>

<style>
  .afl {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .afl__fila {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    min-height: var(--alto-control-sm);
    padding: 2px var(--esp-1) 2px 0;
    border-radius: var(--radio-sm);
    background: var(--color-acento-tenue);
    color: var(--color-texto);
    font-size: var(--tam-fuente-sm);
    transition:
      background var(--transicion-rapida),
      transform var(--transicion-rapida);
  }
  .afl__fila--arrastre {
    z-index: 1;
    background: var(--color-elevado);
    box-shadow: var(--sombra-2);
    transition: none;
  }
  .afl__asa {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    align-self: stretch;
    padding: 0;
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto-tenue);
    cursor: grab;
    touch-action: none;
  }
  .afl__fila--arrastre .afl__asa {
    cursor: grabbing;
  }
  .afl__asa:hover,
  .afl__asa:focus-visible {
    color: var(--color-texto);
  }
  .afl__rel {
    flex: none;
    min-width: 22px;
    height: 18px;
    padding: 0 var(--esp-1);
    border: 1px solid var(--color-acento);
    border-radius: var(--radio-pill);
    background: transparent;
    color: var(--color-acento);
    font-size: var(--tam-fuente-xs);
    font-weight: 700;
    cursor: pointer;
  }
  .afl__rel--o {
    background: var(--color-acento);
    color: var(--color-texto-sobre-acento);
  }
  .afl__txt {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .afl__op {
    color: var(--color-texto-secundario);
  }
  .afl__x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: var(--radio-pill);
    background: transparent;
    color: var(--color-texto-secundario);
    cursor: pointer;
  }
  .afl__x:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }
</style>
