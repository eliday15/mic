<!--
  ImportarDialog — importa registros desde un archivo CSV o XLSX al álbum activo
  (ex "Importar..." del VB6, Module3.bas ActualizaConImportado). Híbrido de
  MigrateDialog (file picker + inspección con chips) y LinkedAlbumsDialog (barra
  de progreso + resultado). Flujo en fases:
    elegir → configurar → ejecutando(analizando) → resumen → ejecutando(aplicando) → resultado.
  El análisis previo (dry-run) muestra "N se actualizarán · M se crearán · …"
  antes de aplicar nada. La huella de la inspección se pasa al aplicar para que
  el backend valide que el archivo no cambió. La grilla se refresca al cerrar.
-->
<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, ArrowRight } from "lucide-svelte";
  import { Modal, Button, Select, TextInput } from "$lib/components/ui";
  import {
    importarInspeccionar,
    importarRegistros,
    escucharImportacionProgreso,
    type InspeccionImport,
    type ResultadoImportacion,
    type ImportacionProgreso,
    type PoliticaImport,
  } from "./importarIpc";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { UnlistenFn } from "$lib/ipc/events";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onAplicado?: () => void;
    onCerrar?: () => void;
  }

  let {
    abierto = $bindable(true),
    estado,
    onAplicado,
    onCerrar,
  }: Props = $props();

  type Fase = "elegir" | "configurar" | "ejecutando" | "resumen" | "resultado";

  let fase = $state<Fase>("elegir");
  let rutaArchivo = $state("");
  let inspeccion = $state<InspeccionImport | null>(null);
  let campoLlave = $state("");
  let politica = $state<PoliticaImport>("sustituir");
  let crearFaltantes = $state(false);
  let resumen = $state<ResultadoImportacion | null>(null); // dry-run
  let resultado = $state<ResultadoImportacion | null>(null); // apply
  let progreso = $state<ImportacionProgreso | null>(null);
  let unlisten: UnlistenFn | null = null;

  // Opciones de llave: solo los campos sugeridos por el backend (principales,
  // no calculados ni multidato, y presentes en el archivo).
  const opcionesLlave = $derived(
    (inspeccion?.camposLlaveSugeridos ?? []).map((n) => ({
      valor: n,
      etiqueta: n,
    })),
  );

  $effect(() => {
    // Limpieza de la suscripción al desmontar (igual que LinkedAlbumsDialog).
    return () => {
      unlisten?.();
      unlisten = null;
    };
  });

  /** Abre el diálogo del sistema y, al elegir archivo, lo inspecciona. */
  async function examinar(): Promise<void> {
    const sel = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "CSV", extensions: ["csv"] },
        { name: "Excel", extensions: ["xlsx"] },
      ],
    });
    if (typeof sel !== "string") return;
    rutaArchivo = sel;
    await inspeccionar();
  }

  async function inspeccionar(): Promise<void> {
    try {
      inspeccion = await importarInspeccionar(estado.albumId, rutaArchivo);
      // Llave por defecto: la primera columna si es elegible; si no, la
      // primera sugerida.
      const primera = inspeccion.columnas[0] ?? "";
      campoLlave = inspeccion.camposLlaveSugeridos.includes(primera)
        ? primera
        : (inspeccion.camposLlaveSugeridos[0] ?? "");
      fase = "configurar";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.importacion);
    }
  }

  /** Análisis en seco (dry-run): clasifica sin escribir y muestra el resumen. */
  async function analizar(): Promise<void> {
    fase = "ejecutando";
    progreso = null;
    resumen = null;
    unlisten = await escucharImportacionProgreso((p) => (progreso = p));
    try {
      resumen = await importarRegistros(
        estado.albumId,
        rutaArchivo,
        campoLlave,
        politica,
        crearFaltantes,
        true,
        inspeccion?.huella,
      );
      fase = "resumen";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.importacion);
      fase = "configurar";
    } finally {
      unlisten?.();
      unlisten = null;
    }
  }

  /** Aplica la importación de verdad y refresca la grilla. */
  async function aplicar(): Promise<void> {
    fase = "ejecutando";
    progreso = null;
    resultado = null;
    unlisten = await escucharImportacionProgreso((p) => (progreso = p));
    try {
      resultado = await importarRegistros(
        estado.albumId,
        rutaArchivo,
        campoLlave,
        politica,
        crearFaltantes,
        false,
        inspeccion?.huella,
      );
      fase = "resultado";
      onAplicado?.();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.importacion);
      fase = "resumen";
    } finally {
      unlisten?.();
      unlisten = null;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }

  /** Etiqueta de la barra de progreso según la fase informada por el backend. */
  const faseLbl = $derived(
    progreso?.fase === "aplicando"
      ? t.importacion.aplicando
      : t.importacion.analizando,
  );

  const pct = $derived(
    progreso && progreso.total > 0
      ? Math.min(100, Math.round((progreso.hechas / progreso.total) * 100))
      : 0,
  );
