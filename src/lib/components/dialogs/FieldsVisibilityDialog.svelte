<!--
  FieldsVisibilityDialog — "Campos a la Vista" (equivale a frmDALV del
  original): qué campos de la tabla activa se muestran en la grilla/tabla,
  cuáles son modificables y en qué orden. Los cambios se aplican al instante
  (cada toggle persiste con campo_editar; el arrastre con campos_reordenar).
-->
<script lang="ts">
  import { GripVertical, Eye, EyeOff } from "lucide-svelte";
  import { Modal, Button, EmptyState } from "$lib/components/ui";
  import { campoEditar, camposReordenar } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { CampoDef, CampoNuevo } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // Copia local reordenable de los campos de la tabla activa.
  // svelte-ignore state_referenced_locally
  let campos = $state<CampoDef[]>(camposDeTabla());

  function camposDeTabla(): CampoDef[] {
    return estado.campos
      .filter((c) => c.tabla === estado.tabla)
      .slice()
      .sort((a, b) => a.ordenVisible - b.ordenVisible);
  }

  /** Reconstruye el CampoNuevo que espera `campo_editar` desde un CampoDef. */
  function aDef(c: CampoDef): CampoNuevo {
    return {
      nombre: c.nombre,
      tabla: c.tabla,
      tipo: c.tipo,
      decimales: c.decimales,
      totalizable: c.totalizable,
      formula: c.formula,
      visible: c.visible,
      modificable: c.modificable,
      ordenVisible: c.ordenVisible,
    };
  }

  /** Sincroniza el estado global del álbum con la lista local editada. */
  function publicar(): void {
    const otros = estado.campos.filter((c) => c.tabla !== estado.tabla);
    estado.setCampos([...otros, ...campos]);
    estado.refrescar();
  }

  /** Alterna visible/modificable de un campo y lo persiste al instante. */
  async function alternar(
    c: CampoDef,
    prop: "visible" | "modificable",
  ): Promise<void> {
    const previo = c[prop];
    c[prop] = !previo;
    try {
      const editado = await campoEditar(estado.albumId, c.id, aDef(c));
      campos = campos.map((x) => (x.id === c.id ? editado : x));
      publicar();
    } catch (e) {
      c[prop] = previo; // revierte el toggle optimista
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  // --- Reordenación por arrastre (mismo patrón que FieldStructureEditor) ---
  let arrastrado = $state<number | null>(null);

  function onDragStart(indice: number): void {
    arrastrado = indice;
  }

  function onDragOver(e: DragEvent, indice: number): void {
    e.preventDefault();
    if (arrastrado === null || arrastrado === indice) return;
    const copia = campos.slice();
    const [m] = copia.splice(arrastrado, 1);
    copia.splice(indice, 0, m);
    arrastrado = indice;
    campos = copia;
  }

  async function onDrop(): Promise<void> {
    if (arrastrado === null) return;
    arrastrado = null;
    try {
      // El backend reordena por ids globales: conserva el orden relativo de
      // la otra tabla y aplica el nuevo orden de la tabla activa.
      const otros = estado.campos
        .filter((c) => c.tabla !== estado.tabla)
        .sort((a, b) => a.ordenVisible - b.ordenVisible)
        .map((c) => c.id);
      await camposReordenar(estado.albumId, [...campos.map((c) => c.id), ...otros]);
      campos = campos.map((c, i) => ({ ...c, ordenVisible: i }));
      publicar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.camposVista.titulo} ancho="md" onCerrar={cerrar}>
  <div class="cv">
    <p class="cv__desc">{t.camposVista.descripcion}</p>

    {#if campos.length === 0}
      <EmptyState titulo={t.vacio.titulo} descripcion={t.vacio.descripcion} />
    {:else}
      <div class="cv__cabecera">
        <span></span>
        <span></span>
        <span class="cv__col">{t.camposVista.visible}</span>
        <span class="cv__col">{t.camposVista.modificable}</span>
      </div>
      <ul class="cv__lista">
        {#each campos as c, i (c.id)}
          <li
            class="cv__fila"
            class:cv__fila--arrastrando={arrastrado === i}
            class:cv__fila--oculta={!c.visible}
            draggable="true"
            ondragstart={() => onDragStart(i)}
            ondragover={(e) => onDragOver(e, i)}
            ondrop={onDrop}
            ondragend={onDrop}
          >
            <span class="cv__grip"><GripVertical size={14} /></span>
            <span class="cv__nombre">
              {#if c.visible}<Eye size={13} />{:else}<EyeOff size={13} />{/if}
              {c.nombre}
            </span>
            <span class="cv__col">
              <input
                type="checkbox"
                checked={c.visible}
                aria-label={`${t.camposVista.visible}: ${c.nombre}`}
                onchange={() => alternar(c, "visible")}
              />
            </span>
            <span class="cv__col">
              <input
                type="checkbox"
                checked={c.modificable}
                disabled={c.tipo === "calculado"}
                aria-label={`${t.camposVista.modificable}: ${c.nombre}`}
                onchange={() => alternar(c, "modificable")}
              />
            </span>
          </li>
        {/each}
      </ul>
      <p class="cv__nota">{t.camposVista.siempreLectura}</p>
    {/if}
  </div>

  {#snippet pie()}
    <Button variante="primario" onclick={cerrar}>{t.accion.cerrar}</Button>
  {/snippet}
</Modal>

<style>
  .cv {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .cv__desc {
    margin: 0;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .cv__cabecera,
  .cv__fila {
    display: grid;
    grid-template-columns: 20px 1fr 90px 110px;
    align-items: center;
    gap: var(--esp-2);
  }
  .cv__cabecera {
    padding: 0 var(--esp-2);
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-tenue);
  }
  .cv__lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 380px;
    overflow-y: auto;
  }
  .cv__fila {
    padding: var(--esp-1) var(--esp-2);
    border: 1px solid transparent;
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
    cursor: grab;
  }
  .cv__fila:hover {
    border-color: var(--color-borde);
  }
  .cv__fila--arrastrando {
    opacity: 0.5;
    border-color: var(--color-acento);
  }
  .cv__fila--oculta .cv__nombre {
    color: var(--color-texto-tenue);
  }
  .cv__grip {
    color: var(--color-texto-tenue);
    display: inline-flex;
  }
  .cv__nombre {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .cv__col {
    text-align: center;
  }
  .cv__nota {
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
</style>
