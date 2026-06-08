<!--
  TreeView — árbol genérico de nodos expandibles con conteo opcional. Funciona
  con cualquier estructura: se le pasan adaptadores para obtener etiqueta, hijos
  y conteo de cada nodo. Es recursivo (se renderiza a sí mismo en los hijos).

  El estado de expansión y de selección se controla por id externo (SvelteSet
  para expandidos, valor único para seleccionado) para que el contenedor mande.
-->
<script lang="ts" generics="N">
  import { ChevronRight } from "lucide-svelte";
  import Self from "./TreeView.svelte";

  interface Props {
    /** Nodos de este nivel. */
    nodos: N[];
    /** Id estable de un nodo (clave de expansión/selección). */
    idDe: (nodo: N) => string;
    /** Etiqueta visible de un nodo. */
    etiquetaDe: (nodo: N) => string;
    /** Hijos de un nodo (lista vacía si es hoja). */
    hijosDe: (nodo: N) => N[];
    /** Conteo a mostrar (null = sin conteo). */
    conteoDe?: (nodo: N) => number | null;
    /** Ids expandidos (compartido entre niveles). */
    expandidos: Set<string>;
    /** Id seleccionado (compartido entre niveles). */
    seleccionado?: string | null;
    onSeleccionar?: (nodo: N) => void;
    onAlternar?: (id: string) => void;
    /** Nivel de profundidad (uso interno para sangría). */
    nivel?: number;
  }

  let {
    nodos,
    idDe,
    etiquetaDe,
    hijosDe,
    conteoDe,
    expandidos,
    seleccionado = null,
    onSeleccionar,
    onAlternar,
    nivel = 0,
  }: Props = $props();

  function alternar(id: string): void {
    onAlternar?.(id);
  }
</script>

<ul class="tree" role={nivel === 0 ? "tree" : "group"}>
  {#each nodos as nodo (idDe(nodo))}
    {@const id = idDe(nodo)}
    {@const hijos = hijosDe(nodo)}
    {@const tieneHijos = hijos.length > 0}
    {@const abierto = expandidos.has(id)}
    {@const conteo = conteoDe?.(nodo) ?? null}
    <li
      class="tree__item"
      role="treeitem"
      aria-selected={seleccionado === id}
      aria-expanded={tieneHijos ? abierto : undefined}
    >
      <div
        class="tree__fila"
        class:tree__fila--sel={seleccionado === id}
        style="--nivel:{nivel}"
      >
        <button
          type="button"
          class="tree__toggle"
          class:tree__toggle--oculto={!tieneHijos}
          aria-label={abierto ? "Contraer" : "Expandir"}
          tabindex={tieneHijos ? 0 : -1}
          onclick={() => tieneHijos && alternar(id)}
        >
          <span class="tree__chev" class:tree__chev--abierto={abierto}>
            <ChevronRight size={14} />
          </span>
        </button>
        <button
          type="button"
          class="tree__etq"
          onclick={() => onSeleccionar?.(nodo)}
        >
          <span class="tree__txt">{etiquetaDe(nodo)}</span>
          {#if conteo !== null}
            <span class="tree__conteo tabular">{conteo}</span>
          {/if}
        </button>
      </div>

      {#if tieneHijos && abierto}
        <Self
          nodos={hijos}
          {idDe}
          {etiquetaDe}
          {hijosDe}
          {conteoDe}
          {expandidos}
          {seleccionado}
          {onSeleccionar}
          {onAlternar}
          nivel={nivel + 1}
        />
      {/if}
    </li>
  {/each}
</ul>

<style>
  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .tree__fila {
    display: flex;
    align-items: center;
    height: var(--alto-control-sm);
    padding-left: calc(var(--nivel) * var(--esp-3));
    border-radius: var(--radio-sm);
    cursor: default;
    transition: background var(--transicion-rapida);
  }
  .tree__fila:hover {
    background: var(--color-hover);
  }
  .tree__fila--sel {
    background: var(--color-seleccion);
  }

  .tree__toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 100%;
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--color-texto-secundario);
    cursor: pointer;
  }
  .tree__toggle--oculto {
    visibility: hidden;
    cursor: default;
  }

  .tree__chev {
    display: inline-flex;
    transition: transform var(--transicion);
  }
  .tree__chev--abierto {
    transform: rotate(90deg);
  }

  .tree__etq {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--esp-2);
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 var(--esp-2) 0 0;
    border: none;
    background: transparent;
    color: var(--color-texto);
    cursor: pointer;
  }

  .tree__txt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tree__conteo {
    flex-shrink: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
</style>
