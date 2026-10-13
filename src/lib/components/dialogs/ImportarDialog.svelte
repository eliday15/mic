<!--
  ImportarDialog — importa registros desde un archivo CSV o XLSX al álbum activo
  (ex "Importar..." del VB6, Module3.bas ActualizaConImportado). Híbrido de
  MigrateDialog (file picker + inspección con chips) y LinkedAlbumsDialog (barra
  de progreso + resultado). Flujo en fases:
    elegir → configurar → ejecutando(analizando) → resumen → ejecutando(aplicando) → resultado.
  La "llave de importación" es un campo del álbum ↔ una columna del archivo. El
  análisis previo (dry-run) dice, sobre esa llave, cuántas filas coinciden y
  lista como ERROR las que no casan con ningún registro (más llaves vacías o
  repetidas); si hay filas sin coincidencia, aplicar exige elegir entre omitirlas
  o importarlas como registros nuevos. Las columnas extra (sin campo en el
  álbum) se ignoran salvo que se marque crearlas como campos. La huella de la
  inspección se pasa al aplicar para que el backend valide que el archivo no
  cambió. La grilla se refresca al cerrar.
-->
<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    FolderOpen,
    ArrowRight,
    CircleAlert,
    CircleCheck,
    TriangleAlert,
  } from "lucide-svelte";
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
  import { camposListar } from "$lib/ipc/commands";
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
  /** Qué hacer con las filas cuya llave no coincide ("" = aún sin decidir). */
  type DecisionNoCoinciden = "" | "omitir" | "crear";

  let fase = $state<Fase>("elegir");
  let rutaArchivo = $state("");
  let inspeccion = $state<InspeccionImport | null>(null);
  let campoLlave = $state("");
  let columnaLlave = $state("");
  let politica = $state<PoliticaImport>("sustituir");
  let importarExtras = $state(false);
  let decision = $state<DecisionNoCoinciden>("");
  let resumen = $state<ResultadoImportacion | null>(null); // dry-run
  let resultado = $state<ResultadoImportacion | null>(null); // apply
  let progreso = $state<ImportacionProgreso | null>(null);
  let unlisten: UnlistenFn | null = null;

  const igual = (a: string, b: string): boolean =>
    a.trim().toLowerCase() === b.trim().toLowerCase();

  // Llave: cualquier campo elegible del álbum ↔ cualquier columna del archivo.
  const opcionesCampo = $derived(
    (inspeccion?.camposLlaveAlbum ?? []).map((n) => ({ valor: n, etiqueta: n })),
  );
  const opcionesColumna = $derived(
    (inspeccion?.columnas ?? [])
      .filter((c) => c.trim() !== "")
      .map((c) => ({ valor: c, etiqueta: c })),
  );

  // Extras sin la columna elegida como llave (esa casa, no se importa).
  const extras = $derived(
    (inspeccion?.columnasExtra ?? []).filter((e) => !igual(e.columna, columnaLlave)),
  );
  // No reconocidas que tampoco pueden crearse (homónimas de variantes, sin nombre…).
  const ignoradas = $derived(
    (inspeccion?.columnasNoReconocidas ?? []).filter(
      (c) =>
        !igual(c, columnaLlave) &&
        !(inspeccion?.columnasExtra ?? []).some((e) => igual(e.columna, c)),
    ),
  );

  const hayNoCoinciden = $derived((resumen?.noCoincidentes.total ?? 0) > 0);
  const puedeAplicar = $derived(!hayNoCoinciden || decision !== "");

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
      const insp = await importarInspeccionar(estado.albumId, rutaArchivo);
      inspeccion = insp;
      // Llave por defecto: la primera columna si casa con un campo elegible;
      // si no, la primera columna sugerida; si no, primer campo ↔ primera columna.
      const campoDe = (col: string): string | undefined =>
        insp.camposLlaveAlbum.find((c) => igual(c, col));
      const columna = [insp.columnas[0] ?? "", ...insp.camposLlaveSugeridos].find(
        (c) => campoDe(c) !== undefined,
      );
      columnaLlave = columna ?? insp.columnas[0] ?? "";
      campoLlave = (columna && campoDe(columna)) || (insp.camposLlaveAlbum[0] ?? "");
      importarExtras = false;
      fase = "configurar";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.importacion);
    }
  }

  /** Al cambiar el campo llave, propone la columna homónima si existe. */
  function alCambiarCampo(campo: string): void {
    const homonima = inspeccion?.columnas.find((c) => igual(c, campo));
    if (homonima) columnaLlave = homonima;
  }

  /** Ejecuta `importar_registros` mostrando la barra de progreso. */
  async function ejecutar(dryRun: boolean): Promise<ResultadoImportacion> {
    fase = "ejecutando";
    progreso = null;
    unlisten = await escucharImportacionProgreso((p) => (progreso = p));
    try {
      return await importarRegistros(estado.albumId, rutaArchivo, {
        campoLlave,
        columnaLlave,
        politica,
        crearFaltantes: !dryRun && decision === "crear",
        importarExtras,
        dryRun,
        huella: inspeccion?.huella,
      });
    } finally {
      unlisten?.();
      unlisten = null;
    }
  }

  /** Análisis en seco (dry-run): clasifica sin escribir y muestra el resumen. */
  async function analizar(): Promise<void> {
    resumen = null;
    decision = "";
    try {
      resumen = await ejecutar(true);
      fase = "resumen";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.importacion);
      fase = "configurar";
    }
  }

  /** Aplica la importación de verdad y refresca la grilla. */
  async function aplicar(): Promise<void> {
    if (!puedeAplicar) return;
    resultado = null;
    try {
      resultado = await ejecutar(false);
      // Campos nuevos (columnas extra): la vista debe ver la estructura vigente.
      if (resultado.camposCreados.length > 0) {
        estado.setCampos(await camposListar(estado.albumId));
      }
      fase = "resultado";
      onAplicado?.();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.importacion);
      fase = "resumen";
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

{#snippet tablaFilas(filas: { fila: number; llave: string }[], total: number)}
  <div class="im__tabla" role="region" aria-label={t.importacion.colLlave}>
    <table>
      <thead>
        <tr><th>{t.importacion.colFila}</th><th>{t.importacion.colLlave}</th></tr>
      </thead>
      <tbody>
        {#each filas as f (f.fila)}
          <tr><td class="tabular">{f.fila}</td><td>{f.llave}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if total > filas.length}
    <span class="im__nota">{t.importacion.mostrando(filas.length, total)}</span>
  {/if}
{/snippet}

{#snippet listas(r: ResultadoImportacion)}
  {#if r.avisos.length > 0}
    <div class="im__avisos">
      <span class="im__etq">{t.importacion.avisos}</span>
      <ul>
        {#each r.avisos as a (a)}
          <li>{a}</li>
        {/each}
      </ul>
    </div>
  {/if}
  {#if r.errores.length > 0}
    <div class="im__avisos im__avisos--error">
      <span class="im__etq">{t.importacion.errores}</span>
      <ul>
        {#each r.errores as e (e)}
          <li>{e}</li>
        {/each}
      </ul>
    </div>
  {/if}
{/snippet}

<!-- Sin cierre por Escape ni clic fuera mientras la operación corre. -->
<Modal
  bind:abierto
  titulo={t.importacion.titulo}
  ancho={fase === "resumen" ? "lg" : "md"}
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

      <fieldset class="im__llave">
        <legend class="im__etq">{t.importacion.llaveTitulo}</legend>
        <div class="im__llave-fila">
          <label class="im__campo">
            <span class="im__rotulo">{t.importacion.llaveCampo}</span>
            <Select
              bind:value={campoLlave}
              opciones={opcionesCampo}
              etiqueta={t.importacion.llaveCampo}
              onCambio={alCambiarCampo}
            />
          </label>
          <span class="im__flecha" aria-hidden="true">↔</span>
          <label class="im__campo">
            <span class="im__rotulo">{t.importacion.llaveColumna}</span>
            <Select
              bind:value={columnaLlave}
              opciones={opcionesColumna}
              etiqueta={t.importacion.llaveColumna}
            />
          </label>
        </div>
        <span class="im__nota">{t.importacion.llaveNota}</span>
      </fieldset>

      <div class="im__grupo">
        <span class="im__etq">{t.importacion.columnasReconocidas}</span>
        <div class="im__chips">
          {#each inspeccion.columnasReconocidas as col (col)}
            <span class="im__chip">{col}</span>
          {/each}
        </div>
      </div>

      {#if extras.length > 0}
        <div class="im__grupo im__extras">
          <span class="im__etq">{t.importacion.extrasTitulo}</span>
          <div class="im__chips">
            {#each extras as e (e.columna)}
              <span class="im__chip" class:im__chip--ignorada={!importarExtras}>
                {e.columna}
                <span class="im__tipo">{t.campos.tipos[e.tipo]}</span>
              </span>
            {/each}
          </div>
          <label class="im__check">
            <input type="checkbox" bind:checked={importarExtras} />
            <span>{t.importacion.importarExtras}</span>
          </label>
          <span class="im__nota">{t.importacion.extrasNota}</span>
        </div>
      {/if}

      {#if ignoradas.length > 0}
        <div class="im__grupo">
          <span class="im__etq">{t.importacion.columnasIgnoradas}</span>
          <div class="im__chips">
            {#each ignoradas as col (col)}
              <span class="im__chip im__chip--ignorada">{col}</span>
            {/each}
          </div>
        </div>
      {/if}

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
      <p class="im__nota">{t.importacion.llaveResumen(campoLlave, columnaLlave)}</p>

      <div class="im__estado im__estado--ok">
        <CircleCheck size={16} />
        <span>{t.importacion.coinciden(resumen.coincidentes)}</span>
      </div>
      <p class="im__cuenta">
        {resumen.actualizados}
        {t.importacion.seActualizaran} ·
        {resumen.sinCambio}
        {t.importacion.sinCambio}
        {#if decision === "crear"}
          · {resumen.noCoincidentes.total} {t.importacion.seCrearan}
        {/if}
      </p>

      {#if hayNoCoinciden}
        <div class="im__panel im__panel--error" role="alert">
          <div class="im__estado">
            <CircleAlert size={16} />
            <strong>{t.importacion.noCoinciden(resumen.noCoincidentes.total)}</strong>
          </div>
          {@render tablaFilas(resumen.noCoincidentes.filas, resumen.noCoincidentes.total)}
        </div>
      {/if}

      {#if resumen.llavesVacias.total > 0}
        <div class="im__panel im__panel--error">
          <div class="im__estado">
            <CircleAlert size={16} />
            <strong>{t.importacion.llavesVacias(resumen.llavesVacias.total)}</strong>
          </div>
          <span class="im__nota tabular">
            {t.importacion.colFila}: {resumen.llavesVacias.filas.map((f) => f.fila).join(", ")}
            {#if resumen.llavesVacias.total > resumen.llavesVacias.filas.length}…{/if}
          </span>
        </div>
      {/if}

      {#if resumen.llavesDuplicadas.total > 0}
        <div class="im__panel im__panel--aviso">
          <div class="im__estado">
            <TriangleAlert size={16} />
            <strong>{t.importacion.llavesDuplicadas(resumen.llavesDuplicadas.total)}</strong>
          </div>
          {@render tablaFilas(resumen.llavesDuplicadas.filas, resumen.llavesDuplicadas.total)}
        </div>
      {/if}

      {#if resumen.camposCreados.length > 0}
        <div class="im__grupo">
          <span class="im__etq">{t.importacion.camposACrear}</span>
          <div class="im__chips">
            {#each resumen.columnasExtra as e (e.columna)}
              <span class="im__chip">
                {e.columna}
                <span class="im__tipo">{t.campos.tipos[e.tipo]}</span>
              </span>
            {/each}
          </div>
        </div>
      {/if}

      {@render listas(resumen)}

      {#if hayNoCoinciden}
        <fieldset class="im__radios im__confirmar">
          <legend class="im__confirmar-titulo">
            {t.importacion.confirmarNoCoinciden(resumen.noCoincidentes.total)}
          </legend>
          <label class="im__radio">
            <input type="radio" bind:group={decision} value="omitir" />
            <span>{t.importacion.omitirNoCoinciden}</span>
          </label>
          <label class="im__radio">
            <input type="radio" bind:group={decision} value="crear" />
            <span>{t.importacion.crearNoCoinciden}</span>
          </label>
          {#if decision === ""}
            <span class="im__nota">{t.importacion.elegirOpcion}</span>
          {/if}
        </fieldset>
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
        {resultado.omitidos}
        {t.importacion.omitidos} ·
        {resultado.sinCambio}
        {t.importacion.sinCambioRes}
      </p>
      {#if resultado.camposCreados.length > 0}
        <div class="im__grupo">
          <span class="im__etq">{t.importacion.camposCreados}</span>
          <div class="im__chips">
            {#each resultado.camposCreados as nombre (nombre)}
              <span class="im__chip">{nombre}</span>
            {/each}
          </div>
        </div>
      {/if}
      {@render listas(resultado)}
    </div>
  {/if}

  {#snippet pie()}
    {#if fase === "configurar"}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
      <Button
        variante="primario"
        onclick={analizar}
        disabled={campoLlave.trim() === "" || columnaLlave.trim() === ""}
      >
        {t.importacion.verResumen}
        <ArrowRight size={16} />
      </Button>
    {:else if fase === "resumen"}
      <Button variante="fantasma" onclick={() => (fase = "configurar")}>
        {t.importacion.volver}
      </Button>
      <Button variante="primario" onclick={aplicar} disabled={!puedeAplicar}>
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
  .im__llave {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    margin: 0;
    padding: var(--esp-3);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
  }
  .im__llave-fila {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    gap: var(--esp-2);
  }
  .im__flecha {
    padding-bottom: var(--esp-2);
    color: var(--color-texto-secundario);
  }
  .im__tipo {
    margin-left: var(--esp-1);
    opacity: 0.7;
  }
  .im__estado {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
  }
  .im__estado--ok {
    color: var(--color-exito);
  }
  .im__panel {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding: var(--esp-2) var(--esp-3);
    border-radius: var(--radio);
  }
  .im__panel--error {
    color: var(--color-peligro);
    background: var(--color-peligro-tenue);
  }
  .im__panel--aviso {
    color: var(--color-aviso);
    background: var(--color-aviso-tenue);
  }
  .im__tabla {
    max-height: 180px;
    overflow-y: auto;
    border-radius: var(--radio);
    background: var(--color-fondo);
    color: var(--color-texto);
  }
  .im__tabla table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--tam-fuente-sm);
  }
  .im__tabla th {
    position: sticky;
    top: 0;
    text-align: left;
    padding: var(--esp-1) var(--esp-2);
    background: var(--color-panel);
    font-weight: 600;
  }
  .im__tabla td {
    padding: 2px var(--esp-2);
    border-top: 1px solid var(--color-borde);
  }
  .im__tabla th:first-child,
  .im__tabla td:first-child {
    width: 4.5em;
    text-align: right;
  }
  .im__confirmar {
    padding: var(--esp-3);
    border: 1px solid var(--color-peligro);
    border-radius: var(--radio);
  }
  .im__confirmar-titulo {
    padding: 0 var(--esp-1);
    font-weight: 600;
    font-size: var(--tam-fuente-sm);
    color: var(--color-peligro);
  }
</style>
