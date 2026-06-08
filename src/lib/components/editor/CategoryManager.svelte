<!--
  CategoryManager — CRUD del catálogo de categorías de un campo multidato.
  Carga `categorias_listar`, permite agregar/quitar/marcar por-defecto y persiste
  con `categorias_actualizar`. Se muestra dentro de un modal.
-->
<script lang="ts">
  import { Plus, Trash2, Star } from "lucide-svelte";
  import { Modal, Button, TextInput, IconButton } from "$lib/components/ui";
  import { categoriasListar, categoriasActualizar } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { CategoriaVal } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    albumId: number;
    campoId: number;
    /** True si el campo pertenece a la tabla principal. */
    principal: boolean;
    titulo?: string;
    onCerrar?: () => void;
    /** Notifica que el catálogo cambió (para refrescar sugerencias). */
    onCambio?: () => void;
  }

  let {
    abierto = $bindable(true),
    albumId,
    campoId,
    principal,
    titulo,
    onCerrar,
    onCambio,
  }: Props = $props();

  let valores = $state<CategoriaVal[]>([]);
  let nuevo = $state("");
  let cargando = $state(false);
  let guardando = $state(false);

  $effect(() => {
    if (abierto) cargar();
  });

  async function cargar(): Promise<void> {
    cargando = true;
    try {
      valores = await categoriasListar(albumId, campoId, principal);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      cargando = false;
    }
  }

  function agregar(): void {
    const v = nuevo.trim();
    if (v === "") return;
    if (valores.some((c) => c.valor.toLowerCase() === v.toLowerCase())) {
      nuevo = "";
      return;
    }
    valores = [...valores, { valor: v, esDefault: false }];
    nuevo = "";
  }

  function quitar(indice: number): void {
    valores = valores.filter((_, i) => i !== indice);
  }

  function alternarDefault(indice: number): void {
    valores = valores.map((c, i) =>
      i === indice ? { ...c, esDefault: !c.esDefault } : c,
    );
  }

  async function guardar(): Promise<void> {
    guardando = true;
    try {
      await categoriasActualizar(albumId, campoId, principal, valores);
      ui.exito(t.mensaje.guardado);
      onCambio?.();
      cerrar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      guardando = false;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }

  function onKeyNuevo(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      agregar();
    }
  }
</script>

<Modal
  bind:abierto
  titulo={titulo ?? t.categorias.titulo}
  ancho="sm"
  onCerrar={cerrar}
>
  <div class="cat">
    <div class="cat__alta">
      <TextInput
        bind:value={nuevo}
        placeholder={t.categorias.valor}
        onkeydown={onKeyNuevo}
      />
      <Button variante="secundario" onclick={agregar} aria-label={t.categorias.agregar}>
        <Plus size={16} />
      </Button>
    </div>

    {#if cargando}
      <p class="cat__estado">{t.app.cargando}</p>
    {:else if valores.length === 0}
      <p class="cat__estado">{t.categorias.sinCategorias}</p>
    {:else}
      <ul class="cat__lista">
        {#each valores as c, i (c.valor)}
          <li class="cat__fila">
            <IconButton
              etiqueta={t.categorias.porDefecto}
              tamano="sm"
              activo={c.esDefault}
              onclick={() => alternarDefault(i)}
            >
              <Star size={14} fill={c.esDefault ? "currentColor" : "none"} />
            </IconButton>
            <span class="cat__txt">{c.valor}</span>
            <IconButton
              etiqueta={t.accion.quitar}
              tamano="sm"
              onclick={() => quitar(i)}
            >
              <Trash2 size={14} />
            </IconButton>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    <Button variante="primario" cargando={guardando} onclick={guardar}>
      {t.accion.guardar}
    </Button>
  {/snippet}
</Modal>

<style>
  .cat {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .cat__alta {
    display: flex;
    gap: var(--esp-2);
  }
  .cat__alta :global(.campo) {
    flex: 1;
  }
  .cat__estado {
    margin: 0;
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-sm);
    text-align: center;
    padding: var(--esp-4);
  }
  .cat__lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 320px;
    overflow-y: auto;
  }
  .cat__fila {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    padding: var(--esp-1);
    border-radius: var(--radio-sm);
  }
  .cat__fila:hover {
    background: var(--color-hover);
  }
  .cat__txt {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
