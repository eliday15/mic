<!--
  Combobox — entrada de texto con autocompletado asíncrono. Llama a `buscar`
  (con debounce) y muestra las sugerencias en un panel navegable por teclado.
  Permite valores libres: el texto tecleado se enlaza en `value`.
-->
<script lang="ts">
  import { debounce } from "$lib/utils/debounce";
  import Spinner from "./Spinner.svelte";

  interface Props {
    /** Texto actual enlazado bidireccionalmente. */
    value?: string;
    /** Proveedor asíncrono de sugerencias por prefijo. */
    buscar: (prefijo: string) => Promise<string[]>;
    placeholder?: string;
    invalido?: boolean;
    disabled?: boolean;
    etiqueta?: string;
    /** Retardo del debounce en ms. */
    retardo?: number;
    /** Mínimo de caracteres para disparar la búsqueda. */
    minimo?: number;
    onSeleccionar?: (valor: string) => void;
  }

  let {
    value = $bindable(""),
    buscar,
    placeholder,
    invalido = false,
    disabled = false,
    etiqueta,
    retardo = 200,
    minimo = 1,
    onSeleccionar,
  }: Props = $props();

  let abierto = $state(false);
  let cargando = $state(false);
  let sugerencias = $state<string[]>([]);
  let resaltado = $state(-1);
  let raiz = $state<HTMLDivElement | null>(null);

  // Token para descartar respuestas obsoletas (carreras de red).
  let token = 0;

  // El retardo se fija al construir el debounce (no se espera que cambie en
  // caliente); se captura en una constante para dejar la intención explícita.
  // svelte-ignore state_referenced_locally
  const retardoDebounce = retardo;

  const lanzar = debounce(async (prefijo: string) => {
    const mio = ++token;
    cargando = true;
    try {
      const res = await buscar(prefijo);
      if (mio !== token) return; // respuesta vieja
      sugerencias = res;
      resaltado = res.length > 0 ? 0 : -1;
      abierto = res.length > 0;
    } finally {
      if (mio === token) cargando = false;
    }
  }, retardoDebounce);

  function onInput(e: Event): void {
    value = (e.currentTarget as HTMLInputElement).value;
    if (value.trim().length >= minimo) {
      lanzar(value.trim());
    } else {
      lanzar.cancel();
      sugerencias = [];
      abierto = false;
      cargando = false;
    }
  }

  function elegir(valor: string): void {
    value = valor;
    abierto = false;
    sugerencias = [];
    resaltado = -1;
    onSeleccionar?.(valor);
  }

  function onKeydown(e: KeyboardEvent): void {
    if (!abierto || sugerencias.length === 0) {
      if (e.key === "ArrowDown" && value.trim().length >= minimo) {
        lanzar(value.trim());
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        resaltado = (resaltado + 1) % sugerencias.length;
        break;
      case "ArrowUp":
        e.preventDefault();
        resaltado =
          (resaltado - 1 + sugerencias.length) % sugerencias.length;
        break;
      case "Enter":
        if (resaltado >= 0) {
          e.preventDefault();
          elegir(sugerencias[resaltado]);
        }
        break;
      case "Escape":
        e.preventDefault();
        // Solo cierra el panel: que no llegue a la capa global (cerraría el modal).
        e.stopPropagation();
        abierto = false;
        break;
    }
  }

  // Cierra el panel al perder el foco fuera del componente.
  $effect(() => {
    if (!abierto) return;
    function onDocClick(e: MouseEvent): void {
      if (raiz && !raiz.contains(e.target as Node)) abierto = false;
    }
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  });
</script>

<div class="combo" bind:this={raiz}>
  <div class="combo__campo" class:combo__campo--invalido={invalido}>
    <input
      class="combo__input"
      type="text"
      role="combobox"
      aria-expanded={abierto}
      aria-controls="combo-lista"
      aria-autocomplete="list"
      aria-label={etiqueta}
      aria-invalid={invalido}
      {placeholder}
      {disabled}
      bind:value
      oninput={onInput}
      onkeydown={onKeydown}
      onfocus={() => {
        if (sugerencias.length > 0) abierto = true;
      }}
    />
    {#if cargando}
      <span class="combo__estado"><Spinner tamano={14} /></span>
    {/if}
  </div>

  {#if abierto && sugerencias.length > 0}
    <ul class="combo__lista" id="combo-lista" role="listbox">
      {#each sugerencias as s, i (s)}
        <li
          class="combo__op"
          class:combo__op--resaltado={i === resaltado}
          role="option"
          aria-selected={i === resaltado}
          onmousedown={(e) => {
            e.preventDefault();
            elegir(s);
          }}
          onmouseenter={() => (resaltado = i)}
        >
          {s}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .combo {
    position: relative;
    width: 100%;
  }

  .combo__campo {
    display: flex;
    align-items: center;
    height: var(--alto-control);
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
    transition: border-color var(--transicion);
  }
  .combo__campo:focus-within {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }
  .combo__campo--invalido {
    border-color: var(--color-peligro);
  }

  .combo__input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--color-texto);
    outline: none;
  }
  .combo__input::placeholder {
    color: var(--color-texto-tenue);
  }

  .combo__estado {
    display: inline-flex;
    margin-left: var(--esp-1);
  }

  .combo__lista {
    position: absolute;
    z-index: var(--z-menu);
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    max-height: 240px;
    margin: 0;
    padding: var(--esp-1);
    list-style: none;
    overflow-y: auto;
    background: var(--color-elevado);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    box-shadow: var(--sombra-2);
  }

  .combo__op {
    padding: var(--esp-1) var(--esp-2);
    border-radius: var(--radio-sm);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .combo__op--resaltado {
    background: var(--color-acento-tenue);
    color: var(--color-acento);
  }
</style>
