<!--
  RecordEditor — editor completo de un registro (equivalente a frmCaptura del
  original). Modal grande con dos columnas:
    - Izquierda: imagen del registro (selector vía plugin dialog + drop) y
      botón "Ver 100 %" que abre un lightbox con la imagen original.
    - Derecha: DynamicForm con todos los campos + multidatos + VariantStrip.

  Soporta modo creación (sin id) y edición (con id). Autoguardado: cada dato
  confirmado programa un guardado diferido (`registro_crear` la primera vez en
  modo creación, `registro_editar` después) y notifica el id por `onGuardado`;
  al cerrar o navegar a una variante se vacía lo pendiente.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import {
    ImagePlus,
    ImageOff,
    Maximize2,
    X,
    ChevronLeft,
    Check,
    CircleAlert,
  } from "lucide-svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Modal, Button, Spinner } from "$lib/components/ui";
  import DynamicForm from "./DynamicForm.svelte";
  import VariantStrip from "./VariantStrip.svelte";
  import {
    registroObtener,
    registroCrear,
    registroEditar,
    registroImagenSet,
    thumbInvalidar,
  } from "$lib/ipc/commands";
  import { thumbUrl, imagenOriginalUrl } from "$lib/ipc/thumbnails";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type {
    CampoDef,
    RegistroCompleto,
    Tabla,
    Valor,
    Valores,
  } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    albumId: number;
    campos: CampoDef[];
    /** Id del registro a editar; `null` para crear uno nuevo. */
    id?: number | null;
    tabla: Tabla;
    /** Principal del que es variante (al crear/editar variantes). */
    idPrincipal?: number | null;
    onCerrar?: () => void;
    /** Notifica id guardado (creado o editado) para refrescar la grilla. */
    onGuardado?: (id: number) => void;
  }

  let {
    abierto = $bindable(true),
    albumId,
    campos,
    id = null,
    tabla,
    idPrincipal = null,
    onCerrar,
    onGuardado,
  }: Props = $props();

  let cargando = $state(false);

  /** Estado del autoguardado, visible en el pie del editor. */
  type EstadoGuardado = "nuevo" | "pendiente" | "guardando" | "guardado" | "error";
  /** Espera tras el último dato confirmado antes de guardar (ms). */
  const ESPERA_AUTOGUARDADO = 500;
  // svelte-ignore state_referenced_locally
  let estadoGuardado = $state<EstadoGuardado>(id === null ? "nuevo" : "guardado");
  let temporizador: ReturnType<typeof setTimeout> | null = null;
  // Cola de guardados: serializa crear/editar para no crear dos veces.
  let cola: Promise<void> = Promise.resolve();
  // svelte-ignore state_referenced_locally
  let idActual = $state<number | null>(id);
  // Tabla y principal "vivos": parten de los props pero cambian al navegar a
  // una variante (o al crear una nueva) sin salir del editor. Permiten volver
  // al principal por la miga de pan.
  // svelte-ignore state_referenced_locally
  let tablaActual = $state<Tabla>(tabla);
  // svelte-ignore state_referenced_locally
  let idPrincipalActual = $state<number | null>(idPrincipal);
  // Principal al que volver cuando se entra a una variante desde su tira. Es
  // null cuando el editor se abrió directamente como captura de variante (desde
  // el menú): en ese caso, guardar/cerrar cede el control al padre.
  let volverAPrincipal = $state<number | null>(null);
  let valores = $state<Valores>({});
  let multidatos = $state<Record<string, string[]>>({});
  let imagen = $state<string | null>(null);
  let imagenVersion = $state<number | null>(null);
  let lightbox = $state(false);

  // Mientras el lightbox está abierto, Escape lo cierra SOLO a él: se captura
  // antes de que el Modal (que también escucha Escape) cierre el editor entero.
  $effect(() => {
    if (!lightbox) return;
    function onEscCaptura(e: KeyboardEvent): void {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopImmediatePropagation();
      lightbox = false;
    }
    window.addEventListener("keydown", onEscCaptura, true);
    return () => window.removeEventListener("keydown", onEscCaptura, true);
  });
  let recargaVariantes = $state(0);

  const camposTabla = $derived(campos.filter((c) => c.tabla === tablaActual));
  const esVariante = $derived(tablaActual === "variantes");

  // Sincroniza SOLO cuando cambian las props (apertura desde el padre).
  // `untrack` evita que el effect rastree `tablaActual`/`camposTabla` que
  // `cargar`/`reiniciarNuevo` leen: sin él, navegar a una variante (que muta
  // `tablaActual`) re-dispararía este effect y revertiría la navegación.
  $effect(() => {
    void id;
    void tabla;
    void idPrincipal;
    untrack(() => {
      idActual = id;
      tablaActual = tabla;
      idPrincipalActual = idPrincipal;
      volverAPrincipal = null;
      if (id !== null) cargar(id);
      else reiniciarNuevo();
    });
  });

  /** ¿El registro nuevo tiene algo que valga la pena guardar? */
  function hayDatos(): boolean {
    if (imagenPendiente !== null) return true;
    if (Object.values(valores).some((v) => v !== null && v !== "")) return true;
    return Object.values(multidatos).some((vs) => vs.length > 0);
  }

  /** Hay texto tecleado sin confirmar: se guarda al salir del campo. */
  function marcarEscribiendo(): void {
    if (estadoGuardado === "guardado") estadoGuardado = "pendiente";
  }

  /** Marca cambios y programa el guardado diferido. */
  function programarGuardado(): void {
    estadoGuardado = "pendiente";
    if (temporizador !== null) clearTimeout(temporizador);
    temporizador = setTimeout(() => {
      temporizador = null;
      void guardarAhora();
    }, ESPERA_AUTOGUARDADO);
  }

  /** Guarda ya lo pendiente (encolado tras cualquier guardado en curso). */
  function guardarAhora(): Promise<void> {
    if (temporizador !== null) {
      clearTimeout(temporizador);
      temporizador = null;
    }
    cola = cola.then(persistir);
    return cola;
  }

  /** Copia de los calculados devueltos por el backend sin pisar lo demás. */
  function aplicarCalculados(reg: RegistroCompleto): void {
    const calc: Valores = {};
    for (const c of camposTabla) {
      if (c.tipo === "calculado") calc[c.nombre] = reg.valores[c.nombre] ?? null;
    }
    valores = { ...valores, ...calc };
  }

  /** Persiste el registro: lo crea la primera vez, después lo edita. */
  async function persistir(): Promise<void> {
    if (estadoGuardado !== "pendiente" && estadoGuardado !== "error") return;
    if (idActual === null && !hayDatos()) {
      estadoGuardado = "nuevo";
      return;
    }
    estadoGuardado = "guardando";
    const tablaGuardar = tablaActual;
    const valoresGuardar = $state.snapshot(valores) as Valores;
    const multidatosGuardar = $state.snapshot(multidatos) as Record<string, string[]>;
    try {
      let rid: number;
      if (idActual === null) {
        rid = await registroCrear(
          albumId,
          tablaGuardar,
          valoresGuardar,
          multidatosGuardar,
          imagenPendiente ?? undefined,
          tablaGuardar === "variantes" ? (idPrincipalActual ?? undefined) : undefined,
        );
        idActual = rid;
        const conImagen = imagenPendiente !== null;
        imagenPendiente = null;
        const reg = await registroObtener(albumId, rid, tablaGuardar);
        aplicarCalculados(reg);
        if (conImagen) {
          imagen = reg.imagen;
          imagenVersion = reg.imagenVersion;
        }
        if (tablaGuardar === "principal") recargaVariantes++;
      } else {
        rid = idActual;
        const reg = await registroEditar(
          albumId,
          rid,
          tablaGuardar,
          valoresGuardar,
          multidatosGuardar,
        );
        aplicarCalculados(reg);
      }
      // Si llegaron cambios durante el guardado, siguen pendientes.
      if (estadoGuardado === "guardando") estadoGuardado = "guardado";
      onGuardado?.(rid);
    } catch (e) {
      estadoGuardado = "error";
      ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
    }
  }

  /** Entra a editar una variante existente sin salir del editor. */
  async function abrirVariante(varianteId: number): Promise<void> {
    if (tablaActual === "variantes") return;
    await guardarAhora();
    volverAPrincipal = idActual;
    tablaActual = "variantes";
    idPrincipalActual = idActual;
    idActual = varianteId;
    cargar(varianteId);
  }

  /** Entra a capturar una variante nueva del principal actual. */
  async function nuevaVariante(): Promise<void> {
    if (tablaActual === "variantes") return;
    await guardarAhora();
    volverAPrincipal = idActual;
    tablaActual = "variantes";
    idPrincipalActual = idActual;
    idActual = null;
    reiniciarNuevo();
  }

  /** Vuelve al principal desde la edición/captura de una variante. */
  async function volverAlPrincipal(): Promise<void> {
    const pid = volverAPrincipal;
    if (pid === null) return;
    await guardarAhora();
    if (volverAPrincipal !== pid) return;
    volverAPrincipal = null;
    tablaActual = "principal";
    idPrincipalActual = null;
    idActual = pid;
    recargaVariantes++;
    cargar(pid);
  }

  function reiniciarNuevo(): void {
    const v: Valores = {};
    const m: Record<string, string[]> = {};
    for (const c of camposTabla) {
      if (c.tipo === "multidato") m[c.nombre] = [];
      else v[c.nombre] = null;
    }
    valores = v;
    multidatos = m;
    imagen = null;
    imagenVersion = null;
    imagenPendiente = null;
    estadoGuardado = "nuevo";
  }

  async function cargar(rid: number): Promise<void> {
    cargando = true;
    try {
      const reg: RegistroCompleto = await registroObtener(albumId, rid, tablaActual);
      valores = { ...reg.valores };
      multidatos = { ...reg.multidatos };
      imagen = reg.imagen;
      imagenVersion = reg.imagenVersion;
      estadoGuardado = "guardado";
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.cargarRegistros);
    } finally {
      cargando = false;
    }
  }

  function onCommitValor(nombre: string, valor: Valor): void {
    if (valores[nombre] === valor) {
      // Tecleó y volvió al valor original: no hay nada que guardar.
      if (estadoGuardado === "pendiente" && temporizador === null) {
        estadoGuardado = idActual === null ? "nuevo" : "guardado";
      }
      return;
    }
    valores = { ...valores, [nombre]: valor };
    programarGuardado();
  }

  function onCommitMultidato(nombre: string, vs: string[]): void {
    multidatos = { ...multidatos, [nombre]: vs };
    programarGuardado();
  }

  /** Aplica una imagen a un registro ya existente. */
  async function aplicarImagen(rutaOrigen: string, rid: number): Promise<void> {
    const res = await registroImagenSet(albumId, rid, tablaActual, rutaOrigen);
    await thumbInvalidar(albumId, rid, tablaActual);
    imagen = res.imagen;
    imagenVersion = res.imagenVersion;
    onGuardado?.(rid);
  }

  /** Imagen pendiente de aplicar tras crear (cuando aún no hay id). */
  let imagenPendiente = $state<string | null>(null);

  async function elegirImagen(): Promise<void> {
    const sel = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "Imágenes", extensions: ["jpg", "jpeg", "png", "gif", "webp", "bmp"] },
      ],
    });
    if (typeof sel !== "string") return;
    if (idActual !== null) {
      try {
        await aplicarImagen(sel, idActual);
      } catch (e) {
        ui.error(typeof e === "string" ? e : t.error.cargarImagen);
      }
    } else {
      // Sin id aún: la imagen viaja con el alta, que se dispara ya mismo.
      imagenPendiente = sel;
      programarGuardado();
    }
  }

  /**
   * Cierra el editor vaciando lo pendiente. El campo con foco confirma su
   * valor al perderlo (blur síncrono) antes de que el modal se desmonte; el
   * guardado sigue en segundo plano y avisa por toast si falla.
   */
  function cerrar(): void {
    (document.activeElement as HTMLElement | null)?.blur?.();
    void guardarAhora();
    abierto = false;
    onCerrar?.();
  }

  // --- Drag & drop de imagen sobre el panel izquierdo --------------------
  let sobreDrop = $state(false);

  // El drop real de archivos del SO llega por el evento de webview gestionado
  // en AlbumView; aquí soportamos el drop HTML estándar (rutas no disponibles
  // en navegador puro, pero sí el feedback visual para arrastres internos).
  function onDragOver(e: DragEvent): void {
    e.preventDefault();
    sobreDrop = true;
  }
  function onDragLeave(): void {
    sobreDrop = false;
  }
  function onDrop(e: DragEvent): void {
    e.preventDefault();
    sobreDrop = false;
  }

  const tieneImagen = $derived(imagen !== null && imagenVersion !== null);
