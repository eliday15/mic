<!--
  FieldStructureEditor — editor de la estructura de campos del álbum (equivale a
  frmEdCmps del original). Lista reordenable por arrastre, alta/edición/baja con
  confirmación, y un formulario de propiedades dependiente del tipo. Para campos
  calculados embebe FormulaEditor con prueba en vivo.
-->
<script lang="ts">
  import { GripVertical, Plus, Pencil, Trash2 } from "lucide-svelte";
  import {
    Modal,
    Button,
    TextInput,
    NumberInput,
    Select,
    ConfirmDialog,
    IconButton,
    EmptyState,
  } from "$lib/components/ui";
  import FormulaEditor from "./FormulaEditor.svelte";
  import {
    campoCrear,
    campoEditar,
    campoEliminar,
    camposReordenar,
  } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type {
    AlbumState,
  } from "$lib/stores/albumState.svelte";
  import type { CampoDef, CampoNuevo, TipoCampo, Tabla } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // Copia ordenable de los campos.
  // svelte-ignore state_referenced_locally
  let campos = $state<CampoDef[]>(
    estado.campos.slice().sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  // Estado del formulario de edición/alta.
  let editorAbierto = $state(false);
  let editandoId = $state<number | null>(null);
  let borrador = $state<CampoNuevo>(nuevoBorrador());
  let guardando = $state(false);

  // Confirmación de borrado.
  let confirmarBorrado = $state(false);
  let idBorrar = $state<number | null>(null);
  let borrando = $state(false);

  // Arrastre de reordenación.
  let arrastrado = $state<number | null>(null);

  // "Moneda" no es un tipo aparte en la UI: es Número + formato de presentación.
  const opcionesTipo: { valor: TipoCampo; etiqueta: string }[] = [
    { valor: "texto", etiqueta: t.campos.tipos.texto },
    { valor: "numerico", etiqueta: t.campos.tipos.numerico },
    { valor: "fecha", etiqueta: t.campos.tipos.fecha },
    { valor: "calculado", etiqueta: t.campos.tipos.calculado },
    { valor: "multidato", etiqueta: t.campos.tipos.multidato },
  ];

  /** Opciones del formato de presentación (mapean tipo+formato internos). */
  type FormatoUi = "simple" | "moneda" | "pct";
  const opcionesFormato: { valor: FormatoUi; etiqueta: string }[] = [
    { valor: "simple", etiqueta: t.campos.formatoSimple },
    { valor: "moneda", etiqueta: t.campos.tipos.moneda },
    { valor: "pct", etiqueta: t.campos.formatoPorcentaje },
  ];

  /** Tipo mostrado en el select (moneda se presenta como número). */
  function tipoUi(tipo: TipoCampo): TipoCampo {
    return tipo === "moneda" ? "numerico" : tipo;
  }

  const opcionesTabla: { valor: Tabla; etiqueta: string }[] = [
    { valor: "principal", etiqueta: t.campos.tabla_.principal },
    { valor: "variantes", etiqueta: t.campos.tabla_.variantes },
  ];

  function nuevoBorrador(): CampoNuevo {
    return {
      nombre: "",
      tabla: "principal",
      tipo: "texto",
      decimales: 2,
      totalizable: false,
      formula: null,
      visible: true,
      modificable: true,
      ordenVisible: campos?.length ?? 0,
      formato: null,
    };
  }

  function tipoNombre(tipo: TipoCampo): string {
    return t.campos.tipos[tipo];
  }

  function abrirAlta(): void {
    editandoId = null;
    borrador = nuevoBorrador();
    formulaTexto = "";
    editorAbierto = true;
  }

  function abrirEdicion(c: CampoDef): void {
    editandoId = c.id;
    borrador = {
      nombre: c.nombre,
      tabla: c.tabla,
      tipo: c.tipo,
      decimales: c.decimales,
      totalizable: c.totalizable,
      formula: c.formula,
      visible: c.visible,
      modificable: c.modificable,
      ordenVisible: c.ordenVisible,
      formato: c.formato,
    };
    formulaTexto = c.formula ?? "";
    editorAbierto = true;
  }

  const esNumero = $derived(
    borrador.tipo === "numerico" || borrador.tipo === "moneda",
  );
  const esCalculado = $derived(borrador.tipo === "calculado");
  const muestraDecimales = $derived(esNumero || esCalculado);
  const muestraFormula = $derived(esCalculado);

  /** Valor del select de formato derivado del tipo+formato internos. */
  const formatoUi = $derived<FormatoUi>(
    borrador.tipo === "moneda" || borrador.formato === "moneda"
      ? "moneda"
      : borrador.formato === "porcentaje"
        ? "pct"
        : "simple",
  );

  /** Aplica el formato elegido: en número mapea al tipo (compatibilidad con
      `moneda` como tipo); en calculado solo toca `formato`. */
  function setFormatoUi(v: FormatoUi): void {
    if (esCalculado) {
      borrador.formato =
        v === "pct" ? "porcentaje" : v === "moneda" ? "moneda" : null;
      return;
    }
    if (v === "moneda") {
      borrador.tipo = "moneda";
      borrador.formato = null;
    } else if (v === "pct") {
      borrador.tipo = "numerico";
      borrador.formato = "porcentaje";
    } else {
      borrador.tipo = "numerico";
      borrador.formato = null;
    }
  }

  // Espejo string de la fórmula para enlace bidireccional con FormulaEditor.
  // Se sincroniza desde el borrador al abrir el editor y se vuelca al borrador
  // antes de guardar (evita un bucle reactivo entre ambos sentidos).
  let formulaTexto = $state("");

  async function guardarCampo(): Promise<void> {
    if (borrador.nombre.trim() === "") {
      ui.error(t.error.campoRequerido);
      return;
    }
    // Vuelca la fórmula editada (solo aplica a campos calculados).
    borrador.formula =
      borrador.tipo === "calculado" && formulaTexto.trim() !== ""
        ? formulaTexto
        : null;
    guardando = true;
    try {
      if (editandoId === null) {
        const creado = await campoCrear(estado.albumId, borrador);
        campos = [...campos, creado];
      } else {
        const editado = await campoEditar(estado.albumId, editandoId, borrador);
        campos = campos.map((c) => (c.id === editandoId ? editado : c));
      }
      estado.setCampos(campos);
      estado.refrescar();
      ui.exito(t.mensaje.guardado);
      editorAbierto = false;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      guardando = false;
    }
  }

  function pedirBorrado(id: number): void {
    idBorrar = id;
    confirmarBorrado = true;
  }

  async function borrarCampo(): Promise<void> {
    if (idBorrar === null) return;
    borrando = true;
    try {
      await campoEliminar(estado.albumId, idBorrar);
      campos = campos.filter((c) => c.id !== idBorrar);
      estado.setCampos(campos);
      estado.refrescar();
      ui.exito(t.mensaje.eliminado);
      confirmarBorrado = false;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      borrando = false;
      idBorrar = null;
    }
  }

  // --- Reordenación por arrastre ----------------------------------------
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
    arrastrado = null;
    try {
      const orden = campos.map((c) => c.id);
      await camposReordenar(estado.albumId, orden);
      // Refleja el nuevo ordenVisible localmente.
      campos = campos.map((c, i) => ({ ...c, ordenVisible: i }));
      estado.setCampos(campos);
      estado.refrescar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.campos.titulo} ancho="lg" onCerrar={cerrar}>
  <div class="fse">
    <div class="fse__barra">
      <span class="fse__hint">{t.campos.reordenar}</span>
      <Button variante="secundario" tamano="sm" onclick={abrirAlta}>
        <Plus size={14} />
        {t.campos.nuevo}
      </Button>
    </div>

    {#if campos.length === 0}
      <EmptyState titulo={t.vacio.titulo} descripcion={t.vacio.descripcion} />
    {:else}
      <ul class="fse__lista">
        {#each campos as c, i (c.id)}
          <li
            class="fse__fila"
            class:fse__fila--arrastrando={arrastrado === i}
            draggable="true"
            ondragstart={() => onDragStart(i)}
            ondragover={(e) => onDragOver(e, i)}
            ondrop={onDrop}
            ondragend={onDrop}
          >
            <span class="fse__grip"><GripVertical size={14} /></span>
            <span class="fse__nombre">{c.nombre}</span>
            <span class="fse__tipo">{tipoNombre(c.tipo)}</span>
            <span class="fse__tabla">{t.campos.tabla_[c.tabla]}</span>
            <span class="fse__acc">
              <IconButton etiqueta={t.accion.editar} tamano="sm" onclick={() => abrirEdicion(c)}>
                <Pencil size={14} />
              </IconButton>
              <IconButton etiqueta={t.accion.eliminar} tamano="sm" onclick={() => pedirBorrado(c.id)}>
                <Trash2 size={14} />
              </IconButton>
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#snippet pie()}
    <Button variante="primario" onclick={cerrar}>{t.accion.cerrar}</Button>
  {/snippet}
</Modal>

<!-- Formulario de alta/edición -->
{#if editorAbierto}
  <Modal
    bind:abierto={editorAbierto}
    titulo={editandoId === null ? t.campos.nuevo : t.accion.editar}
    ancho="md"
  >
    <div class="campo-form">
      <label class="campo-form__campo">
        <span class="campo-form__etq">{t.campos.nombre}</span>
        <TextInput bind:value={borrador.nombre} />
      </label>

      <div class="campo-form__fila">
        <label class="campo-form__campo">
          <span class="campo-form__etq">{t.campos.tipo}</span>
          <Select
            value={tipoUi(borrador.tipo)}
            opciones={opcionesTipo}
            etiqueta={t.campos.tipo}
            onCambio={(v) => {
              borrador.tipo = v;
              // El formato solo tiene sentido en número/calculado.
              if (v !== "numerico" && v !== "calculado") borrador.formato = null;
            }}
          />
        </label>
        <label class="campo-form__campo">
          <span class="campo-form__etq">{t.campos.tabla}</span>
          <Select bind:value={borrador.tabla} opciones={opcionesTabla} />
        </label>
      </div>

      <div class="campo-form__fila">
        {#if esNumero || esCalculado}
          <label class="campo-form__campo">
            <span class="campo-form__etq">{t.campos.formato}</span>
            <Select
              value={formatoUi}
              opciones={opcionesFormato}
              etiqueta={t.campos.formato}
              onCambio={setFormatoUi}
            />
          </label>
        {/if}
        {#if muestraDecimales}
          <label class="campo-form__campo campo-form__campo--corto">
            <span class="campo-form__etq">{t.campos.decimales}</span>
            <NumberInput bind:value={borrador.decimales} min={0} max={8} step={1} />
          </label>
        {/if}
      </div>

      <div class="campo-form__checks">
        <label class="campo-form__check">
          <input type="checkbox" bind:checked={borrador.visible} />
          {t.campos.visible}
        </label>
        <label class="campo-form__check">
          <input type="checkbox" bind:checked={borrador.modificable} />
          {t.campos.modificable}
        </label>
        <label class="campo-form__check">
          <input type="checkbox" bind:checked={borrador.totalizable} />
          {t.campos.totalizable}
        </label>
      </div>

      {#if muestraFormula}
        <div class="campo-form__formula">
          <FormulaEditor
            albumId={estado.albumId}
            bind:formula={formulaTexto}
            campos={campos.filter((c) => c.id !== editandoId)}
          />
        </div>
      {/if}
    </div>

    {#snippet pie()}
      <Button variante="fantasma" onclick={() => (editorAbierto = false)}>
        {t.accion.cancelar}
      </Button>
      <Button variante="primario" cargando={guardando} onclick={guardarCampo}>
        {t.accion.guardar}
      </Button>
    {/snippet}
  </Modal>
{/if}

{#if confirmarBorrado}
  <ConfirmDialog
    bind:abierto={confirmarBorrado}
    titulo={t.confirmar.eliminarCampo}
    mensaje={t.confirmar.eliminarCampoDesc}
    textoConfirmar={t.accion.eliminar}
    peligro
    cargando={borrando}
    onConfirmar={borrarCampo}
    onCancelar={() => (idBorrar = null)}
  />
{/if}

<style>
  .fse {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .fse__barra {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .fse__hint {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }
  .fse__lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 420px;
    overflow-y: auto;
  }
  .fse__fila {
    display: grid;
    grid-template-columns: 20px 1fr auto auto auto;
    align-items: center;
    gap: var(--esp-2);
    padding: var(--esp-1) var(--esp-2);
    border: 1px solid transparent;
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
    cursor: grab;
  }
  .fse__fila:hover {
    border-color: var(--color-borde);
  }
  .fse__fila--arrastrando {
    opacity: 0.5;
    border-color: var(--color-acento);
  }
  .fse__grip {
    color: var(--color-texto-tenue);
    display: inline-flex;
  }
  .fse__nombre {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .fse__tipo,
  .fse__tabla {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-secundario);
    padding: 2px var(--esp-2);
    border-radius: var(--radio-pill);
    background: var(--color-panel);
  }
  .fse__acc {
    display: inline-flex;
    gap: var(--esp-1);
  }

  .campo-form {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .campo-form__fila {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--esp-3);
  }
  .campo-form__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .campo-form__campo--corto {
    max-width: 140px;
  }
  .campo-form__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .campo-form__checks {
    display: flex;
    gap: var(--esp-4);
    flex-wrap: wrap;
  }
  .campo-form__check {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
    cursor: pointer;
  }
  .campo-form__formula {
    padding-top: var(--esp-2);
    border-top: 1px solid var(--color-borde);
  }
</style>
