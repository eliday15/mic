<!--
  MenuBar — barra de menús propia (sustituye al menú nativo) con desplegables
  Archivo / Editar / Ver / Herramientas / Ayuda. Cada menú es una lista de
  acciones; al elegir una se invoca `onAccion(id)`. Navegación con teclado y
  cierre al hacer clic fuera o pulsar Escape.
-->
<script lang="ts">
  import { t } from "$lib/i18n/es";

  export interface ItemMenu {
    id: string;
    etiqueta?: string;
    separador?: boolean;
    atajo?: string;
    deshabilitado?: boolean;
  }

  interface MenuDef {
    id: string;
    etiqueta: string;
    items: ItemMenu[];
  }

  interface Props {
    /** Indica si hay un álbum activo (habilita acciones dependientes). */
    hayAlbum: boolean;
    onAccion: (id: string) => void;
  }

  let { hayAlbum, onAccion }: Props = $props();

  let abierto = $state<string | null>(null);
  let raiz = $state<HTMLDivElement | null>(null);

  const menus = $derived<MenuDef[]>([
    {
      id: "archivo",
      etiqueta: t.menu.archivo,
      items: [
        { id: "nuevo-album", etiqueta: t.archivo.nuevoAlbum, atajo: "⌘N" },
        { id: "abrir", etiqueta: t.archivo.abrir, atajo: "⌘O" },
        { id: "migrar", etiqueta: t.archivo.importar },
        { id: "imprimir", etiqueta: t.archivo.imprimir, deshabilitado: !hayAlbum },
        { id: "s1", separador: true },
        { id: "copiar-album", etiqueta: t.archivo.copiarAlbum, deshabilitado: !hayAlbum },
        { id: "exportar", etiqueta: t.archivo.exportar, deshabilitado: !hayAlbum },
        { id: "importar-registros", etiqueta: t.importacion.accionMenu, deshabilitado: !hayAlbum },
        { id: "empacar", etiqueta: t.archivo.empacar, deshabilitado: !hayAlbum },
        { id: "desempacar", etiqueta: t.archivo.desempacar },
        { id: "compactar", etiqueta: t.archivo.compactar, deshabilitado: !hayAlbum },
        { id: "cerrar-album", etiqueta: t.archivo.cerrarAlbum, deshabilitado: !hayAlbum },
      ],
    },
    {
      id: "editar",
      etiqueta: t.menu.editar,
      items: [
        { id: "nueva-imagen", etiqueta: t.editar.nuevaImagen, deshabilitado: !hayAlbum },
        { id: "nueva-variante", etiqueta: t.editar.nuevaVariante, deshabilitado: !hayAlbum },
        { id: "editar-registro", etiqueta: t.editar.editarRegistro, deshabilitado: !hayAlbum },
        { id: "ocultar", etiqueta: t.editar.ocultar, deshabilitado: !hayAlbum },
        { id: "mostrar", etiqueta: t.editar.mostrar, deshabilitado: !hayAlbum },
        { id: "eliminar", etiqueta: t.editar.eliminar, deshabilitado: !hayAlbum },
        { id: "s2", separador: true },
        { id: "seleccionar-todo", etiqueta: t.editar.seleccionarTodo, deshabilitado: !hayAlbum },
        { id: "deseleccionar", etiqueta: t.editar.deseleccionar, deshabilitado: !hayAlbum },
        { id: "invertir-seleccion", etiqueta: t.editar.invertirSeleccion, deshabilitado: !hayAlbum },
      ],
    },
    {
      id: "ver",
      etiqueta: t.menu.ver,
      items: [
        { id: "vista-grilla", etiqueta: t.ver.grilla, deshabilitado: !hayAlbum },
        { id: "vista-tabla", etiqueta: t.ver.tabla, deshabilitado: !hayAlbum },
        { id: "s3", separador: true },
        { id: "panel-grupos", etiqueta: t.ver.panelGrupos, deshabilitado: !hayAlbum },
        { id: "inspector", etiqueta: t.ver.inspector, deshabilitado: !hayAlbum },
        { id: "s4", separador: true },
        { id: "campos-vista", etiqueta: t.ver.camposVista, deshabilitado: !hayAlbum },
        { id: "mostrar-ocultos", etiqueta: t.ver.mostrarOcultos, deshabilitado: !hayAlbum },
        { id: "visor", etiqueta: t.ver.visor, deshabilitado: !hayAlbum },
        { id: "s4b", separador: true },
        { id: "tema", etiqueta: t.ver.tema },
      ],
    },
    {
      id: "herramientas",
      etiqueta: t.menu.herramientas,
      items: [
        { id: "buscar", etiqueta: t.herramientas.buscar, atajo: "⌘F", deshabilitado: !hayAlbum },
        { id: "ordenar", etiqueta: t.herramientas.ordenar, deshabilitado: !hayAlbum },
        { id: "filtros", etiqueta: t.herramientas.filtros, deshabilitado: !hayAlbum },
        { id: "s5", separador: true },
        { id: "totalizar", etiqueta: t.herramientas.totalizar, deshabilitado: !hayAlbum },
        { id: "act-masiva", etiqueta: t.herramientas.actMasiva, deshabilitado: !hayAlbum },
        { id: "imagenes-carpeta", etiqueta: t.herramientas.imagenesCarpeta, deshabilitado: !hayAlbum },
        { id: "recalcular", etiqueta: t.herramientas.recalcular, deshabilitado: !hayAlbum },
        { id: "ligados", etiqueta: t.herramientas.ligados, deshabilitado: !hayAlbum },
        { id: "s6", separador: true },
        { id: "campos", etiqueta: t.herramientas.campos, deshabilitado: !hayAlbum },
      ],
    },
    {
      id: "ayuda",
      etiqueta: t.menu.ayuda,
      items: [{ id: "acerca", etiqueta: t.ayuda.acercaDe }],
    },
  ]);

  function alternar(id: string): void {
    abierto = abierto === id ? null : id;
  }

  function entrar(id: string): void {
    // Si ya hay un menú abierto, cambiar con hover (comportamiento de barra).
    if (abierto !== null) abierto = id;
  }

  function elegir(item: ItemMenu): void {
    if (item.separador || item.deshabilitado) return;
    abierto = null;
    onAccion(item.id);
  }

  $effect(() => {
    if (abierto === null) return;
    function onDoc(e: MouseEvent): void {
      if (raiz && !raiz.contains(e.target as Node)) abierto = null;
    }
    function onKey(e: KeyboardEvent): void {
      if (e.key === "Escape") abierto = null;
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  });
</script>

<div bind:this={raiz} class="mb" role="menubar">
  {#each menus as menu (menu.id)}
    <div class="mb__grupo">
      <button
        type="button"
        class="mb__btn"
        class:mb__btn--activo={abierto === menu.id}
        role="menuitem"
        aria-haspopup="true"
        aria-expanded={abierto === menu.id}
        onclick={() => alternar(menu.id)}
        onmouseenter={() => entrar(menu.id)}
      >
        {menu.etiqueta}
      </button>

      {#if abierto === menu.id}
        <div class="mb__menu" role="menu">
          {#each menu.items as item (item.id)}
            {#if item.separador}
              <div class="mb__sep" role="separator"></div>
            {:else}
              <button
                type="button"
                class="mb__item"
                role="menuitem"
                disabled={item.deshabilitado}
                onclick={() => elegir(item)}
              >
                <span class="mb__etq">{item.etiqueta}</span>
                {#if item.atajo}
                  <span class="mb__atajo">{item.atajo}</span>
                {/if}
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .mb {
    display: flex;
    align-items: stretch;
    height: 100%;
  }
  .mb__grupo {
    position: relative;
  }
  .mb__btn {
    height: 100%;
    padding: 0 var(--esp-3);
    border: none;
    background: transparent;
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
    transition:
      background var(--transicion-rapida),
      color var(--transicion-rapida);
  }
  .mb__btn:hover,
  .mb__btn--activo {
    background: var(--color-hover);
    color: var(--color-texto);
  }

  .mb__menu {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: var(--z-menu);
    min-width: 220px;
    padding: var(--esp-1);
    background: var(--color-elevado);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    box-shadow: var(--sombra-2);
  }

  .mb__item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--esp-4);
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
  .mb__item:hover:not(:disabled) {
    background: var(--color-acento-tenue);
    color: var(--color-acento);
  }
  .mb__item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .mb__atajo {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
  .mb__sep {
    height: 1px;
    margin: var(--esp-1);
    background: var(--color-borde);
  }
</style>
