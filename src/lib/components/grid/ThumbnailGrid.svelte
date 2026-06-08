<!--
  ThumbnailGrid — grilla virtual de miniaturas con virtualizador propio.

  Celdas uniformes. El número de columnas se deriva del ancho disponible y del
  zoom (tamaño de celda). Las filas se virtualizan: solo se montan las visibles
  más un overscan, posicionadas con `translateY` dentro de un contenedor de la
  altura total. Las ventanas de datos se piden alineadas a páginas vía la caché
  `CacheVentanas`. Soporta selección (click / ⌘ / shift), doble click para abrir
  el editor, menú contextual y navegación por teclado (flechas, Espacio, Enter).
-->
<script lang="ts">
  import { ImageOff, EyeOff, ImagePlus } from "lucide-svelte";
  import { ContextMenu, EmptyState } from "$lib/components/ui";
  import type { OpcionMenu } from "$lib/components/ui";
  import type { CacheVentanas } from "$lib/utils/ventanas";
  import { thumbUrl } from "$lib/ipc/thumbnails";
  import { registroEditar } from "$lib/ipc/commands";
  import { formatearPorTipo } from "$lib/utils/format";
  import { validarValor } from "$lib/domain/validation";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type {
    CampoDef,
    RegistroLigero,
    TamanoThumb,
    Valor,
  } from "$lib/domain/types";

  interface Props {
    estado: AlbumState;
    /** Caché de ventanas compartida (la crea AlbumView). */
    cache: CacheVentanas;
    /** Contador de recarga: cambia cuando la caché obtiene páginas nuevas. */
    tick: number;
    /** Doble click sobre una celda → abrir editor. */
    onAbrir?: (id: number) => void;
    /** Acción de menú contextual sobre la selección. */
    onAccion?: (accion: string, id: number) => void;
    /** Notifica un cambio de datos para re-render (edición en card). */
    onRefrescar?: () => void;
  }

  let { estado, cache, tick, onAbrir, onAccion, onRefrescar }: Props = $props();

  const GAP = 12;
  const OVERSCAN = 2;

  let contenedor = $state<HTMLDivElement | null>(null);
  let anchoViewport = $state(0);
  let altoViewport = $state(0);
  let scrollTop = $state(0);
  let focoIndice = $state(0);
  let menu = $state<ContextMenu | null>(null);
  let menuId = $state<number | null>(null);
  // Ancla para selección por rango con Shift.
  let ancla = 0;

  // Tamaño de celda: con columnas fijas (ex 8/4/2 por línea) se calcula del
  // ancho disponible; en auto lo manda el zoom.
  const cellSize = $derived(
    estado.columnasFijas !== null && anchoViewport > 0
      ? Math.max(
          60,
          Math.floor((anchoViewport + GAP) / estado.columnasFijas) - GAP,
        )
      : estado.zoom,
  );
  // Líneas de datos bajo cada miniatura: TODOS los campos a la vista, sin
  // tope (como los datos bajo cada imagen del original). La celda crece según
  // cuántos campos estén visibles; se controla en Ver → Campos a la Vista.
  const lineasVista = $derived(Math.max(1, estado.camposVisibles.length));

  // La letra escala con el zoom igual que la foto (a zoom por defecto 180 →
  // 12/11 px, como antes), acotada para seguir legible en los extremos.
  // Con `letraEscala` apagado (toggle junto al zoom) queda fija en 12/11.
  const fuenteEtq = $derived(
    estado.letraEscala
      ? Math.round(Math.min(22, Math.max(10, cellSize * (12 / 180))))
      : 12,
  );
  const fuenteEtq2 = $derived(
    estado.letraEscala
      ? Math.round(Math.min(20, Math.max(9, cellSize * (11 / 180))))
      : 11,
  );
  const altoLinea1 = $derived(fuenteEtq + 6);
  const altoLinea2 = $derived(fuenteEtq2 + 5);

  // Alto exacto de la zona de etiquetas (sin gap interno): 8 px de respiro +
  // línea principal + líneas extra.
  const ETIQUETA = $derived(8 + altoLinea1 + (lineasVista - 1) * altoLinea2);
  const altoFila = $derived(cellSize + ETIQUETA + GAP);
  const columnas = $derived(
    estado.columnasFijas ??
      Math.max(1, Math.floor((anchoViewport + GAP) / (cellSize + GAP))),
  );
  const total = $derived(estado.total);
  const filas = $derived(Math.ceil(total / columnas));
  const altoTotal = $derived(filas * altoFila);

  const filaPrimera = $derived(
    Math.max(0, Math.floor(scrollTop / altoFila) - OVERSCAN),
  );
  const filasVisibles = $derived(
    Math.ceil(altoViewport / altoFila) + OVERSCAN * 2,
  );
  const filaUltima = $derived(Math.min(filas, filaPrimera + filasVisibles));

  // Tamaño de miniatura más adecuado al zoom.
  const tamThumb = $derived<TamanoThumb>(
    cellSize <= 128 ? 128 : cellSize <= 256 ? 256 : 512,
  );

  /** Primer campo a la vista (la etiqueta principal de la card). */
  const campoPrimario = $derived(estado.camposVisibles[0] ?? null);

  // Pide al backend las páginas que cubren el rango visible.
  $effect(() => {
    void tick;
    const desde = filaPrimera * columnas;
    const hasta = filaUltima * columnas;
    cache.asegurarRango(desde, hasta);
  });

  // Publica el rango realmente visible (sin overscan) para la barra de estado
  // ("1–24 / 234", como el panel de rango del original).
  $effect(() => {
    if (total === 0) {
      estado.setRangoVisible(null);
      return;
    }
    const primeraReal = Math.floor(scrollTop / altoFila);
    const ultimaReal = Math.ceil((scrollTop + altoViewport) / altoFila);
    const desde = Math.min(total, primeraReal * columnas + 1);
    const hasta = Math.min(total, ultimaReal * columnas);
    estado.setRangoVisible([desde, hasta]);
  });

  // Reinicia scroll al cambiar la consulta (la caché se invalida sola por
  // versionConsulta al pedir páginas).
  $effect(() => {
    void estado.versionConsulta;
    if (contenedor) contenedor.scrollTop = 0;
    scrollTop = 0;
    focoIndice = 0;
  });

  function onScroll(): void {
    if (contenedor) scrollTop = contenedor.scrollTop;
  }

  function registroEn(indice: number): RegistroLigero | undefined {
    void tick;
    return cache.obtener(indice);
  }

  function indiceCelda(fila: number, col: number): number {
    return fila * columnas + col;
  }

  // --- Selección ---------------------------------------------------------
  function clickCelda(e: MouseEvent, indice: number, reg: RegistroLigero): void {
    focoIndice = indice;
    if (e.shiftKey) {
      seleccionarRango(ancla, indice);
    } else if (e.metaKey || e.ctrlKey) {
      estado.alternarSeleccion(reg.id);
      ancla = indice;
    } else {
      estado.seleccionarUno(reg.id);
      ancla = indice;
    }
  }

  function seleccionarRango(a: number, b: number): void {
    const desde = Math.min(a, b);
    const hasta = Math.max(a, b);
    const ids: number[] = [];
    for (let i = desde; i <= hasta; i++) {
      const r = cache.obtener(i);
      if (r) ids.push(r.id);
    }
    estado.seleccionarVarios(ids);
  }

  // --- Menú contextual ---------------------------------------------------
  // "Ocultar" cambia a "Mostrar" cuando el registro bajo el cursor está oculto.
  let menuReg = $state<RegistroLigero | null>(null);
  const opcionesMenu = $derived<OpcionMenu[]>([
    { id: "editar", etiqueta: t.editar.editarRegistro },
    { id: "ver100", etiqueta: t.ver.visor },
    { id: "variantes", etiqueta: t.registro.variantes },
    { id: "sep", separador: true },
    menuReg?.oculto
      ? { id: "mostrar", etiqueta: t.editar.mostrar }
      : { id: "ocultar", etiqueta: t.editar.ocultar },
    { id: "eliminar", etiqueta: t.accion.eliminar, peligro: true },
  ]);

  function abrirMenu(e: MouseEvent, indice: number, reg: RegistroLigero): void {
    e.preventDefault();
    focoIndice = indice;
    if (!estado.estaSeleccionado(reg.id)) estado.seleccionarUno(reg.id);
    menuId = reg.id;
    menuReg = reg;
    menu?.abrir(e.clientX, e.clientY);
  }

  function elegirMenu(accion: string): void {
    if (menuId !== null) onAccion?.(accion, menuId);
  }

  // --- Teclado -----------------------------------------------------------
  function onKeydown(e: KeyboardEvent): void {
    let nuevo = focoIndice;
    switch (e.key) {
      case "ArrowRight":
        nuevo = Math.min(total - 1, focoIndice + 1);
        break;
      case "ArrowLeft":
        nuevo = Math.max(0, focoIndice - 1);
        break;
      case "ArrowDown":
        nuevo = Math.min(total - 1, focoIndice + columnas);
        break;
      case "ArrowUp":
        nuevo = Math.max(0, focoIndice - columnas);
        break;
      case " ": {
        e.preventDefault();
        const r = cache.obtener(focoIndice);
        if (r) estado.alternarSeleccion(r.id);
        return;
      }
      case "Enter": {
        const r = cache.obtener(focoIndice);
        if (r) onAbrir?.(r.id);
        return;
      }
      default:
        return;
    }
    e.preventDefault();
    focoIndice = nuevo;
    desplazarAFoco();
    const r = cache.obtener(nuevo);
    if (r && !e.shiftKey) {
      estado.seleccionarUno(r.id);
      ancla = nuevo;
    } else if (r && e.shiftKey) {
      seleccionarRango(ancla, nuevo);
    }
  }

  function desplazarAFoco(): void {
    if (!contenedor) return;
    const fila = Math.floor(focoIndice / columnas);
    const arriba = fila * altoFila;
    const abajo = arriba + altoFila;
    if (arriba < contenedor.scrollTop) {
      contenedor.scrollTop = arriba;
    } else if (abajo > contenedor.scrollTop + altoViewport) {
      contenedor.scrollTop = abajo - altoViewport;
    }
  }

  // Observador de tamaño del viewport.
  $effect(() => {
    if (!contenedor) return;
    const ro = new ResizeObserver((ents) => {
      const r = ents[0].contentRect;
      anchoViewport = r.width;
      altoViewport = r.height;
    });
    ro.observe(contenedor);
    return () => ro.disconnect();
  });

  // Etiqueta de la celda: primer campo visible formateado, o el id si no hay.
  function etiquetaDe(reg: RegistroLigero): string {
    const vis = estado.camposVisibles;
    if (vis.length === 0) return `#${reg.id}`;
    const campo = vis[0];
    const v = reg.valores[campo.nombre] ?? null;
    const texto = formatearPorTipo(v, campo.tipo, campo.decimales, campo.formato);
    return texto !== "" ? texto : `#${reg.id}`;
  }

  // Resto de campos a la vista (líneas 2..N). El texto se formatea INLINE en
  // el template (no aquí): así la línea editada se refresca al cerrar su input
  // aunque `reg` sea el mismo objeto mutado (igual que hace TableView).
  const camposExtras = $derived(estado.camposVisibles.slice(1));

  // --- Edición directa en la card (ex txtv del frmDocument) ---------------
  let editClave = $state<string | null>(null);
  let textoEdicion = $state("");

  function editableEnCard(campo: CampoDef): boolean {
    return (
      campo.modificable &&
      campo.tipo !== "calculado" &&
      campo.tipo !== "multidato"
    );
  }

  function claveDe(reg: RegistroLigero, campo: CampoDef): string {
    return `${reg.id}:${campo.id}`;
  }

  function iniciarEdicionCard(
    e: MouseEvent,
    reg: RegistroLigero,
    campo: CampoDef,
  ): void {
    if (!editableEnCard(campo)) return; // deja que el click seleccione la card
    e.stopPropagation();
    editClave = claveDe(reg, campo);
    const v = reg.valores[campo.nombre] ?? null;
    textoEdicion = v === null ? "" : String(v);
  }

  async function confirmarEdicionCard(
    campo: CampoDef,
    reg: RegistroLigero,
  ): Promise<void> {
    if (editClave === null) return;
    editClave = null;
    const bruto: Valor = textoEdicion === "" ? null : textoEdicion;
    const res = validarValor(campo, bruto);
    if (!res.ok) {
      ui.error(res.error ?? t.error.valorInvalido);
      return;
    }
    const nuevo = res.valor ?? null;
    if ((reg.valores[campo.nombre] ?? null) === nuevo) return;
    try {
      const actualizado = await registroEditar(
        estado.albumId,
        reg.id,
        estado.tabla,
        { [campo.nombre]: nuevo },
      );
      // Recarga las ventanas de datos (incluye calculados recalculados). Se
      // refetchea en vez de mutar: la caché no es reactiva y las mutaciones
      // no invalidan los derivados memoizados. El scroll se conserva.
      void actualizado;
      cache.reiniciar();
      onRefrescar?.();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
    }
  }

  function onKeyEdicionCard(
    e: KeyboardEvent,
    campo: CampoDef,
    reg: RegistroLigero,
  ): void {
    if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      void confirmarEdicionCard(campo, reg);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      editClave = null;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  bind:this={contenedor}
  class="grid"
  style="--cell-size:{cellSize}px; --fz1:{fuenteEtq}px; --lh1:{altoLinea1}px; --fz2:{fuenteEtq2}px; --lh2:{altoLinea2}px"
  tabindex="0"
  role="grid"
  aria-label="Cuadrícula de registros"
  aria-rowcount={filas}
  onscroll={onScroll}
  onkeydown={onKeydown}
>
  {#if total === 0}
    <div class="grid__vacio">
      <EmptyState titulo={t.vacio.albumTitulo} descripcion={t.vacio.albumDesc}>
        {#snippet icono()}<ImagePlus size={36} />{/snippet}
      </EmptyState>
    </div>
  {/if}
  <div class="grid__lienzo" style="height:{altoTotal}px">
    {#each Array.from({ length: Math.max(0, filaUltima - filaPrimera) }) as _, fi (filaPrimera + fi)}
      {@const fila = filaPrimera + fi}
      <div
        class="grid__fila"
        style="transform:translateY({fila * altoFila}px); grid-template-columns:repeat({columnas}, var(--cell-size));"
        role="row"
      >
        {#each Array.from({ length: columnas }) as _c, col (col)}
          {@const indice = indiceCelda(fila, col)}
          {#if indice < total}
            {@const reg = registroEn(indice)}
            <div
              class="cel"
              class:cel--sel={reg && estado.estaSeleccionado(reg.id)}
              class:cel--foco={indice === focoIndice}
              class:cel--oculta={reg?.oculto}
              role="gridcell"
              aria-selected={reg ? estado.estaSeleccionado(reg.id) : false}
            >
              {#if reg}
                <button
                  type="button"
                  class="cel__btn"
                  onclick={(e) => clickCelda(e, indice, reg)}
                  ondblclick={() => onAbrir?.(reg.id)}
                  oncontextmenu={(e) => abrirMenu(e, indice, reg)}
                >
                  <div class="cel__img" style="height:{cellSize}px">
                    {#if reg.imagen}
                      <img
                        class="cel__pic"
                        src={thumbUrl(estado.albumId, estado.tabla, reg.id, tamThumb, reg.imagenVersion ?? 0)}
                        alt=""
                        loading="lazy"
                        onerror={(e) => (e.currentTarget as HTMLImageElement).classList.add("cel__pic--err")}
                      />
                    {:else}
                      <span class="cel__sinimg"><ImageOff size={20} /></span>
                    {/if}
                    {#if reg.tieneVariantes}
                      <span class="cel__badge" title={t.registro.variantes}></span>
                    {/if}
                    {#if reg.oculto}
                      <span class="cel__oculto" title={t.editar.ocultar}>
                        <EyeOff size={14} />
                      </span>
                    {/if}
                  </div>
                </button>

                <!-- Datos a la vista: clic en una línea modificable la edita
                     ahí mismo (ex txtv del frmDocument). -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class="cel__datos"
                  onclick={(e) => clickCelda(e, indice, reg)}
                  ondblclick={() => onAbrir?.(reg.id)}
                  oncontextmenu={(e) => abrirMenu(e, indice, reg)}
                >
                  {#if campoPrimario && editClave === claveDe(reg, campoPrimario)}
                    <!-- svelte-ignore a11y_autofocus -->
                    <input
                      class="cel__input"
                      bind:value={textoEdicion}
                      autofocus
                      onclick={(e) => e.stopPropagation()}
                      onkeydown={(e) => onKeyEdicionCard(e, campoPrimario, reg)}
                      onblur={() => confirmarEdicionCard(campoPrimario, reg)}
                    />
                  {:else}
                    <span
                      class="cel__etq"
                      class:cel__editable={campoPrimario !== null &&
                        editableEnCard(campoPrimario)}
                      onclick={(e) =>
                        campoPrimario && iniciarEdicionCard(e, reg, campoPrimario)}
                    >
                      {etiquetaDe(reg)}
                    </span>
                  {/if}
                  {#each camposExtras as campoX (campoX.id)}
                    {#if editClave === claveDe(reg, campoX)}
                      <!-- svelte-ignore a11y_autofocus -->
                      <input
                        class="cel__input"
                        bind:value={textoEdicion}
                        autofocus
                        onclick={(e) => e.stopPropagation()}
                        onkeydown={(e) => onKeyEdicionCard(e, campoX, reg)}
                        onblur={() => confirmarEdicionCard(campoX, reg)}
                      />
                    {:else}
                      <span
                        class="cel__etq2"
                        class:cel__editable={editableEnCard(campoX)}
                        onclick={(e) => iniciarEdicionCard(e, reg, campoX)}
                      >
                        <span class="cel__etqnom">{campoX.nombre}:</span>
                        {formatearPorTipo(
                          reg.valores[campoX.nombre] ?? null,
                          campoX.tipo,
                          campoX.decimales,
                          campoX.formato,
                          )}
                      </span>
                    {/if}
                  {/each}
                </div>
              {:else}
                <div class="cel__skeleton" style="height:{cellSize}px"></div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    {/each}
  </div>
</div>

<ContextMenu bind:this={menu} opciones={opcionesMenu} onElegir={elegirMenu} />

<style>
  .grid {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    outline: none;
    padding: 12px;
    background: var(--color-fondo);
  }
  .grid:focus-visible {
    box-shadow: inset var(--foco-ring);
  }

  .grid__lienzo {
    position: relative;
    width: 100%;
  }

  .grid__vacio {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--color-texto-secundario);
  }

  .grid__fila {
    position: absolute;
    top: 0;
    left: 0;
    display: grid;
    gap: 12px;
    justify-content: start;
  }

  .cel {
    border-radius: var(--radio);
  }

  .cel__btn {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    width: var(--cell-size);
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-texto);
    cursor: pointer;
    text-align: center;
  }

  .cel__img {
    position: relative;
    width: var(--cell-size);
    border-radius: var(--radio);
    overflow: hidden;
    background: var(--color-panel);
    border: 2px solid transparent;
    display: grid;
    place-items: center;
    transition:
      border-color var(--transicion),
      box-shadow var(--transicion),
      transform var(--transicion);
  }
  .cel__btn:hover .cel__img {
    transform: translateY(-2px);
    box-shadow: var(--sombra-2);
  }

  .cel--sel .cel__img {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }
  .cel--foco .cel__img {
    border-color: var(--color-acento-hover);
  }

  .cel__pic {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  :global(.cel__pic--err) {
    visibility: hidden;
  }
  .cel__sinimg {
    color: var(--color-texto-tenue);
  }
  .cel__badge {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-panel);
  }

  /* Registro oculto: atenuado y con marca de "ojo tachado". */
  .cel--oculta .cel__pic,
  .cel--oculta .cel__sinimg {
    opacity: 0.35;
    filter: grayscale(0.6);
  }
  .cel--oculta .cel__etq {
    opacity: 0.5;
  }
  .cel__oculto {
    position: absolute;
    top: 4px;
    left: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
  }

  .cel__etq {
    font-size: var(--fz1, var(--tam-fuente-xs));
    color: var(--color-texto-secundario);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    height: var(--lh1, 18px);
    line-height: var(--lh1, 18px);
  }

  /* Bloque de datos: hermano del botón de imagen (permite inputs de edición),
     sin gap interno para que el alto calculado sea exacto. */
  .cel__datos {
    display: flex;
    flex-direction: column;
    gap: 0;
    min-width: 0;
    width: var(--cell-size);
    margin-top: var(--esp-1);
    text-align: center;
    cursor: default;
  }

  /* Línea modificable: invita a editar al pasar el cursor. */
  .cel__editable {
    cursor: text;
    border-radius: 3px;
  }
  .cel__editable:hover {
    background: var(--color-hover);
  }

  /* Input de edición directa en la card. */
  .cel__input {
    width: 100%;
    height: var(--lh2, 16px);
    padding: 0 4px;
    border: 1px solid var(--color-acento);
    border-radius: 3px;
    background: var(--color-superficie);
    color: var(--color-texto);
    font-size: var(--fz2, var(--tam-fuente-xs));
    text-align: center;
    outline: none;
  }

  /* Líneas de datos a la vista (campos visibles 2..N). */
  .cel__etq2 {
    font-size: var(--fz2, var(--tam-fuente-xs));
    color: var(--color-texto-secundario);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    height: var(--lh2, 16px);
    line-height: var(--lh2, 16px);
  }
  .cel__etqnom {
    color: var(--color-texto-tenue);
  }
  .cel--oculta .cel__etq2 {
    opacity: 0.5;
  }

  .cel__skeleton {
    width: var(--cell-size);
    border-radius: var(--radio);
    background: linear-gradient(
      90deg,
      var(--color-panel) 25%,
      var(--color-elevado) 50%,
      var(--color-panel) 75%
    );
    background-size: 200% 100%;
    animation: brillo 1.2s ease-in-out infinite;
  }

  @keyframes brillo {
    to {
      background-position: -200% 0;
    }
  }
</style>
