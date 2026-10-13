<!--
  LigaForm — formulario de una liga (ex-frmEdligado del VB6), usado por
  LinkedAlbumsDialog. Elige el álbum ligado y el campo llave, muestra una vista
  previa de la estructura (campos que coinciden / que solo existen en el
  ligado, vía liga_inspeccionar) y las opciones: importar registros nuevos,
  importar campos nuevos y qué foto conservar cuando ambos tienen.
-->
<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Image as ImageIcon, TriangleAlert } from "lucide-svelte";
  import { Button, Chip, Select, Spinner, TextInput } from "$lib/components/ui";
  import {
    ligaInspeccionar,
    type FuenteFotos,
    type InspeccionLiga,
  } from "./ligadosIpc";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    estado: AlbumState;
    rutaAlbum: string;
    llave: string;
    crearFaltantes: boolean;
    importarCamposNuevos: boolean;
    fotos: FuenteFotos;
  }

  let {
    estado,
    rutaAlbum = $bindable(),
    llave = $bindable(),
    crearFaltantes = $bindable(),
    importarCamposNuevos = $bindable(),
    fotos = $bindable(),
  }: Props = $props();

  let inspeccion = $state<InspeccionLiga | null>(null);
  let inspeccionando = $state(false);
  let errorInspeccion = $state<string | null>(null);
  /** Descarta respuestas de inspecciones viejas si cambia la ruta a mitad. */
  let turno = 0;

  // Campos elegibles como llave: principales, no calculados ni multidato y,
  // si ya se inspeccionó el ligado, presentes en ambos álbumes.
  const camposLlave = $derived(
    estado.campos.filter(
      (c) =>
        c.tabla === "principal" &&
        c.tipo !== "calculado" &&
        c.tipo !== "multidato" &&
        (inspeccion === null || inspeccion.camposComunes.includes(c.nombre)),
    ),
  );

  const llaveNoComun = $derived(
    inspeccion !== null &&
      llave !== "" &&
      !inspeccion.camposComunes.includes(llave),
  );

  $effect(() => {
    const ruta = rutaAlbum.trim();
    void inspeccionar(ruta);
  });

  // Al conocer los campos comunes, si la llave elegida no es común se propone
  // la primera común.
  $effect(() => {
    if (llaveNoComun && camposLlave.length > 0) llave = camposLlave[0].nombre;
  });

  async function inspeccionar(ruta: string): Promise<void> {
    const mio = ++turno;
    inspeccion = null;
    errorInspeccion = null;
    if (ruta === "") return;
    inspeccionando = true;
    try {
      const r = await ligaInspeccionar(estado.albumId, ruta);
      if (mio === turno) inspeccion = r;
    } catch (e) {
      if (mio === turno) errorInspeccion = typeof e === "string" ? e : t.error.generico;
    } finally {
      if (mio === turno) inspeccionando = false;
    }
  }

  async function elegirAlbum(): Promise<void> {
    const sel = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "MIC", extensions: ["micdb"] }],
    });
    if (typeof sel === "string") rutaAlbum = sel;
  }
</script>