</script>

<Modal
  bind:abierto
  ancho="xl"
  titulo={esVariante
    ? idActual === null
      ? t.registro.nuevaVariante
      : t.registro.editarVariante
    : idActual === null
      ? t.registro.nuevo
      : t.registro.editar}
  onCerrar={cerrar}
>
  {#if cargando}
    <div class="re__cargando"><Spinner tamano={24} /></div>
  {:else}
    <div class="re">
      <!-- Columna imagen -->
      <div class="re__imgcol">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="re__imgbox"
          class:re__imgbox--drop={sobreDrop}
          ondragover={onDragOver}
          ondragleave={onDragLeave}
          ondrop={onDrop}
        >
          {#if tieneImagen}
            <img
              class="re__img"
              src={thumbUrl(albumId, tablaActual, idActual ?? 0, 512, imagenVersion ?? 0)}
              alt={t.registro.imagen}
            />
            <button
              type="button"
              class="re__ver100"
              title="Ver 100 %"
              onclick={() => (lightbox = true)}
            >
              <Maximize2 size={14} /> 100 %
            </button>
          {:else}
            <div class="re__sinimg">
              <ImageOff size={32} />
              <span>{t.registro.sinImagen}</span>
            </div>
          {/if}
        </div>
        <Button variante="secundario" ancho onclick={elegirImagen}>
          <ImagePlus size={16} />
          {t.registro.elegirImagen}
        </Button>
      </div>

      <!-- Columna datos -->
      <!-- Teclear sin confirmar aún (el guardado ocurre al salir del campo). -->
      <div class="re__datos" oninput={marcarEscribiendo}>
        {#if esVariante && volverAPrincipal !== null}
          <button type="button" class="re__volver" onclick={volverAlPrincipal}>
            <ChevronLeft size={14} />
            {t.registro.volverPrincipal}
          </button>
        {/if}

        <DynamicForm
          {albumId}
          campos={camposTabla}
          {valores}
          {multidatos}
          vistaPreviaCalculados
          {onCommitValor}
          {onCommitMultidato}
        />

        {#if !esVariante && idActual !== null}
          <div class="re__variantes">
            <VariantStrip
              {albumId}
              idPrincipal={idActual}
              recarga={recargaVariantes}
              onSeleccionar={abrirVariante}
              onNueva={nuevaVariante}
            />
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#snippet pie()}
    <span class="re__estado re__estado--{estadoGuardado}" role="status" aria-live="polite">
      {#if estadoGuardado === "guardando"}
        <Spinner tamano={12} />
        {t.registro.autoguardado.guardando}
      {:else if estadoGuardado === "guardado"}
        <Check size={14} />
        {t.registro.autoguardado.guardado}
      {:else if estadoGuardado === "pendiente"}
        {t.registro.autoguardado.pendiente}
      {:else if estadoGuardado === "error"}
        <CircleAlert size={14} />
        {t.registro.autoguardado.error}
        <button type="button" class="re__reintentar" onclick={() => void guardarAhora()}>
          {t.registro.autoguardado.reintentar}
        </button>
      {:else}
        {t.registro.autoguardado.nuevo}
      {/if}
    </span>
    <Button variante="primario" onclick={cerrar}>
      {t.accion.cerrar}
    </Button>
  {/snippet}
</Modal>

{#if lightbox && tieneImagen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="lightbox" onclick={() => (lightbox = false)}>
    <button
      type="button"
      class="lightbox__cerrar"
      aria-label={t.accion.cerrar}
      onclick={() => (lightbox = false)}
    >
      <X size={20} />
    </button>
    <img
      class="lightbox__img"
      src={imagenOriginalUrl(albumId, tablaActual, idActual ?? 0, imagenVersion ?? 0)}
      alt={t.registro.imagen}
    />
  </div>
{/if}

<style>
  .re__estado {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    margin-right: auto;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .re__estado--guardado {
    color: var(--color-exito, var(--color-texto-secundario));
  }
  .re__estado--error {
    color: var(--color-peligro, #e5484d);
  }
  .re__reintentar {
    margin-left: var(--esp-1);
    padding: 0;
    border: none;
    background: none;
    color: var(--color-acento);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .re {
    display: grid;
    grid-template-columns: 320px 1fr;
    gap: var(--esp-5);
    min-height: 420px;
  }
  .re__cargando {
    display: grid;
    place-items: center;
    min-height: 320px;
  }

  .re__imgcol {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .re__imgbox {
    position: relative;
    aspect-ratio: 1;
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-lg);
    background: var(--color-panel);
    overflow: hidden;
    display: grid;
    place-items: center;
  }
  .re__imgbox--drop {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }
  .re__img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .re__ver100 {
    position: absolute;
    bottom: var(--esp-2);
    right: var(--esp-2);
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: none;
    border-radius: var(--radio-sm);
    background: var(--color-overlay);
    color: #fff;
    font-size: var(--tam-fuente-xs);
    cursor: pointer;
  }
  .re__sinimg {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--esp-2);
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-sm);
  }

  .re__datos {
    display: flex;
    flex-direction: column;
    gap: var(--esp-4);
    min-width: 0;
  }
  .re__variantes {
    padding-top: var(--esp-3);
    border-top: 1px solid var(--color-borde);
  }
  .re__volver {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    align-self: flex-start;
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    background: var(--color-panel);
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
    transition:
      background var(--transicion-rapida),
      color var(--transicion-rapida);
  }
  .re__volver:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    display: grid;
    place-items: center;
    padding: var(--esp-6);
    background: rgba(0, 0, 0, 0.88);
    cursor: zoom-out;
  }
  .lightbox__img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .lightbox__cerrar {
    position: absolute;
    top: var(--esp-4);
    right: var(--esp-4);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--alto-control-lg);
    height: var(--alto-control-lg);
    border: none;
    border-radius: var(--radio);
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    cursor: pointer;
  }
  .lightbox__cerrar:hover {
    background: rgba(255, 255, 255, 0.22);
  }
</style>
