<!--
  TableView — vista de tabla densa con filas virtuales. Reusa las mismas
  ventanas de datos que la grilla (CacheVentanas). Las columnas son los campos
  visibles; el encabezado ordena por click (asc → desc → quitar). Edición de
  celda inline con Enter/Tab que confirma vía `registro_editar`. Incluye una
  columna de miniatura pequeña.
-->
<script lang="ts">
  import { ArrowUp, ArrowDown, ImageOff } from "lucide-svelte";
  import type { CacheVentanas } from "$lib/utils/ventanas";
  import { thumbUrl } from "$lib/ipc/thumbnails";
  import { formatearPorTipo } from "$lib/utils/format";
  import { registroEditar } from "$lib/ipc/commands";
  import { validarValor } from "$lib/domain/validation";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { CampoDef, RegistroLigero, Valor } from "$lib/domain/types";

  interface Props {
    estado: AlbumState;
    /** Caché de ventanas compartida (la crea AlbumView). */
    cache: CacheVentanas;
    /** Contador de recarga: cambia cuando la caché obtiene páginas nuevas. */
    tick: number;
    /** Solicita un re-render tras una edición local de celda. */
    onRefrescar?: () => void;
    onAbrir?: (id: number) => void;
  }

  let { estado, cache, tick, onRefrescar, onAbrir }: Props = $props();

  const ALTO_FILA = 34;
  const OVERSCAN = 4;
  const ANCHO_MINI = 44;

  let contenedor = $state<HTMLDivElement | null>(null);
  let altoViewport = $state(0);
  let scrollTop = $state(0);
  // Celda en edición: `${indice}:${campoId}`.
  let editando = $state<string | null>(null);
  let textoEdicion = $state("");

  const columnas = $derived(estado.camposVisibles);
  const total = $derived(estado.total);
  const altoTotal = $derived(total * ALTO_FILA);

  const filaPrimera = $derived(
    Math.max(0, Math.floor(scrollTop / ALTO_FILA) - OVERSCAN),
  );
  const visibles = $derived(Math.ceil(altoViewport / ALTO_FILA) + OVERSCAN * 2);
  const filaUltima = $derived(Math.min(total, filaPrimera + visibles));

  $effect(() => {
    void tick;
    cache.asegurarRango(filaPrimera, filaUltima);
  });

  $effect(() => {
    void estado.versionConsulta;
    if (contenedor) contenedor.scrollTop = 0;
    scrollTop = 0;
  });

  function onScroll(): void {
    if (contenedor) scrollTop = contenedor.scrollTop;
  }

  function registroEn(indice: number): RegistroLigero | undefined {
    void tick;
    return cache.obtener(indice);
  }

  function dirOrden(campo: CampoDef): "asc" | "desc" | null {
    const o = estado.orden.find((x) => x.campo === campo.nombre);
    return o ? o.direccion : null;
  }

  function clickHeader(campo: CampoDef): void {
    estado.alternarOrden(campo.nombre);
  }

  function clickFila(e: MouseEvent, reg: RegistroLigero): void {
    if (e.metaKey || e.ctrlKey) estado.alternarSeleccion(reg.id);
    else estado.seleccionarUno(reg.id);
  }

  // --- Edición inline ----------------------------------------------------
  function editable(campo: CampoDef): boolean {
    return (
      campo.modificable &&
      campo.tipo !== "calculado" &&
      campo.tipo !== "multidato"
    );
  }

  function iniciarEdicion(
    indice: number,
    campo: CampoDef,
    reg: RegistroLigero,
  ): void {
    if (!editable(campo)) return;
    editando = `${indice}:${campo.id}`;
    const v = reg.valores[campo.nombre] ?? null;
    textoEdicion = v === null ? "" : String(v);
  }

  async function confirmarEdicion(
    campo: CampoDef,
    reg: RegistroLigero,
  ): Promise<void> {
    editando = null;
    const bruto: Valor = textoEdicion === "" ? null : textoEdicion;
    const res = validarValor(campo, bruto);
    if (!res.ok) {
      ui.error(res.error ?? t.error.valorInvalido);
      return;
    }
    const nuevo = res.valor ?? null;
    if ((reg.valores[campo.nombre] ?? null) === nuevo) return;
    try {
      const actualizado = await registroEditar(estado.albumId, reg.id, estado.tabla, {
        [campo.nombre]: nuevo,
      });
      // Refleja en la caché local mutando el registro en memoria.
      for (const c of columnas) {
        reg.valores[c.nombre] = actualizado.valores[c.nombre] ?? reg.valores[c.nombre];
      }
      onRefrescar?.();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
    }
  }

  function onKeyCelda(
    e: KeyboardEvent,
    campo: CampoDef,
    reg: RegistroLigero,
  ): void {
    if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      confirmarEdicion(campo, reg);
    } else if (e.key === "Escape") {
      e.preventDefault();
      editando = null;
    }
  }

  $effect(() => {
    if (!contenedor) return;
    const ro = new ResizeObserver((ents) => {
      altoViewport = ents[0].contentRect.height;
    });
    ro.observe(contenedor);
    return () => ro.disconnect();
  });
</script>

