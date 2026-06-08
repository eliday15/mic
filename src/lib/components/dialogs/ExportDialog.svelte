<!--
  ExportDialog — exporta el conjunto filtrado actual a CSV o XLSX (ex-frmExp del
  VB6). UI dual-list: el usuario mueve campos entre "disponibles" e "incluidos"
  y ordena los incluidos (Subir/Bajar). Respeta filtro y orden activos del
  estado; el backend ignora offset/limit de la `QueryReq`.
-->
<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { Modal, Button, Select, TextInput } from "$lib/components/ui";
  import { exportarRegistros, type FormatoExport } from "./exportarIpc";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // Campos de la tabla activa, ordenados por ordenVisible. Se capturan al abrir
  // el diálogo (snapshot): la lista de campos no cambia mientras está abierto.
  // svelte-ignore state_referenced_locally
  const camposTabla = estado.campos
    .filter((c) => c.tabla === estado.tabla)
    .slice()
    .sort((a, b) => a.ordenVisible - b.ordenVisible);

  // Incluidos arranca con los visibles (en su ordenVisible); el resto, disponibles.
  let incluidos = $state<string[]>(
    camposTabla.filter((c) => c.visible).map((c) => c.nombre),
  );
  let disponibles = $state<string[]>(
    camposTabla.filter((c) => !c.visible).map((c) => c.nombre),
  );

  // Selección activa en cada lista (un nombre por lista).
  let selDisponible = $state<string | null>(null);
  let selIncluido = $state<string | null>(null);

  let formato = $state<FormatoExport>("csv");
  let destino = $state("");
  let exportando = $state(false);

  const opcionesFormato = [
    { valor: "csv" as FormatoExport, etiqueta: t.exportar.csv },
    { valor: "xlsx" as FormatoExport, etiqueta: t.exportar.xlsx },
    { valor: "csv-mic" as FormatoExport, etiqueta: t.exportar.csvMic },
  ];

  function agregar(): void {
    if (selDisponible === null) return;
    const nombre = selDisponible;
    disponibles = disponibles.filter((n) => n !== nombre);
    incluidos = [...incluidos, nombre];
    selDisponible = null;
    selIncluido = nombre;
  }

  function quitar(): void {
    if (selIncluido === null) return;
    const nombre = selIncluido;
    incluidos = incluidos.filter((n) => n !== nombre);
    disponibles = [...disponibles, nombre];
    selIncluido = null;
    selDisponible = nombre;
  }

  function subir(): void {
    if (selIncluido === null) return;
    const i = incluidos.indexOf(selIncluido);
    if (i <= 0) return;
    const copia = [...incluidos];
    [copia[i - 1], copia[i]] = [copia[i], copia[i - 1]];
    incluidos = copia;
  }

  function bajar(): void {
    if (selIncluido === null) return;
    const i = incluidos.indexOf(selIncluido);
    if (i < 0 || i >= incluidos.length - 1) return;
    const copia = [...incluidos];
    [copia[i], copia[i + 1]] = [copia[i + 1], copia[i]];
    incluidos = copia;
  }

  async function examinar(): Promise<void> {
    const ext = formato === "xlsx" ? "xlsx" : "csv";
    const sel = await save({
      defaultPath: `${estado.nombre}.${ext}`,
      filters: [
        {
          name: formato === "xlsx" ? "Excel" : "CSV",
          extensions: [ext],
        },
      ],
    });
    if (typeof sel === "string") destino = sel;
  }

  async function exportar(): Promise<void> {
    if (incluidos.length === 0) {
      ui.aviso(t.exportar.sinCampos);
      return;
    }
    if (destino.trim() === "") return;
    exportando = true;
    try {
      // El backend ignora offset/limit; pasamos un rango simbólico.
      const req = estado.construirQuery(0, 1);
      const n = await exportarRegistros(
        estado.albumId,
        req,
        incluidos,
        formato,
        destino.trim(),
      );
      ui.exito(`${n} ${t.exportar.resultado}`);
      cerrar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      exportando = false;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.exportar.titulo} ancho="md" onCerrar={cerrar}>
  <div class="exp">
    <div class="exp__listas">
      <div class="exp__col">
        <span class="exp__etq">{t.exportar.disponibles}</span>
        <ul class="exp__lista" role="listbox" aria-label={t.exportar.disponibles}>
          {#each disponibles as nombre (nombre)}
            <li>
              <button
                type="button"
                class="exp__item"
                class:exp__item--sel={selDisponible === nombre}
                role="option"
                aria-selected={selDisponible === nombre}
                onclick={() => (selDisponible = nombre)}
                ondblclick={() => {
                  selDisponible = nombre;
                  agregar();
                }}
              >
                {nombre}
              </button>
            </li>
          {/each}
        </ul>
      </div>

      <div class="exp__acciones">
        <Button
          variante="secundario"
          onclick={agregar}
          disabled={selDisponible === null}
        >
          {t.exportar.agregar}
        </Button>
        <Button
          variante="secundario"
          onclick={quitar}
          disabled={selIncluido === null}
        >
          {t.exportar.quitar}
        </Button>
      </div>

      <div class="exp__col">
        <span class="exp__etq">{t.exportar.incluidos}</span>
        <ul class="exp__lista" role="listbox" aria-label={t.exportar.incluidos}>
          {#each incluidos as nombre (nombre)}
            <li>
              <button
                type="button"
                class="exp__item"
                class:exp__item--sel={selIncluido === nombre}
                role="option"
                aria-selected={selIncluido === nombre}
                onclick={() => (selIncluido = nombre)}
                ondblclick={() => {
                  selIncluido = nombre;
                  quitar();
                }}
              >
                {nombre}
              </button>
            </li>
          {/each}
        </ul>
      </div>

      <div class="exp__orden">
        <Button
          variante="fantasma"
          onclick={subir}
          disabled={selIncluido === null}
        >
          {t.exportar.subir}
        </Button>
        <Button
          variante="fantasma"
          onclick={bajar}
          disabled={selIncluido === null}
        >
          {t.exportar.bajar}
        </Button>
      </div>
    </div>

    <div class="exp__campos">
      <label class="exp__grupo">
        <span class="exp__etq">{t.exportar.formato}</span>
        <Select bind:value={formato} opciones={opcionesFormato} />
      </label>

      <label class="exp__grupo">
        <span class="exp__etq">{t.exportar.destino}</span>
        <div class="exp__destino">
          <TextInput bind:value={destino} placeholder={`${estado.nombre}.csv`} />
          <Button variante="secundario" onclick={examinar}>
            {t.accion.examinar}
          </Button>
        </div>
      </label>
    </div>

    <p class="exp__nota">
      {formato === "csv-mic" ? t.exportar.notaMic : t.exportar.nota}
    </p>
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    <Button
      variante="primario"
      onclick={exportar}
      disabled={incluidos.length === 0 || destino.trim() === "" || exportando}
      cargando={exportando}
    >
      {exportando ? t.exportar.exportando : t.accion.aceptar}
    </Button>
  {/snippet}
</Modal>

<style>
  .exp {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .exp__listas {
    display: grid;
    grid-template-columns: 1fr auto 1fr auto;
    gap: var(--esp-2);
    align-items: stretch;
  }
  .exp__col {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
    min-width: 0;
  }
  .exp__etq {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .exp__lista {
    list-style: none;
    margin: 0;
    padding: var(--esp-1);
    height: 220px;
    overflow-y: auto;
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
  }
  .exp__item {
    display: block;
    width: 100%;
    padding: var(--esp-1) var(--esp-2);
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .exp__item:hover {
    background: var(--color-hover);
  }
  .exp__item--sel {
    background: var(--color-acento-tenue);
    color: var(--color-acento);
  }
  .exp__acciones,
  .exp__orden {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--esp-2);
  }
  .exp__campos {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: var(--esp-3);
  }
  .exp__grupo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .exp__destino {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--esp-2);
    align-items: center;
  }
  .exp__nota {
    margin: 0;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
</style>
