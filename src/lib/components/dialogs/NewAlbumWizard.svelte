<!--
  NewAlbumWizard — asistente para crear un álbum nuevo. Pide nombre y ruta de
  destino (vía plugin dialog `save`) y permite definir los campos iniciales de
  forma manual. La importación de plantillas .xms queda fuera de v1 (nota).
  Al crear, delega en el store de álbumes (`albumes.crear`).
-->
<script lang="ts">
  import { FolderOpen, Plus, Trash2, Save, DollarSign, Percent } from "lucide-svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { desktopDir } from "@tauri-apps/api/path";
  import {
    Modal,
    Button,
    TextInput,
    Select,
    IconButton,
    ConfirmDialog,
  } from "$lib/components/ui";
  import { albumes } from "$lib/stores/albums.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import {
    plantillasListar,
    plantillaGuardar,
    plantillaEliminar,
  } from "$lib/ipc/commands";
  import { t } from "$lib/i18n/es";
  import type { CampoNuevo, Plantilla, TipoCampo } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), onCerrar }: Props = $props();

  /** Campo del formulario: como CampoNuevo pero con fórmula siempre string
      (para el binding del input; se normaliza a null al crear/guardar). */
  type CampoForm = Omit<CampoNuevo, "formula"> & { formula: string };

  let nombre = $state("");
  let ruta = $state("");
  let campos = $state<CampoForm[]>([campoNuevo("Nombre", "texto")]);
  let creando = $state(false);

  // Ubicación por defecto: Escritorio/<Nombre>.micdb. Se actualiza con el
  // nombre hasta que el usuario elige una ruta propia con "Examinar…".
  let escritorio = $state("");
  let rutaManual = $state(false);

  $effect(() => {
    desktopDir()
      .then((d) => (escritorio = d.replace(/[/\\]+$/, "")))
      .catch(() => (escritorio = ""));
  });

  $effect(() => {
    if (rutaManual || escritorio === "") return;
    const base = nombre.trim() === "" ? "album" : nombre.trim();
    ruta = `${escritorio}/${base}.micdb`;
  });

  /** Plantilla elegida (vacío = desde cero). */
  let plantillaSel = $state("");
  let plantillas = $state<Plantilla[]>([]);
  /** Estado del input inline para guardar la plantilla actual. */
  let guardandoPlantilla = $state(false);
  let nombrePlantilla = $state("");
  /** Confirmación de borrado de la plantilla seleccionada. */
  let confirmarBorrarPlantilla = $state(false);

  // "Moneda" no es un tipo aparte en la UI: es Número + formato $ (toggle).
  const opcionesTipo: { valor: TipoCampo; etiqueta: string }[] = [
    { valor: "texto", etiqueta: t.campos.tipos.texto },
    { valor: "numerico", etiqueta: t.campos.tipos.numerico },
    { valor: "fecha", etiqueta: t.campos.tipos.fecha },
    { valor: "calculado", etiqueta: t.campos.tipos.calculado },
    { valor: "multidato", etiqueta: t.campos.tipos.multidato },
  ];

  /** Tipo mostrado en el select (moneda se presenta como número). */
  function tipoUi(tipo: TipoCampo): TipoCampo {
    return tipo === "moneda" ? "numerico" : tipo;
  }

  function esNumero(tipo: TipoCampo): boolean {
    return tipo === "numerico" || tipo === "moneda";
  }

  /** El campo admite formato de presentación ($ / %). */
  function admiteFormato(tipo: TipoCampo): boolean {
    return esNumero(tipo) || tipo === "calculado";
  }

  function monedaActiva(c: CampoForm): boolean {
    return c.tipo === "moneda" || c.formato === "moneda";
  }

  /** Alterna formato moneda: en número mapea al tipo; en calculado a formato. */
  function alternarMoneda(c: CampoForm): void {
    if (c.tipo === "calculado") {
      c.formato = c.formato === "moneda" ? null : "moneda";
      return;
    }
    if (c.tipo === "moneda") {
      c.tipo = "numerico";
    } else {
      c.tipo = "moneda";
      c.formato = null;
    }
  }

  /** Alterna formato porcentaje (excluyente con moneda). */
  function alternarPorcentaje(c: CampoForm): void {
    if (c.formato === "porcentaje") {
      c.formato = null;
    } else {
      c.formato = "porcentaje";
      if (c.tipo === "moneda") c.tipo = "numerico";
    }
  }

  const opcionesPlantilla = $derived([
    { valor: "", etiqueta: t.plantillas.desdeCero },
    ...plantillas.map((p) => ({ valor: p.nombre, etiqueta: p.nombre })),
  ]);

  $effect(() => {
    void cargarPlantillas();
  });

  async function cargarPlantillas(): Promise<void> {
    try {
      plantillas = await plantillasListar();
    } catch {
      plantillas = [];
    }
  }

  function campoNuevo(nom: string, tipo: TipoCampo, orden = 0): CampoForm {
    return {
      nombre: nom,
      tabla: "principal",
      tipo,
      decimales: 2,
      totalizable: false,
      formula: "",
      visible: true,
      modificable: true,
      ordenVisible: orden,
      formato: null,
    };
  }

  /** Normaliza los campos del formulario a CampoNuevo (fórmula vacía → null). */
  function aDefiniciones(lista: CampoForm[]): CampoNuevo[] {
    return lista
      .filter((c) => c.nombre.trim() !== "")
      .map((c, i) => ({
        ...c,
        nombre: c.nombre.trim(),
        formula:
          c.tipo === "calculado" && c.formula.trim() !== ""
            ? c.formula.trim()
            : null,
        ordenVisible: i,
      }));
  }

  /** Aplica la plantilla elegida reemplazando la lista de campos del wizard. */
  function aplicarPlantilla(nom: string): void {
    plantillaSel = nom;
    if (nom === "") return;
    const pl = plantillas.find((p) => p.nombre === nom);
    if (!pl) return;
    campos = pl.campos.map((c, i) => ({
      ...c,
      formula: c.formula ?? "",
      ordenVisible: i,
    }));
  }

  /** Guarda los campos actuales como plantilla con el nombre dado. */
  async function confirmarGuardarPlantilla(): Promise<void> {
    const nom = nombrePlantilla.trim();
    if (nom === "") return;
    const definidos = aDefiniciones(campos);
    try {
      await plantillaGuardar(nom, definidos);
      await cargarPlantillas();
      plantillaSel = nom;
      ui.exito(t.plantillas.guardada);
      guardandoPlantilla = false;
      nombrePlantilla = "";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  /** Elimina la plantilla seleccionada y refresca el selector. */
  async function eliminarPlantilla(): Promise<void> {
    const nom = plantillaSel;
    if (nom === "") return;
    try {
      await plantillaEliminar(nom);
      plantillaSel = "";
      await cargarPlantillas();
      ui.exito(t.plantillas.eliminada);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      confirmarBorrarPlantilla = false;
    }
  }

  async function elegirRuta(): Promise<void> {
    const base = nombre.trim() === "" ? "album" : nombre.trim();
    const sel = await save({
      title: t.nuevoAlbum.ubicacion,
      defaultPath: escritorio !== "" ? `${escritorio}/${base}.micdb` : `${base}.micdb`,
      filters: [{ name: "MIC", extensions: ["micdb"] }],
    });
    if (typeof sel === "string") {
      ruta = sel;
      rutaManual = true;
    }
  }

  let listaEl = $state<HTMLUListElement | null>(null);

  function agregarCampo(): void {
    campos = [...campos, campoNuevo("", "texto", campos.length)];
  }

  /** Enfoca el input de nombre de la fila `indice` (tras el render). */
  function enfocarFila(indice: number): void {
    queueMicrotask(() => {
      const inputs = listaEl?.querySelectorAll<HTMLInputElement>("input");
      inputs?.[indice]?.focus();
    });
  }

  function agregarCampoYEnfocar(): void {
    const nueva = campos.length;
    agregarCampo();
    enfocarFila(nueva);
  }

  /**
   * Enter en el nombre de un campo: en la última fila crea otra y salta a
   * ella (si la actual no está vacía); en las demás, baja a la siguiente.
   */
  function onEnterCampo(e: KeyboardEvent, i: number): void {
    if (e.key !== "Enter") return;
    e.preventDefault();
    if (i === campos.length - 1) {
      if (campos[i].nombre.trim() === "") return;
      agregarCampoYEnfocar();
    } else {
      enfocarFila(i + 1);
    }
  }

  function quitarCampo(indice: number): void {
    campos = campos.filter((_, i) => i !== indice);
  }

  /**
   * Sugiere un nombre libre cuando la ruta ya existe: "Test" → "Test 2",
   * "Test 2" → "Test 3"… La ruta se regenera (o se ajusta si era manual).
   */
  function sugerirNombreLibre(): void {
    const m = nombre.trim().match(/^(.*?)(?:\s+(\d+))?$/);
    const base = (m?.[1] ?? nombre.trim()) || "album";
    const n = m?.[2] ? Number(m[2]) + 1 : 2;
    nombre = `${base} ${n}`;
    if (rutaManual) {
      const corte = Math.max(ruta.lastIndexOf("/"), ruta.lastIndexOf("\\"));
      if (corte >= 0) ruta = `${ruta.slice(0, corte + 1)}${nombre}.micdb`;
    }
  }

  async function crear(): Promise<void> {
    if (nombre.trim() === "") {
      ui.aviso(t.nuevoAlbum.nombre);
      return;
    }
    if (ruta.trim() === "") {
      ui.aviso(t.nuevoAlbum.ubicacion);
      return;
    }
    const definidos = aDefiniciones(campos);
    creando = true;
    try {
      await albumes.crear(ruta.trim(), nombre.trim(), definidos);
      ui.exito(t.mensaje.albumCreado);
      cerrar();
    } catch (e) {
      if (typeof e === "string" && e.includes("ya existe")) {
        sugerirNombreLibre();
        ui.aviso(t.nuevoAlbum.yaExiste);
      } else {
        ui.error(typeof e === "string" ? e : t.error.crearAlbum);
      }
    } finally {
      creando = false;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.nuevoAlbum.titulo} ancho="lg" onCerrar={cerrar}>
  <div class="na">
    <div class="na__campo">
      <span class="na__etq">{t.plantillas.titulo}</span>
      <div class="na__plantilla">
        <Select
          value={plantillaSel}
          opciones={opcionesPlantilla}
          etiqueta={t.plantillas.titulo}
          onCambio={aplicarPlantilla}
        />
        {#if plantillaSel !== ""}
          <Button
            variante="fantasma"
            onclick={() => (confirmarBorrarPlantilla = true)}
          >
            <Trash2 size={16} />
            {t.plantillas.eliminar}
          </Button>
        {/if}
        <Button variante="secundario" onclick={() => (guardandoPlantilla = true)}>
          <Save size={16} />
          {t.plantillas.guardarComo}
        </Button>
      </div>
      {#if guardandoPlantilla}
        <div class="na__guardar">
          <TextInput
            bind:value={nombrePlantilla}
            placeholder={t.plantillas.nombre}
          />
          <Button
            variante="primario"
            tamano="sm"
            onclick={confirmarGuardarPlantilla}
            disabled={nombrePlantilla.trim() === ""}
          >
            {t.accion.guardar}
          </Button>
          <Button
            variante="fantasma"
            tamano="sm"
            onclick={() => {
              guardandoPlantilla = false;
              nombrePlantilla = "";
            }}
          >
            {t.accion.cancelar}
          </Button>
        </div>
      {/if}
    </div>

    <label class="na__campo">
      <span class="na__etq">{t.nuevoAlbum.nombre}</span>
      <TextInput bind:value={nombre} />
    </label>

    <label class="na__campo">
      <span class="na__etq">{t.nuevoAlbum.ubicacion}</span>
      <div class="na__ruta">
        <TextInput bind:value={ruta} placeholder="…/album.micdb" readonly />
        <Button variante="secundario" onclick={elegirRuta}>
          <FolderOpen size={16} />
          {t.accion.examinar}
        </Button>
      </div>
    </label>

    <div class="na__campos">
      <div class="na__camposhead">
        <span class="na__etq">{t.nuevoAlbum.campos}</span>
        <Button variante="fantasma" tamano="sm" onclick={agregarCampoYEnfocar}>
          <Plus size={14} />
          {t.campos.nuevo}
        </Button>
      </div>

      <ul class="na__lista" bind:this={listaEl}>
        {#each campos as campo, i (i)}
          <li class="na__item">
            <div class="na__fila">
              <TextInput
                bind:value={campo.nombre}
                placeholder={t.campos.nombre}
                onkeydown={(e) => onEnterCampo(e, i)}
              />
              <div class="na__tipo">
                <Select
                  value={tipoUi(campo.tipo)}
                  opciones={opcionesTipo}
                  etiqueta={t.campos.tipo}
                  onCambio={(v) => (campo.tipo = v)}
                />
                <!-- Formato $ / % : aplica a número y calculado. -->
                {#if admiteFormato(campo.tipo)}
                  <IconButton
                    etiqueta={t.campos.tipos.moneda}
                    tamano="sm"
                    activo={monedaActiva(campo)}
                    title={t.campos.tipos.moneda}
                    onclick={() => alternarMoneda(campo)}
                  >
                    <DollarSign size={14} />
                  </IconButton>
                  <IconButton
                    etiqueta={t.campos.formatoPorcentaje}
                    tamano="sm"
                    activo={campo.formato === "porcentaje"}
                    title={t.campos.formatoPorcentaje}
                    onclick={() => alternarPorcentaje(campo)}
                  >
                    <Percent size={14} />
                  </IconButton>
                {/if}
              </div>
              <IconButton etiqueta={t.accion.quitar} tamano="sm" onclick={() => quitarCampo(i)}>
                <Trash2 size={14} />
              </IconButton>
            </div>

            <!-- Fórmula inline del campo calculado. -->
            {#if campo.tipo === "calculado"}
              <div class="na__formula" title={t.nuevoAlbum.formulaNota}>
                <TextInput
                  bind:value={campo.formula}
                  placeholder={t.nuevoAlbum.formulaPlaceholder}
                >
                  {#snippet prefijo()}<span class="na__fx">=</span>{/snippet}
                </TextInput>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    <Button variante="primario" cargando={creando} onclick={crear}>
      {t.nuevoAlbum.crear}
    </Button>
  {/snippet}
</Modal>

{#if confirmarBorrarPlantilla}
  <ConfirmDialog
    bind:abierto={confirmarBorrarPlantilla}
    titulo={t.confirmar.eliminarPlantilla}
    mensaje={plantillaSel}
    textoConfirmar={t.accion.eliminar}
    peligro
    onConfirmar={eliminarPlantilla}
  />
{/if}

<style>
  .na {
    display: flex;
    flex-direction: column;
    gap: var(--esp-4);
  }
  .na__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .na__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .na__ruta {
    display: flex;
    gap: var(--esp-2);
  }
  .na__plantilla {
    display: flex;
    gap: var(--esp-2);
    align-items: center;
  }
  .na__plantilla :global(.select) {
    flex: 1;
  }
  .na__guardar {
    display: flex;
    gap: var(--esp-2);
    align-items: center;
    margin-top: var(--esp-2);
  }
  .na__guardar :global(.campo) {
    flex: 1;
  }
  .na__ruta :global(.campo) {
    flex: 1;
  }
  .na__campos {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding-top: var(--esp-2);
    border-top: 1px solid var(--color-borde);
  }
  .na__camposhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .na__lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    max-height: 280px;
    overflow-y: auto;
  }
  .na__item {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .na__fila {
    display: grid;
    grid-template-columns: 1fr 215px auto;
    align-items: center;
    gap: var(--esp-2);
  }
  .na__formula {
    /* Sangrada bajo el nombre, alineada con su columna. */
    margin-right: 225px;
  }
  .na__fx {
    font-family: var(--fuente-mono);
    color: var(--color-acento);
    font-weight: 600;
  }
  .na__tipo {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
  }
  .na__tipo :global(.select) {
    flex: 1;
    min-width: 0;
  }
</style>