<div class="tabla">
  <!-- Encabezado fijo -->
  <div class="tabla__head" role="row">
    <div class="tabla__celd tabla__celd--mini" style="width:{ANCHO_MINI}px"></div>
    {#each columnas as campo (campo.id)}
      <button
        type="button"
        class="tabla__th"
        class:tabla__th--num={campo.tipo === "numerico" || campo.tipo === "moneda"}
        onclick={() => clickHeader(campo)}
      >
        <span class="tabla__thtxt">{campo.nombre}</span>
        {#if dirOrden(campo) === "asc"}
          <ArrowUp size={12} />
        {:else if dirOrden(campo) === "desc"}
          <ArrowDown size={12} />
        {/if}
      </button>
    {/each}
  </div>

  <!-- Cuerpo virtual -->
  <div
    bind:this={contenedor}
    class="tabla__body"
    onscroll={onScroll}
    role="rowgroup"
  >
    <div class="tabla__lienzo" style="height:{altoTotal}px">
      {#each Array.from({ length: Math.max(0, filaUltima - filaPrimera) }) as _, fi (filaPrimera + fi)}
        {@const indice = filaPrimera + fi}
        {@const reg = registroEn(indice)}
        <div
          class="tabla__fila"
          class:tabla__fila--sel={reg && estado.estaSeleccionado(reg.id)}
          style="transform:translateY({indice * ALTO_FILA}px)"
          role="row"
        >
          {#if reg}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="tabla__celd tabla__celd--mini"
              style="width:{ANCHO_MINI}px"
              onclick={(e) => clickFila(e, reg)}
              ondblclick={() => onAbrir?.(reg.id)}
            >
              {#if reg.imagen}
                <img
                  class="tabla__mini"
                  src={thumbUrl(estado.albumId, estado.tabla, reg.id, 128, reg.imagenVersion ?? 0)}
                  alt=""
                  loading="lazy"
                />
              {:else}
                <span class="tabla__sinimg"><ImageOff size={14} /></span>
              {/if}
            </div>
            {#each columnas as campo (campo.id)}
              {@const clave = `${indice}:${campo.id}`}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="tabla__celd"
                class:tabla__celd--num={campo.tipo === "numerico" || campo.tipo === "moneda"}
                class:tabla__celd--edit={editando === clave}
                onclick={(e) => clickFila(e, reg)}
                ondblclick={() => iniciarEdicion(indice, campo, reg)}
              >
                {#if editando === clave}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    class="tabla__input"
                    bind:value={textoEdicion}
                    autofocus
                    onkeydown={(e) => onKeyCelda(e, campo, reg)}
                    onblur={() => confirmarEdicion(campo, reg)}
                  />
                {:else}
                  <span class="tabla__valor">
                    {campo.tipo === "multidato"
                      ? `${reg.valores[campo.nombre] ?? ""}`
                      : formatearPorTipo(reg.valores[campo.nombre] ?? null, campo.tipo, campo.decimales, campo.formato)}
                  </span>
                {/if}
              </div>
            {/each}
          {:else}
            <div class="tabla__celd tabla__celd--mini" style="width:{ANCHO_MINI}px">
              <span class="tabla__sk"></span>
            </div>
            {#each columnas as campo (campo.id)}
              <div class="tabla__celd"><span class="tabla__sk"></span></div>
            {/each}
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .tabla {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-fondo);
  }

  .tabla__head {
    display: flex;
    align-items: stretch;
    height: var(--alto-control);
    background: var(--color-panel);
    border-bottom: 1px solid var(--color-borde);
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .tabla__th {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    flex: 1;
    min-width: 120px;
    padding: 0 var(--esp-2);
    border: none;
    border-right: 1px solid var(--color-borde);
    background: transparent;
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .tabla__th:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }
  .tabla__th--num {
    justify-content: flex-end;
  }
  .tabla__thtxt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tabla__body {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .tabla__lienzo {
    position: relative;
    width: 100%;
  }

  .tabla__fila {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    display: flex;
    height: 34px;
    border-bottom: 1px solid var(--color-borde);
  }
  .tabla__fila:hover {
    background: var(--color-hover);
  }
  .tabla__fila--sel {
    background: var(--color-seleccion);
  }

  .tabla__celd {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 120px;
    padding: 0 var(--esp-2);
    border-right: 1px solid var(--color-borde);
    overflow: hidden;
  }
  .tabla__celd--num {
    justify-content: flex-end;
    font-variant-numeric: tabular-nums;
  }
  .tabla__celd--mini {
    flex: 0 0 auto;
    justify-content: center;
    min-width: 0;
  }
  .tabla__celd--edit {
    padding: 0;
  }

  .tabla__valor {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }
  .tabla__celd--num .tabla__valor {
    text-align: right;
  }

  .tabla__mini {
    width: 28px;
    height: 28px;
    border-radius: var(--radio-sm);
    object-fit: cover;
  }
  .tabla__sinimg {
    color: var(--color-texto-tenue);
  }

  .tabla__input {
    width: 100%;
    height: 100%;
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-acento);
    background: var(--color-superficie);
    color: var(--color-texto);
    outline: none;
  }

  .tabla__sk {
    display: block;
    width: 60%;
    height: 10px;
    border-radius: var(--radio-pill);
    background: var(--color-elevado);
  }
</style>
