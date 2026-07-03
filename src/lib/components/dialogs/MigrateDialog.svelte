<!--
  MigrateDialog — asistente de migración de un .mdb de Access a un álbum .micdb.
  Flujo: elegir .mdb → inspeccionar → confirmar destino → ejecutar con barra de
  progreso (evento `migracion-progreso`) → abrir el álbum resultante.
-->
<script lang="ts">
  import { FolderOpen, Database, ArrowRight } from "lucide-svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    Modal,
    Button,
    TextInput,
    Spinner,
    EmptyState,
  } from "$lib/components/ui";
  import {
    migracionVerificarMdbtools,
    migracionInspeccionar,
    migracionEjecutar,
  } from "$lib/ipc/commands";
  import { escucharMigracionProgreso } from "$lib/ipc/events";
  import type { UnlistenFn } from "$lib/ipc/events";
  import { albumes } from "$lib/stores/albums.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type {
    MdbInspeccion,
    MigracionProgreso,
    MigracionReporte,
  } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), onCerrar }: Props = $props();

  type Fase = "verificando" | "elegir" | "inspeccion" | "ejecutando" | "reporte";

  let fase = $state<Fase>("verificando");
  let herramientasOk = $state(false);
  let rutaMdb = $state("");
  let rutaDestino = $state("");
  let inspeccion = $state<MdbInspeccion | null>(null);
  let progreso = $state<MigracionProgreso | null>(null);
  let reporte = $state<MigracionReporte | null>(null);
  let cargando = $state(false);
  /// Último error, mostrado DENTRO del diálogo (el toast desaparece solo; esto
  /// no: la lección de los cuelgues "sin error" de v3.0.1–v3.0.4).
  let errorMsg = $state<string | null>(null);
  let version = $state("");
  let unlisten: UnlistenFn | null = null;

  $effect(() => {
    if (abierto) {
      verificar();
      // Escucha los pasos/progreso durante TODO el ciclo del diálogo: la
      // inspección también emite pasos en vivo (vigía del backend).
      void escucharMigracionProgreso((p) => (progreso = p)).then((un) => {
        unlisten = un;
      });
      // Versión visible para diagnóstico remoto (en navegador/mock no existe).
      import("@tauri-apps/api/app")
        .then((m) => m.getVersion())
        .then((v) => (version = v))
        .catch(() => (version = "dev"));
    }
    return () => {
      unlisten?.();
      unlisten = null;
    };
  });

  async function verificar(): Promise<void> {
    fase = "verificando";
    try {
      herramientasOk = await migracionVerificarMdbtools();
      fase = "elegir";
    } catch {
      herramientasOk = false;
      fase = "elegir";
    }
  }

  async function elegirMdb(): Promise<void> {
    const sel = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Access", extensions: ["mdb"] }],
    });
    if (typeof sel === "string") {
      rutaMdb = sel;
      await inspeccionar();
    }
  }

  async function inspeccionar(): Promise<void> {
    cargando = true;
    errorMsg = null;
    progreso = null;
    try {
      inspeccion = await migracionInspeccionar(rutaMdb);
      fase = "inspeccion";
    } catch (e) {
      errorMsg = typeof e === "string" ? e : t.error.migracion;
      ui.error(errorMsg);
    } finally {
      cargando = false;
    }
  }

  async function elegirDestino(): Promise<void> {
    const sel = await save({
      title: t.migracion.destino,
      defaultPath: "album.micdb",
      filters: [{ name: "MIC", extensions: ["micdb"] }],
    });
    if (typeof sel === "string") rutaDestino = sel;
  }

  async function ejecutar(): Promise<void> {
    if (rutaDestino.trim() === "") {
      ui.aviso(t.migracion.destino);
      return;
    }
    fase = "ejecutando";
    progreso = null;
    errorMsg = null;
    try {
      reporte = await migracionEjecutar(rutaMdb, rutaDestino.trim());
      fase = "reporte";
      ui.exito(t.mensaje.migracionOk);
    } catch (e) {
      errorMsg = typeof e === "string" ? e : t.error.migracion;
      ui.error(errorMsg);
      fase = "inspeccion";
    }
  }

  async function abrirResultado(): Promise<void> {
    try {
      await albumes.abrir(rutaDestino.trim());
      cerrar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.cargarAlbum);
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }

  const pct = $derived(
    progreso && progreso.total > 0
      ? Math.min(100, Math.round((progreso.hechas / progreso.total) * 100))
      : 0,
  );
</script>

