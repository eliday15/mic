<!--
  LinkedAlbumsDialog — gestiona los álbumes ligados (ex-frmAlbumsL/frmEdligado/
  frmstligas del VB6). Lista las ligas, permite crear/editar/eliminar y
  actualizar (una o todas) sincronizando datos desde otro álbum .micdb mediante
  un campo llave común. El formulario vive en LigaForm. Durante la
  actualización muestra una barra de progreso (evento liga-progreso) y el
  resultado final (incluidas fotos importadas y campos creados).
-->
<script lang="ts">
  import { Plus, Pencil, Trash2, RefreshCw, Link2 } from "lucide-svelte";
  import { Modal, Button, Chip, EmptyState } from "$lib/components/ui";
  import LigaForm from "./LigaForm.svelte";
  import {
    ligadosListar,
    ligaGuardar,
    ligaEliminar,
    ligaActualizar,
    ligasActualizarTodas,
    escucharLigaProgreso,
    type FuenteFotos,
    type Liga,
    type ResultadoLiga,
    type LigaProgreso,
  } from "./ligadosIpc";
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

  type Fase = "lista" | "formulario" | "ejecutando" | "resultado";

  let fase = $state<Fase>("lista");
  let ligas = $state<Liga[]>([]);

  // Formulario de liga (nueva o edición).
  let editId = $state(0);
  let rutaAlbum = $state("");
  let llave = $state("");
  let crearFaltantes = $state(false);
  let importarCamposNuevos = $state(false);
  let fotos = $state<FuenteFotos>("actual");

  // Progreso / resultado de la actualización.
  let progreso = $state<LigaProgreso | null>(null);
  let resultado = $state<ResultadoLiga | null>(null);
  let unlisten: UnlistenFn | null = null;

  $effect(() => {
    if (abierto) void recargar();
    return () => {
      unlisten?.();
      unlisten = null;
    };
  });

  async function recargar(): Promise<void> {
    try {
      ligas = await ligadosListar(estado.albumId);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  function nombreArchivo(ruta: string): string {
    const base = ruta.split(/[/\\]/).pop() ?? ruta;
    return base.replace(/\.micdb$/i, "");
  }

  function nuevaLiga(): void {
    editId = 0;
    rutaAlbum = "";
    llave =
      estado.campos.find(
        (c) =>
          c.tabla === "principal" &&
          c.tipo !== "calculado" &&
          c.tipo !== "multidato",
      )?.nombre ?? "";
    crearFaltantes = false;
    importarCamposNuevos = false;
    fotos = "actual";
    fase = "formulario";
  }

  function editarLiga(liga: Liga): void {
    editId = liga.id;
    rutaAlbum = liga.rutaAlbum;
    llave = liga.llave;
    crearFaltantes = liga.crearFaltantes;
    importarCamposNuevos = liga.importarCamposNuevos;
    fotos = liga.fotos;
    fase = "formulario";
  }

  async function guardar(): Promise<void> {
    if (rutaAlbum.trim() === "" || llave.trim() === "") {
      ui.aviso(t.ligados.descripcion);
      return;
    }
    try {
      await ligaGuardar(estado.albumId, {
        id: editId,
        rutaAlbum: rutaAlbum.trim(),
        llave,
        crearFaltantes,
        importarCamposNuevos,
        fotos,
      });
      await recargar();
      fase = "lista";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  async function eliminar(liga: Liga): Promise<void> {
    try {
      await ligaEliminar(estado.albumId, liga.id);
      await recargar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  async function actualizar(liga: Liga): Promise<void> {
    await ejecutar(() => ligaActualizar(estado.albumId, liga.id));
  }

  async function actualizarTodas(): Promise<void> {
    await ejecutar(async () => {
      const lista = await ligasActualizarTodas(estado.albumId);
      // Combina los resultados de todas las ligas en uno solo.
      return lista.reduce<ResultadoLiga>(
        (acc, r) => ({
          actualizados: acc.actualizados + r.actualizados,
          creados: acc.creados + r.creados,
          sinCoincidencia: acc.sinCoincidencia + r.sinCoincidencia,
          fotosCopiadas: acc.fotosCopiadas + r.fotosCopiadas,
          camposCreados: [...acc.camposCreados, ...r.camposCreados],
        }),
        {
          actualizados: 0,
          creados: 0,
          sinCoincidencia: 0,
          fotosCopiadas: 0,
          camposCreados: [],
        },
      );
    });
  }

  async function ejecutar(
    fn: () => Promise<ResultadoLiga>,
  ): Promise<void> {
    fase = "ejecutando";
    progreso = null;
    resultado = null;
    unlisten = await escucharLigaProgreso((p) => (progreso = p));
    try {
      resultado = await fn();
      // Campos creados por la liga: la grilla/inspector deben verlos ya.
      if (resultado.camposCreados.length > 0) {
        estado.setCampos(await camposListar(estado.albumId));
      }
      fase = "resultado";
      onAplicado?.();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
      fase = "lista";
    } finally {
      unlisten?.();
      unlisten = null;
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

<Modal bind:abierto titulo={t.ligados.titulo} ancho="md" onCerrar={cerrar}>
  {#if fase === "lista"}
    <div class="lg">
      <p class="lg__desc">{t.ligados.descripcion}</p>
      {#if ligas.length === 0}
        <EmptyState titulo={t.ligados.sinLigas}>
          {#snippet icono()}
            <Link2 size={28} />
          {/snippet}
        </EmptyState>
      {:else}
        <ul class="lg__lista">
          {#each ligas as liga (liga.id)}
            <li class="lg__item">
              <div class="lg__info">
                <span class="lg__album">{nombreArchivo(liga.rutaAlbum)}</span>
                <span class="lg__llave">{t.ligados.llave}: {liga.llave}</span>
                <div class="lg__badges">
                  {#if liga.crearFaltantes}
                    <span class="lg__badge">{t.ligados.badgeFaltantes}</span>
                  {/if}
                  {#if liga.importarCamposNuevos}
                    <span class="lg__badge">{t.ligados.badgeCampos}</span>
                  {/if}
                  {#if liga.fotos === "ligado"}
                    <span class="lg__badge">{t.ligados.badgeFotosLigado}</span>
                  {/if}
                </div>
              </div>
              <div class="lg__acciones">
                <Button
                  variante="secundario"
                  tamano="sm"
                  onclick={() => actualizar(liga)}
                  title={t.ligados.actualizar}
                >
                  <RefreshCw size={14} />
                </Button>
                <Button
                  variante="fantasma"
                  tamano="sm"
                  onclick={() => editarLiga(liga)}
                  title={t.ligados.editarLiga}
                >
                  <Pencil size={14} />
                </Button>
                <Button
                  variante="fantasma"
                  tamano="sm"
                  onclick={() => eliminar(liga)}
                  title={t.ligados.eliminarLiga}
                >
                  <Trash2 size={14} />
                </Button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else if fase === "formulario"}
    <LigaForm
      {estado}
      bind:rutaAlbum
      bind:llave
      bind:crearFaltantes
      bind:importarCamposNuevos
      bind:fotos
    />
  {:else if fase === "ejecutando"}
    <div class="lg">
      <div class="lg__progreso">
        <div class="lg__barra">
          <div class="lg__relleno" style="width:{pct}%"></div>
        </div>
        <span class="lg__faselbl">
          {t.ligados.progreso}
          {#if progreso && progreso.total > 0}
            · {progreso.hechas}/{progreso.total}
          {/if}
        </span>
      </div>
    </div>
  {:else if fase === "resultado" && resultado}
    <div class="lg">
      <dl class="lg__cifras">
        <div><dt>{resultado.actualizados}</dt><dd>{t.ligados.resultado.actualizados}</dd></div>
        <div><dt>{resultado.creados}</dt><dd>{t.ligados.resultado.creados}</dd></div>
        <div><dt>{resultado.fotosCopiadas}</dt><dd>{t.ligados.resultado.fotosCopiadas}</dd></div>
        <div><dt>{resultado.sinCoincidencia}</dt><dd>{t.ligados.resultado.sinCoincidencia}</dd></div>
      </dl>
      {#if resultado.camposCreados.length > 0}
        <div class="lg__creados">
          <span class="lg__etq">{t.ligados.resultado.camposCreados}</span>
          <div class="lg__chips">
            {#each resultado.camposCreados as nombre, i (i)}
              <Chip variante="acento">{nombre}</Chip>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#snippet pie()}
    {#if fase === "lista"}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cerrar}</Button>
      {#if ligas.length > 0}
        <Button variante="secundario" onclick={actualizarTodas}>
          <RefreshCw size={16} />
          {t.ligados.actualizarTodas}
        </Button>
      {/if}
      <Button variante="primario" onclick={nuevaLiga}>
        <Plus size={16} />
        {t.ligados.nuevo}
      </Button>
    {:else if fase === "formulario"}
      <Button variante="fantasma" onclick={() => (fase = "lista")}>
        {t.accion.cancelar}
      </Button>
      <Button
        variante="primario"
        onclick={guardar}
        disabled={rutaAlbum.trim() === "" || llave.trim() === ""}
      >
        {t.accion.guardar}
      </Button>
    {:else if fase === "resultado"}
      <Button variante="primario" onclick={() => (fase = "lista")}>
        {t.accion.aceptar}
      </Button>
    {:else}
      <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    {/if}
  {/snippet}
</Modal>

<style>
  .lg {
    display: flex;
    flex-direction: column;
    gap: var(--esp-4);
  }
  .lg__desc {
    margin: 0;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .lg__lista {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .lg__item {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    padding: var(--esp-2) var(--esp-3);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-panel);
  }
  .lg__info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .lg__album {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lg__llave {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
  }
  .lg__badge {
    font-size: var(--tam-fuente-xs);
    padding: 2px var(--esp-2);
    border-radius: var(--radio-pill);
    background: var(--color-acento-tenue);
    color: var(--color-acento);
    white-space: nowrap;
  }
  .lg__acciones {
    display: flex;
    gap: var(--esp-1);
  }
  .lg__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .lg__progreso {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding: var(--esp-4) 0;
  }
  .lg__barra {
    height: 8px;
    border-radius: var(--radio-pill);
    background: var(--color-panel);
    overflow: hidden;
  }
  .lg__relleno {
    height: 100%;
    background: var(--color-acento);
    transition: width var(--transicion);
  }
  .lg__faselbl {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
    text-align: center;
  }
  .lg__badges {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }
  .lg__badges:empty {
    display: none;
  }
  .lg__cifras {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--esp-2);
    margin: 0;
    padding: var(--esp-2) 0;
  }
  .lg__cifras div {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-panel);
  }
  .lg__cifras dt {
    font-size: var(--tam-fuente-lg);
    font-weight: 600;
  }
  .lg__cifras dd {
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
    text-align: center;
  }
  .lg__creados {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .lg__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }
</style>