</script>

<!-- Sin cierre por Escape ni clic fuera mientras la operación corre. -->
<Modal
  bind:abierto
  titulo={t.importacion.titulo}
  ancho="md"
  cerrarEscape={fase !== "ejecutando"}
  cerrarFuera={fase !== "ejecutando"}
  onCerrar={cerrar}
>
  {#if fase === "elegir"}
    <div class="im">
      <label class="im__campo">
        <span class="im__etq">{t.importacion.archivo}</span>
        <div class="im__ruta">
          <TextInput
            bind:value={rutaArchivo}
            readonly
            placeholder="…/datos.csv"
          />
          <Button variante="secundario" onclick={examinar}>
            <FolderOpen size={16} />
            {t.importacion.examinar}
          </Button>
        </div>
      </label>
      <p class="im__nota">{t.importacion.formatos}</p>
    </div>
  {:else if fase === "configurar" && inspeccion}
    <div class="im">
      <div class="im__resumen">
        <div class="im__dato">
          <span class="im__rotulo">{t.importacion.encoding}</span>
          <span class="im__valor">{inspeccion.encoding}</span>
        </div>
        <div class="im__dato">
          <span class="im__rotulo">{t.importacion.totalFilas}</span>
          <span class="im__valor tabular">{inspeccion.totalFilas}</span>
        </div>
      </div>

      <div class="im__grupo">
        <span class="im__etq">{t.importacion.columnasReconocidas}</span>
        <div class="im__chips">
          {#each inspeccion.columnasReconocidas as col (col)}
            <span class="im__chip">{col}</span>
          {/each}
        </div>
      </div>

      {#if inspeccion.columnasNoReconocidas.length > 0}
        <div class="im__grupo">
          <span class="im__etq">{t.importacion.columnasNoReconocidas}</span>
          <div class="im__chips">
            {#each inspeccion.columnasNoReconocidas as col (col)}
              <span class="im__chip im__chip--ignorada">{col}</span>
            {/each}
          </div>
        </div>
      {/if}

      <label class="im__campo">
        <span class="im__etq">{t.importacion.campoLlave}</span>
        <Select
          bind:value={campoLlave}
          opciones={opcionesLlave}
          etiqueta={t.importacion.campoLlave}
        />
        <span class="im__nota">{t.importacion.campoLlaveNota}</span>
      </label>

      <fieldset class="im__radios">
        <legend class="im__etq">{t.importacion.politica}</legend>
        <label class="im__radio">
          <input type="radio" bind:group={politica} value="sustituir" />
          <span>{t.importacion.sustituir}</span>
        </label>
        <label class="im__radio">
          <input type="radio" bind:group={politica} value="mantener" />
          <span>{t.importacion.mantener}</span>
        </label>
        <label class="im__radio">
          <input type="radio" bind:group={politica} value="rellenar_vacios" />
          <span>{t.importacion.rellenarVacios}</span>
        </label>
      </fieldset>

      <label class="im__check">
        <input type="checkbox" bind:checked={crearFaltantes} />
        <span>{t.importacion.crearFaltantes}</span>
      </label>
    </div>
  {:else if fase === "ejecutando"}
    <div class="im">
      <div class="im__progreso">
        <div class="im__barra">
          <div class="im__relleno" style="width:{pct}%"></div>
        </div>
        <span class="im__faselbl">
          {faseLbl}
          {#if progreso && progreso.total > 0}
            · {progreso.hechas}/{progreso.total}
          {/if}
        </span>
      </div>
    </div>
  {:else if fase === "resumen" && resumen}
    <div class="im">
      <span class="im__etq">{t.importacion.resumenTitulo}</span>
      <p class="im__cuenta">
        {resumen.actualizados}
        {t.importacion.seActualizaran} ·
        {resumen.creados}
        {t.importacion.seCrearan} ·
        {resumen.sinCambio}
        {t.importacion.sinCambio}
      </p>
      {#if resumen.avisos.length > 0}
        <div class="im__avisos">
          <span class="im__etq">{t.importacion.avisos}</span>
          <ul>
            {#each resumen.avisos as a (a)}
              <li>{a}</li>
            {/each}
          </ul>
        </div>
      {/if}
      {#if resumen.errores.length > 0}
        <div class="im__avisos im__avisos--error">
          <span class="im__etq">{t.importacion.errores}</span>
          <ul>
            {#each resumen.errores as e (e)}
              <li>{e}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {:else if fase === "resultado" && resultado}
    <div class="im">
      <span class="im__etq">{t.importacion.resultadoTitulo}</span>
      <p class="im__cuenta">
        {resultado.actualizados}
        {t.importacion.actualizados} ·
        {resultado.creados}
        {t.importacion.creados} ·
        {resultado.sinCambio}
        {t.importacion.sinCambioRes}
      </p>
      {#if resultado.avisos.length > 0}
        <div class="im__avisos">
          <span class="im__etq">{t.importacion.avisos}</span>
          <ul>
            {#each resultado.avisos as a (a)}
              <li>{a}</li>
            {/each}
          </ul>
        </div>
      {/if}
      {#if resultado.errores.length > 0}
        <div class="im__avisos im__avisos--error">
          <span class="im__etq">{t.importacion.errores}</span>
          <ul>
            {#each resultado.errores as e (e)}
              <li>{e}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/if}

  {#snippet pie()}
    {#if fase === "configurar"}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
      <Button
        variante="primario"
        onclick={analizar}
        disabled={campoLlave.trim() === ""}
      >
        {t.importacion.verResumen}
        <ArrowRight size={16} />
      </Button>
    {:else if fase === "resumen"}
      <Button variante="fantasma" onclick={() => (fase = "configurar")}>
        {t.importacion.volver}
      </Button>
      <Button variante="primario" onclick={aplicar}>
        {t.importacion.aplicar}
      </Button>
    {:else if fase === "resultado"}
      <Button variante="primario" onclick={cerrar}>{t.accion.cerrar}</Button>
    {:else}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    {/if}
  {/snippet}
</Modal>

<style>
  .im {
    display: flex;
    flex-direction: column;
    gap: var(--esp-4);
  }
  .im__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .im__grupo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .im__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .im__ruta {
    display: flex;
    gap: var(--esp-2);
  }
  .im__ruta :global(.campo) {
    flex: 1;
  }
  .im__nota {
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .im__resumen {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--esp-3);
  }
  .im__dato {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    padding: var(--esp-2);
    border-radius: var(--radio);
    background: var(--color-panel);
  }
  .im__rotulo {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .im__valor {
    font-size: var(--tam-fuente-lg);
    font-weight: 600;
  }
  .im__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
    max-height: 120px;
    overflow-y: auto;
  }
  .im__chip {
    font-size: var(--tam-fuente-xs);
    padding: 2px var(--esp-2);
    border-radius: var(--radio-pill);
    background: var(--color-acento-tenue);
    color: var(--color-acento);
  }
  .im__chip--ignorada {
    background: var(--color-panel);
    color: var(--color-texto-secundario);
    text-decoration: line-through;
  }
  .im__radios {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    margin: 0;
    padding: 0;
    border: none;
  }
  .im__radio {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
  }
  .im__check {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
  }
  .im__progreso {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding: var(--esp-4) 0;
  }
  .im__barra {
    height: 8px;
    border-radius: var(--radio-pill);
    background: var(--color-panel);
    overflow: hidden;
  }
  .im__relleno {
    height: 100%;
    background: var(--color-acento);
    transition: width var(--transicion);
  }
  .im__faselbl {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
    text-align: center;
  }
  .im__cuenta {
    margin: 0;
    padding: var(--esp-2) 0;
    font-size: var(--tam-fuente-lg);
    text-align: center;
  }
  .im__avisos {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-sm);
    color: var(--color-aviso);
  }
  .im__avisos--error {
    color: var(--color-peligro);
  }
  .im__avisos ul {
    margin: 0;
    padding-left: var(--esp-4);
  }
</style>