<Modal bind:abierto titulo={t.migracion.titulo} ancho="md" onCerrar={cerrar}>
  {#if fase === "verificando"}
    <div class="mg__centro">
      <Spinner tamano={24} />
      <span>{t.migracion.verificandoHerramientas}</span>
    </div>
  {:else if !herramientasOk}
    <EmptyState
      titulo={t.migracion.faltanHerramientas}
      descripcion={t.migracion.faltanHerramientasDesc}
    >
      {#snippet icono()}
        <Database size={28} />
      {/snippet}
    </EmptyState>
  {:else if fase === "elegir"}
    <div class="mg">
      <label class="mg__campo">
        <span class="mg__etq">{t.migracion.archivoMdb}</span>
        <div class="mg__ruta">
          <TextInput bind:value={rutaMdb} readonly placeholder="…/album.mdb" />
          <Button variante="secundario" cargando={cargando} onclick={elegirMdb}>
            <FolderOpen size={16} />
            {t.accion.examinar}
          </Button>
        </div>
      </label>

      {#if cargando}
        <!-- Paso EN VIVO del backend: si algo se atora, aquí dice exactamente dónde. -->
        <div class="mg__paso">
          <Spinner tamano={14} />
          <span>{progreso?.fase ?? t.migracion.inspeccionando}</span>
        </div>
      {/if}

      {#if errorMsg}
        <div class="mg__error" role="alert">{errorMsg}</div>
      {/if}
    </div>
  {:else if fase === "inspeccion" && inspeccion}
    <div class="mg">
      {#if errorMsg}
        <div class="mg__error" role="alert">{errorMsg}</div>
      {/if}
      <div class="mg__resumen">
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.inspeccion.totalEstimado}</span>
          <span class="mg__valor tabular">{inspeccion.totalEstimado}</span>
        </div>
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.inspeccion.tieneVariantes}</span>
          <span class="mg__valor">{inspeccion.tieneVariantes ? t.accion.si : t.accion.no}</span>
        </div>
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.inspeccion.campos}</span>
          <span class="mg__valor tabular">{inspeccion.campos.length}</span>
        </div>
      </div>

      <div class="mg__campos">
        {#each inspeccion.campos as c (c.nombre)}
          <span class="mg__chip">{c.nombre} · {c.tipo}</span>
        {/each}
      </div>

      <label class="mg__campo">
        <span class="mg__etq">{t.migracion.destino}</span>
        <div class="mg__ruta">
          <TextInput bind:value={rutaDestino} readonly placeholder="…/album.micdb" />
          <Button variante="secundario" onclick={elegirDestino}>
            <FolderOpen size={16} />
            {t.accion.examinar}
          </Button>
        </div>
      </label>
    </div>
  {:else if fase === "ejecutando"}
    <div class="mg">
      <div class="mg__progreso">
        <div class="mg__barra">
          <div class="mg__relleno" style="width:{pct}%"></div>
        </div>
        <span class="mg__faselbl">
          {progreso?.fase ?? t.migracion.progreso.preparando}
          {#if progreso && progreso.total > 0}
            · {progreso.hechas}/{progreso.total}
          {/if}
        </span>
      </div>
    </div>
  {:else if fase === "reporte" && reporte}
    <div class="mg">
      <div class="mg__resumen mg__resumen--rep">
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.reporte.filasPrincipal}</span>
          <span class="mg__valor tabular">{reporte.filasPrincipal}</span>
        </div>
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.reporte.filasVariantes}</span>
          <span class="mg__valor tabular">{reporte.filasVariantes}</span>
        </div>
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.reporte.imagenesEncontradas}</span>
          <span class="mg__valor tabular">{reporte.imagenesEncontradas}</span>
        </div>
        <div class="mg__dato">
          <span class="mg__rotulo">{t.migracion.reporte.imagenesFaltantes}</span>
          <span class="mg__valor tabular">{reporte.imagenesFaltantes.length}</span>
        </div>
      </div>
      {#if reporte.advertencias.length > 0}
        <div class="mg__avisos">
          <span class="mg__etq">{t.migracion.reporte.advertencias}</span>
          <ul>
            {#each reporte.advertencias as a (a)}
              <li>{a}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/if}

  {#snippet pie()}
    <!-- Versión visible: para diagnóstico remoto por captura de pantalla. -->
    <span class="mg__version">v{version}</span>
    {#if fase === "inspeccion"}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
      <Button variante="primario" onclick={ejecutar}>
        {t.migracion.ejecutar}
        <ArrowRight size={16} />
      </Button>
    {:else if fase === "reporte"}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cerrar}</Button>
      <Button variante="primario" onclick={abrirResultado}>{t.accion.abrir}</Button>
    {:else}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    {/if}
  {/snippet}
</Modal>

<style>
  .mg {
    display: flex;
    flex-direction: column;
    gap: var(--esp-4);
  }
  .mg__centro {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--esp-3);
    padding: var(--esp-6);
    color: var(--color-texto-secundario);
  }
  .mg__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .mg__paso {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .mg__error {
    padding: var(--esp-3);
    border: 1px solid var(--color-peligro, #b91c1c);
    border-radius: var(--radio-md);
    background: color-mix(in srgb, var(--color-peligro, #b91c1c) 10%, transparent);
    color: var(--color-peligro, #b91c1c);
    font-size: var(--tam-fuente-sm);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .mg__version {
    margin-right: auto;
    font-size: var(--tam-fuente-xs, 11px);
    color: var(--color-texto-secundario);
    opacity: 0.7;
  }
  .mg__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .mg__ruta {
    display: flex;
    gap: var(--esp-2);
  }
  .mg__ruta :global(.campo) {
    flex: 1;
  }
  .mg__resumen {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--esp-3);
  }
  .mg__resumen--rep {
    grid-template-columns: repeat(2, 1fr);
  }
  .mg__dato {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    padding: var(--esp-2);
    border-radius: var(--radio);
    background: var(--color-panel);
  }
  .mg__rotulo {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .mg__valor {
    font-size: var(--tam-fuente-lg);
    font-weight: 600;
  }
  .mg__campos {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
    max-height: 120px;
    overflow-y: auto;
  }
  .mg__chip {
    font-size: var(--tam-fuente-xs);
    padding: 2px var(--esp-2);
    border-radius: var(--radio-pill);
    background: var(--color-panel);
    color: var(--color-texto-secundario);
  }
  .mg__progreso {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding: var(--esp-4) 0;
  }
  .mg__barra {
    height: 8px;
    border-radius: var(--radio-pill);
    background: var(--color-panel);
    overflow: hidden;
  }
  .mg__relleno {
    height: 100%;
    background: var(--color-acento);
    transition: width var(--transicion);
  }
  .mg__faselbl {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
    text-align: center;
  }
  .mg__avisos {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-sm);
    color: var(--color-aviso);
  }
  .mg__avisos ul {
    margin: 0;
    padding-left: var(--esp-4);
  }
</style>
