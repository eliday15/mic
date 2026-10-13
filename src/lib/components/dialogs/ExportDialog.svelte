<!--
  ExportDialog — exporta el conjunto filtrado actual a CSV o XLSX (ex-frmExp del
  VB6). Los campos se eligen y ordenan en un `FieldPicker` (casillas + arrastre).
  Respeta filtro y orden activos del estado; el backend ignora offset/limit de
  la `QueryReq`. La selección de campos y el formato usados al exportar se
  recuerdan por álbum (los campos creados después entran al final, marcados).
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { Modal, Button, Select, TextInput, FieldPicker } from "$lib/components/ui";
  import { exportarRegistros, type FormatoExport } from "./exportarIpc";
  import {
    guardarRecordado,
    leerRecordado,
    restaurarCampos,
    type CamposRecordados,
  } from "./recordarDialogo";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  /** Lo que se recuerda por álbum entre aperturas del diálogo. */
  interface ExportRecordado extends CamposRecordados {
    formato: FormatoExport;
  }

  const SECCION = "exportar";
  const FORMATOS: FormatoExport[] = ["csv", "xlsx", "csv-mic"];

  // Campos de la tabla activa, ordenados por ordenVisible. Se capturan al abrir
  // el diálogo (snapshot): la lista de campos no cambia mientras está abierto.
  // svelte-ignore state_referenced_locally
  const camposTabla = estado.campos
    .filter((c) => c.tabla === estado.tabla)
    .slice()
    .sort((a, b) => a.ordenVisible - b.ordenVisible);

  // svelte-ignore state_referenced_locally
  const claveAlbum = estado.ruta || estado.nombre;
  const recordado = leerRecordado<ExportRecordado>(SECCION, claveAlbum);

  const opcionesCampos = camposTabla.map((c) => ({
    valor: c.nombre,
    detalle: t.campos.tipos[c.tipo],
  }));

  // Incluidos: lo recordado para el álbum o, si no hay, los visibles en orden.
  let incluidos = $state<string[]>(
    restaurarCampos(
      recordado,
      camposTabla.map((c) => c.nombre),
    ) ?? camposTabla.filter((c) => c.visible).map((c) => c.nombre),
  );

  let formato = $state<FormatoExport>(
    recordado?.formato && FORMATOS.includes(recordado.formato) ? recordado.formato : "csv",
  );
  let destino = $state("");
  let exportando = $state(false);

  const extension = $derived(formato === "xlsx" ? "xlsx" : "csv");

  // Al cambiar de formato, un destino ya escrito cambia de extensión con él.
  $effect(() => {
    const ext = extension;
    const actual = untrack(() => destino);
    const nuevo = actual.replace(/\.(csv|xlsx)$/i, `.${ext}`);
    if (nuevo !== actual) destino = nuevo;
  });


  const opcionesFormato = [
    { valor: "csv" as FormatoExport, etiqueta: t.exportar.csv },
    { valor: "xlsx" as FormatoExport, etiqueta: t.exportar.xlsx },
    { valor: "csv-mic" as FormatoExport, etiqueta: t.exportar.csvMic },
  ];

  async function examinar(): Promise<void> {
    const ext = extension;
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
    // Se recuerda la configuración usada (al cancelar no se guarda nada).
    guardarRecordado<ExportRecordado>(SECCION, claveAlbum, {
      campos: [...incluidos],
      conocidos: camposTabla.map((c) => c.nombre),
      formato,
    });
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
    <div class="exp__grupo">
      <span class="exp__etq">{t.exportar.campos}</span>
      <FieldPicker
        opciones={opcionesCampos}
        bind:seleccion={incluidos}
        etiqueta={t.exportar.campos}
        alto="300px"
      />
    </div>

    <div class="exp__campos">
      <label class="exp__grupo">
        <span class="exp__etq">{t.exportar.formato}</span>
        <Select bind:value={formato} opciones={opcionesFormato} />
      </label>

      <label class="exp__grupo">
        <span class="exp__etq">{t.exportar.destino}</span>
        <div class="exp__destino">
          <TextInput bind:value={destino} placeholder={`${estado.nombre}.${extension}`} />
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
  .exp__etq {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .exp__campos {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
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
