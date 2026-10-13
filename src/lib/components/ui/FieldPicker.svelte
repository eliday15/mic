<!--
  FieldPicker — selector moderno de campos con orden: una sola lista donde cada
  campo se incluye/excluye con su casilla y los incluidos se reordenan
  arrastrando el asa (puntero, con hueco animado e indicador de destino) o con
  las flechas arriba/abajo sobre el asa. Incluye búsqueda, acciones rápidas
  "Todos / Ninguno" y el conteo de incluidos. `seleccion` es la lista ordenada
  de valores incluidos (enlazable).
-->
<script lang="ts">
  import { tick } from "svelte";
  import { GripVertical, Search } from "lucide-svelte";
  import TextInput from "./TextInput.svelte";
  import { t } from "$lib/i18n/es";

  interface OpcionCampo {
    valor: string;
    /** Texto mostrado (por defecto, el valor). */
    etiqueta?: string;
    /** Dato secundario tenue (p. ej., el tipo de campo). */
    detalle?: string;
  }

  interface Props {
    /** Todos los campos elegibles, en su orden natural. */
    opciones: OpcionCampo[];
    /** Valores incluidos, en el orden elegido. */
    seleccion?: string[];
    /** Alto fijo de la lista desplazable (CSS): el diálogo no salta al filtrar. */
    alto?: string;
    /** Etiqueta accesible del grupo. */
    etiqueta?: string;
  }

  let {
    opciones,
    seleccion = $bindable([]),
    alto = "280px",
    etiqueta,
  }: Props = $props();

  /** Alto fijo de fila: permite calcular el destino del arrastre sin medir. */
  const ALTO = 32;
  /** Margen (px) junto al borde de la lista que activa el autodesplazamiento. */
  const BORDE_SCROLL = 28;

  let busqueda = $state("");
  let lista = $state<HTMLDivElement | null>(null);

  const porValor = $derived(new Map(opciones.map((o) => [o.valor, o] as const)));

  function normalizar(s: string): string {
    return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
  }

  const filtro = $derived(normalizar(busqueda.trim()));
  const filtrando = $derived(filtro !== "");

  function coincide(valor: string): boolean {
    if (!filtrando) return true;
    const o = porValor.get(valor);
    return normalizar(o?.etiqueta ?? valor).includes(filtro);
  }

  // Incluidos que aún existen como opción (en el orden elegido).
  const incluidos = $derived(seleccion.filter((v) => porValor.has(v)));
  const incluidosVisibles = $derived(incluidos.filter(coincide));
  const otros = $derived(
    opciones.filter((o) => !seleccion.includes(o.valor) && coincide(o.valor)),
  );

  function texto(valor: string): string {
    return porValor.get(valor)?.etiqueta ?? valor;
  }

  function alternar(valor: string): void {
    seleccion = seleccion.includes(valor)
      ? seleccion.filter((v) => v !== valor)
      : [...seleccion, valor];
  }

  /** Incluye todos los campos visibles (respeta la búsqueda activa). */
  function todos(): void {
    seleccion = [...seleccion, ...otros.map((o) => o.valor)];
  }

  /** Excluye todos los campos visibles (respeta la búsqueda activa). */
  function ninguno(): void {
    const quitar = new Set(incluidosVisibles);
    seleccion = seleccion.filter((v) => !quitar.has(v));
  }

  function mover(origen: number, destino: number): void {
    if (origen === destino) return;
    const copia = [...incluidos];
    const [item] = copia.splice(origen, 1);
    copia.splice(destino, 0, item);
    seleccion = copia;
  }

  // --- Arrastre con puntero ----------------------------------------------
  interface Arrastre {
    valor: string;
    origen: number;
    destino: number;
    /** Desplazamiento vertical del elemento arrastrado (acotado a la lista). */
    dy: number;
    inicioY: number;
    inicioScroll: number;
  }

  let arrastre = $state<Arrastre | null>(null);
  // Tras soltar, se suspenden las transiciones un cuadro para que las filas no
  // "reboten" al pasar del desplazamiento visual a su posición real.
  let asentando = $state(false);
  let ultimoY = 0;
  let raf = 0;

  function recalcular(): void {
    if (!arrastre || !lista) return;
    const n = incluidos.length;
    const bruto = ultimoY - arrastre.inicioY + (lista.scrollTop - arrastre.inicioScroll);
    const dy = Math.max(-arrastre.origen * ALTO, Math.min((n - 1 - arrastre.origen) * ALTO, bruto));
    arrastre.dy = dy;
    arrastre.destino = arrastre.origen + Math.round(dy / ALTO);
  }

  /** Autodesplaza la lista cuando el puntero se acerca a sus bordes. */
  function bucleScroll(): void {
    if (!arrastre || !lista) return;
    const r = lista.getBoundingClientRect();
    if (ultimoY < r.top + BORDE_SCROLL) lista.scrollTop -= 8;
    else if (ultimoY > r.bottom - BORDE_SCROLL) lista.scrollTop += 8;
    recalcular();
    raf = requestAnimationFrame(bucleScroll);
  }

  function iniciarArrastre(e: PointerEvent, valor: string, indice: number): void {
    if (filtrando || e.button !== 0 || !lista) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    ultimoY = e.clientY;
    arrastre = {
      valor,
      origen: indice,
      destino: indice,
      dy: 0,
      inicioY: e.clientY,
      inicioScroll: lista.scrollTop,
    };
    raf = requestAnimationFrame(bucleScroll);
  }

  function moverPuntero(e: PointerEvent): void {
    if (!arrastre) return;
    ultimoY = e.clientY;
    recalcular();
  }

  function terminarArrastre(confirmar: boolean): void {
    if (!arrastre) return;
    cancelAnimationFrame(raf);
    const { origen, destino } = arrastre;
    asentando = true;
    arrastre = null;
    if (confirmar) mover(origen, destino);
    requestAnimationFrame(() => (asentando = false));
  }

  /** Desplazamiento visual de la fila `j` durante el arrastre. */
  function desplazamiento(j: number): number {
    if (!arrastre) return 0;
    const { origen: o, destino: d } = arrastre;
    if (j === o) return arrastre.dy;
    if (o < d && j > o && j <= d) return -ALTO;
    if (d < o && j >= d && j < o) return ALTO;
    return 0;
  }

  // --- Teclado (alternativa accesible al arrastre) ------------------------
  async function teclaAsa(e: KeyboardEvent, valor: string, indice: number): Promise<void> {
    if (filtrando) return;
    let destino = indice;
    if (e.key === "ArrowUp") destino = indice - 1;
    else if (e.key === "ArrowDown") destino = indice + 1;
    else if (e.key === "Home") destino = 0;
    else if (e.key === "End") destino = incluidos.length - 1;
    else return;
    e.preventDefault();
    destino = Math.max(0, Math.min(incluidos.length - 1, destino));
    if (destino === indice) return;
    mover(indice, destino);
    // El nodo cambia de lugar en el DOM y pierde el foco: se recupera.
    await tick();
    lista
      ?.querySelector<HTMLElement>(`[data-asa="${CSS.escape(valor)}"]`)
      ?.focus();
  }