<div class="lf">
  <label class="lf__campo">
    <span class="lf__etq">{t.ligados.album}</span>
    <div class="lf__ruta">
      <TextInput bind:value={rutaAlbum} readonly placeholder="…/album.micdb" />
      <Button variante="secundario" onclick={elegirAlbum}>
        <FolderOpen size={16} />
        {t.ligados.elegirAlbum}
      </Button>
    </div>
  </label>

  {#if inspeccionando}
    <div class="lf__previa lf__previa--cargando">
      <Spinner tamano={14} />
      <span>{t.ligados.inspeccionando}</span>
    </div>
  {:else if errorInspeccion}
    <div class="lf__previa lf__previa--error">
      <TriangleAlert size={14} />
      <span>{errorInspeccion}</span>
    </div>
  {:else if inspeccion}
    <section class="lf__previa" aria-label={t.ligados.vistaPrevia}>
      <div class="lf__previa-cab">
        <span class="lf__etq">{t.ligados.vistaPrevia}</span>
        <span class="lf__resumen">
          {t.ligados.resumenLigado(inspeccion.totalRegistros, inspeccion.conImagen)}
        </span>
      </div>
      <div class="lf__grupo">
        <span class="lf__subetq">{t.ligados.camposComunes}</span>
        <div class="lf__chips">
          {#each inspeccion.camposComunes as nombre (nombre)}
            <Chip variante="acento">{nombre}</Chip>
          {:else}
            <span class="lf__nada">{t.ligados.ninguno}</span>
          {/each}
        </div>
      </div>
      <div class="lf__grupo">
        <span class="lf__subetq">{t.ligados.camposSoloLigado}</span>
        <div class="lf__chips">
          {#each inspeccion.camposSoloLigado as nombre (nombre)}
            <Chip>{nombre}</Chip>
          {:else}
            <span class="lf__nada">{t.ligados.ninguno}</span>
          {/each}
        </div>
      </div>
      {#if inspeccion.camposNoImportables.length > 0}
        <div class="lf__grupo">
          <span class="lf__subetq">{t.ligados.camposNoImportables}</span>
          <span class="lf__nada">{inspeccion.camposNoImportables.join(", ")}</span>
        </div>
      {/if}
    </section>
  {/if}

  <label class="lf__campo">
    <span class="lf__etq">{t.ligados.llave}</span>
    <Select
      bind:value={llave}
      opciones={camposLlave.map((c) => ({
        valor: c.nombre,
        etiqueta: c.nombre,
      }))}
      etiqueta={t.ligados.llave}
      invalido={llaveNoComun}
    />
    {#if llaveNoComun}
      <span class="lf__aviso">{t.ligados.llaveNoComun}</span>
    {/if}
  </label>

  <fieldset class="lf__grupo-opc">
    <legend class="lf__etq">{t.ligados.opciones}</legend>
    <label class="lf__check">
      <input type="checkbox" bind:checked={crearFaltantes} />
      <span>{t.ligados.crearSiNoExiste}</span>
    </label>
    <label class="lf__check">
      <input type="checkbox" bind:checked={importarCamposNuevos} />
      <span>
        {t.ligados.importarCampos}
        {#if inspeccion}
          ({inspeccion.camposSoloLigado.length})
        {/if}
        <small class="lf__nota">{t.ligados.importarCamposNota}</small>
      </span>
    </label>
  </fieldset>

  <fieldset class="lf__grupo-opc">
    <legend class="lf__etq">{t.ligados.fotos}</legend>
    <p class="lf__nota lf__nota--icono">
      <ImageIcon size={14} />
      {t.ligados.fotosNota}
    </p>
    <label class="lf__check">
      <input type="radio" bind:group={fotos} value="actual" />
      <span>{t.ligados.fotosActual}</span>
    </label>
    <label class="lf__check">
      <input type="radio" bind:group={fotos} value="ligado" />
      <span>{t.ligados.fotosLigado}</span>
    </label>
  </fieldset>
</div>

<style>
  .lf {
    display: flex;
    flex-direction: column;
    gap: var(--esp-4);
  }
  .lf__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .lf__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .lf__subetq {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .lf__ruta {
    display: flex;
    gap: var(--esp-2);
  }
  .lf__ruta :global(.campo) {
    flex: 1;
  }
  .lf__previa {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding: var(--esp-3);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-panel);
  }
  .lf__previa--cargando,
  .lf__previa--error {
    flex-direction: row;
    align-items: center;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .lf__previa--error {
    color: var(--color-peligro);
  }
  .lf__previa-cab {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--esp-2);
  }
  .lf__resumen {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .lf__grupo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .lf__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }
  .lf__nada {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
  .lf__aviso {
    font-size: var(--tam-fuente-xs);
    color: var(--color-aviso);
  }
  .lf__grupo-opc {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    margin: 0;
    padding: 0;
    border: none;
  }
  .lf__grupo-opc legend {
    margin-bottom: var(--esp-2);
  }
  .lf__check {
    display: flex;
    align-items: flex-start;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
  }
  .lf__check input {
    margin-top: 2px;
  }
  .lf__nota {
    display: block;
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .lf__nota--icono {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
  }
</style>