</script>

<div class="fp" role="group" aria-label={etiqueta}>
  <div class="fp__barra">
    <div class="fp__buscar">
      <TextInput bind:value={busqueda} placeholder={t.selectorCampos.buscar} type="search">
        {#snippet prefijo()}<Search size={14} />{/snippet}
      </TextInput>
    </div>
    <span class="fp__conteo">{t.selectorCampos.conteo(incluidos.length, opciones.length)}</span>
    <button type="button" class="fp__rapida" onclick={todos} disabled={otros.length === 0}>
      {t.selectorCampos.todos}
    </button>
    <button
      type="button"
      class="fp__rapida"
      onclick={ninguno}
      disabled={incluidosVisibles.length === 0}
    >
      {t.selectorCampos.ninguno}
    </button>
  </div>

  <div class="fp__lista" bind:this={lista} style:height={alto}>
    {#if incluidosVisibles.length === 0 && otros.length === 0}
      <p class="fp__vacio">{t.selectorCampos.sinResultados}</p>
    {:else}
      {#if !filtrando || incluidosVisibles.length > 0}
        <div class="fp__seccion">
          <span>{t.selectorCampos.incluidos}</span>
          {#if filtrando}
            <span class="fp__pista">{t.selectorCampos.filtrando}</span>
          {/if}
        </div>
      {/if}
      {#if incluidosVisibles.length === 0}
        {#if !filtrando}
          <p class="fp__vacio fp__vacio--sm">{t.selectorCampos.vacio}</p>
        {/if}
      {:else}
        <ul
          class="fp__orden"
          class:fp__orden--arrastrando={arrastre !== null}
          class:fp__orden--asentando={asentando}
          style:height={`${incluidosVisibles.length * ALTO}px`}
        >
          {#if arrastre}
            <li
              class="fp__destino"
              aria-hidden="true"
              style:transform={`translateY(${arrastre.destino * ALTO}px)`}
            ></li>
          {/if}
          {#each incluidosVisibles as valor, i (valor)}
            {@const o = porValor.get(valor)}
            <li
              class="fp__fila fp__fila--incluida"
              class:fp__fila--arrastrada={arrastre?.valor === valor}
              style:transform={`translateY(${desplazamiento(i)}px)`}
            >
              <button
                type="button"
                class="fp__asa"
                data-asa={valor}
                aria-label={t.selectorCampos.arrastrar(texto(valor))}
                title={filtrando ? t.selectorCampos.filtrando : t.selectorCampos.arrastrar(texto(valor))}
                disabled={filtrando}
                onpointerdown={(e) => iniciarArrastre(e, valor, i)}
                onpointermove={moverPuntero}
                onpointerup={() => terminarArrastre(true)}
                onpointercancel={() => terminarArrastre(false)}
                onkeydown={(e) => teclaAsa(e, valor, i)}
              >
                <GripVertical size={14} />
              </button>
              <label class="fp__check">
                <input type="checkbox" checked onchange={() => alternar(valor)} />
                {#if !filtrando}<span class="fp__pos">{i + 1}</span>{/if}
                <span class="fp__nombre">{texto(valor)}</span>
                {#if o?.detalle}<span class="fp__detalle">{o.detalle}</span>{/if}
              </label>
            </li>
          {/each}
        </ul>
      {/if}

      {#if otros.length > 0}
        <div class="fp__seccion">{t.selectorCampos.otros}</div>
        <ul class="fp__otros">
          {#each otros as o (o.valor)}
            <li class="fp__fila">
              <span class="fp__asa fp__asa--vacia" aria-hidden="true"></span>
              <label class="fp__check">
                <input type="checkbox" onchange={() => alternar(o.valor)} />
                <span class="fp__nombre">{o.etiqueta ?? o.valor}</span>
                {#if o.detalle}<span class="fp__detalle">{o.detalle}</span>{/if}
              </label>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
</div>

<style>
  .fp {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    min-width: 0;
  }
  .fp__barra {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
  }
  .fp__buscar {
    flex: 1;
    min-width: 0;
  }
  .fp__conteo {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .fp__rapida {
    padding: 2px var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-pill);
    background: var(--color-superficie);
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-xs);
    cursor: pointer;
    transition:
      background var(--transicion),
      color var(--transicion);
  }
  .fp__rapida:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-texto);
  }
  .fp__rapida:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .fp__lista {
    overflow-y: auto;
    padding: var(--esp-1);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
  }
  .fp__seccion {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--esp-2);
    padding: var(--esp-1) var(--esp-2);
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-tenue);
  }
  .fp__pista {
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
  }
  .fp__vacio {
    margin: 0;
    padding: var(--esp-4) var(--esp-2);
    text-align: center;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
  .fp__vacio--sm {
    padding: var(--esp-2);
  }

  .fp__orden,
  .fp__otros {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .fp__orden {
    position: relative;
  }
  .fp__fila {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    height: 32px;
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
  }
  .fp__fila:hover {
    background: var(--color-hover);
  }
  .fp__fila--incluida {
    position: relative;
    z-index: 1;
    transition: transform var(--transicion);
  }
  .fp__orden--asentando .fp__fila--incluida {
    transition: none;
  }
  .fp__orden--arrastrando .fp__fila:hover {
    background: var(--color-superficie);
  }
  .fp__fila--arrastrada,
  .fp__orden--arrastrando .fp__fila--arrastrada:hover {
    z-index: 3;
    transition: none;
    background: var(--color-elevado);
    box-shadow: var(--sombra-2);
    outline: 1px solid var(--color-acento);
  }
  .fp__destino {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 0;
    height: 32px;
    box-sizing: border-box;
    border: 1.5px dashed var(--color-acento);
    border-radius: var(--radio-sm);
    background: var(--color-acento-tenue);
    transition: transform var(--transicion);
  }

  .fp__asa {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 24px;
    height: 100%;
    padding: 0;
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto-tenue);
    cursor: grab;
    touch-action: none;
  }
  .fp__asa:hover:not(:disabled),
  .fp__asa:focus-visible {
    color: var(--color-acento);
  }
  .fp__asa:focus-visible {
    outline: 2px solid var(--color-acento);
    outline-offset: -2px;
  }
  .fp__asa:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .fp__orden--arrastrando .fp__asa {
    cursor: grabbing;
  }
  .fp__asa--vacia {
    cursor: default;
  }

  .fp__check {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    flex: 1;
    min-width: 0;
    height: 100%;
    padding-right: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto);
    cursor: pointer;
    user-select: none;
  }
  .fp__check input {
    flex: none;
    margin: 0;
    accent-color: var(--color-acento);
  }
  .fp__pos {
    flex: none;
    min-width: 18px;
    padding: 0 4px;
    border-radius: var(--radio-pill);
    background: var(--color-acento-tenue);
    color: var(--color-acento);
    font-size: var(--tam-fuente-xs);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .fp__nombre {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fp__detalle {
    flex: none;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
</style>
