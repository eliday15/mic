<!--
  InspectorPanel — panel derecho de inspección/edición.
    - Selección de 1 registro: miniatura grande + DynamicForm compacto con
      guardado al confirmar cada campo (registro_editar), recargando los
      calculados; resumen de variantes.
    - Selección de N registros: edición en lote de un único campo común
      (registros_editar_lote).
  Se oculta cuando no hay selección.
-->
<script lang="ts">
  import { ImageOff, Layers } from "lucide-svelte";
  import {
    Select,
    Button,
    TextInput,
    NumberInput,
    Spinner,
  } from "$lib/components/ui";
  import DynamicForm from "$lib/components/editor/DynamicForm.svelte";
  import {
    registroObtener,
    registroEditar,
    registrosEditarLote,
    variantesListar,
  } from "$lib/ipc/commands";
  import { thumbUrl } from "$lib/ipc/thumbnails";
  import { validarValor } from "$lib/domain/validation";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type {
    RegistroCompleto,
    Valor,
    Valores,
  } from "$lib/domain/types";

  interface Props {
    estado: AlbumState;
    /** Disparador externo de recarga del registro mostrado. */
    recarga?: number;
    onAbrirEditor?: (id: number) => void;
  }

  let { estado, recarga = 0, onAbrirEditor }: Props = $props();

  let cargando = $state(false);
  let registro = $state<RegistroCompleto | null>(null);
  let valores = $state<Valores>({});
  let multidatos = $state<Record<string, string[]>>({});
  let numVariantes = $state(0);

  // Edición en lote (multi-selección).
  let campoLote = $state("");
  let valorLoteTexto = $state("");
  let valorLoteNum = $state<number | null>(null);
  let aplicandoLote = $state(false);

  const ids = $derived([...estado.seleccion]);
  const unico = $derived(ids.length === 1 ? ids[0] : null);
  const multiple = $derived(ids.length > 1);

  const camposTabla = $derived(
    estado.campos.filter((c) => c.tabla === estado.tabla),
  );

  // Campos editables en lote (sin calculados/multidatos).
  const camposLote = $derived(
    camposTabla
      .filter((c) => c.modificable && c.tipo !== "calculado" && c.tipo !== "multidato")
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  const campoLoteDef = $derived(
    camposLote.find((c) => c.nombre === campoLote) ?? null,
  );
  const loteEsNumero = $derived(
    campoLoteDef?.tipo === "numerico" || campoLoteDef?.tipo === "moneda",
  );

  $effect(() => {
    void recarga;
    const id = unico;
    if (id !== null) cargar(id);
    else registro = null;
  });

  async function cargar(id: number): Promise<void> {
    cargando = true;
    try {
      const reg = await registroObtener(estado.albumId, id, estado.tabla);
      registro = reg;
      valores = { ...reg.valores };
      multidatos = { ...reg.multidatos };
      if (estado.tabla === "principal") {
        const vs = await variantesListar(estado.albumId, id);
        numVariantes = vs.length;
      } else {
        numVariantes = 0;
      }
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.cargarRegistros);
      registro = null;
    } finally {
      cargando = false;
    }
  }

  async function commitValor(nombre: string, valor: Valor): Promise<void> {
    if (unico === null) return;
    try {
      const reg = await registroEditar(estado.albumId, unico, estado.tabla, {
        [nombre]: valor,
      });
      valores = { ...reg.valores };
      registro = reg;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
    }
  }

  async function commitMultidato(nombre: string, vs: string[]): Promise<void> {
    if (unico === null) return;
    try {
      const reg = await registroEditar(
        estado.albumId,
        unico,
        estado.tabla,
        {},
        { [nombre]: vs },
      );
      multidatos = { ...reg.multidatos };
      valores = { ...reg.valores };
      registro = reg;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
    }
  }

  async function aplicarLote(): Promise<void> {
    if (campoLoteDef === null) return;
    const bruto: Valor = loteEsNumero
      ? valorLoteNum
      : valorLoteTexto === ""
        ? null
        : valorLoteTexto;
    const res = validarValor(campoLoteDef, bruto);
    if (!res.ok) {
      ui.error(res.error ?? t.error.valorInvalido);
      return;
    }
    aplicandoLote = true;
    try {
      await registrosEditarLote(estado.albumId, ids, estado.tabla, {
        [campoLoteDef.nombre]: res.valor ?? null,
      });
      estado.refrescar();
      ui.exito(t.mensaje.guardado);
      valorLoteTexto = "";
      valorLoteNum = null;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.guardarRegistro);
    } finally {
      aplicandoLote = false;
    }
  }
</script>

<aside class="insp">
  {#if ids.length === 0}
    <div class="insp__vacio">{t.registro.sinSeleccionInspector}</div>
  {:else if multiple}
    <!-- Edición en lote -->
    <div class="insp__cabe">
      <span class="insp__conteo tabular">{ids.length}</span>
      <span class="insp__conteolbl">{t.registro.seleccion}</span>
    </div>
    <div class="insp__lote">
      <p class="insp__lotedesc">{t.registro.edicionLoteDesc}</p>
      <label class="insp__campo">
        <span class="insp__etq">{t.filtros.campo}</span>
        <Select
          bind:value={campoLote}
          opciones={camposLote.map((c) => ({ valor: c.nombre, etiqueta: c.nombre }))}
          placeholder={t.filtros.campo}
        />
      </label>
      {#if campoLoteDef}
        <label class="insp__campo">
          <span class="insp__etq">{t.filtros.valor}</span>
          {#if loteEsNumero}
            <NumberInput bind:value={valorLoteNum} />
          {:else}
            <TextInput bind:value={valorLoteTexto} />
          {/if}
        </label>
        <Button
          variante="primario"
          ancho
          cargando={aplicandoLote}
          onclick={aplicarLote}
        >
          {t.registro.edicionLote}
        </Button>
      {/if}
    </div>
  {:else if cargando}
    <div class="insp__centro"><Spinner tamano={20} /></div>
  {:else if registro && unico !== null}
    <!-- Edición individual -->
    <div class="insp__img">
      {#if registro.imagen}
        <img
          class="insp__pic"
          src={thumbUrl(estado.albumId, estado.tabla, unico, 256, registro.imagenVersion ?? 0)}
          alt={t.registro.imagen}
        />
      {:else}
        <span class="insp__sinimg"><ImageOff size={28} /></span>
      {/if}
    </div>

    <div class="insp__acc">
      <Button variante="secundario" tamano="sm" ancho onclick={() => onAbrirEditor?.(unico ?? 0)}>
        {t.editar.editarRegistro}
      </Button>
    </div>

    {#if numVariantes > 0}
      <div class="insp__variantes">
        <Layers size={14} />
        <span>{numVariantes} {t.grupos.conteo}</span>
      </div>
    {/if}

    <div class="insp__form">
      <DynamicForm
        albumId={estado.albumId}
        campos={camposTabla}
        {valores}
        {multidatos}
        compacto
        onCommitValor={commitValor}
        onCommitMultidato={commitMultidato}
      />
    </div>
  {/if}
</aside>

<style>
  .insp {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
    height: 100%;
    overflow-y: auto;
    padding: var(--esp-3);
    background: var(--color-panel);
    border-left: 1px solid var(--color-borde);
  }
  .insp__vacio {
    margin: auto;
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-sm);
    text-align: center;
  }
  .insp__centro {
    display: grid;
    place-items: center;
    flex: 1;
  }

  .insp__cabe {
    display: flex;
    align-items: baseline;
    gap: var(--esp-2);
  }
  .insp__conteo {
    font-size: var(--tam-fuente-xl);
    font-weight: 700;
  }
  .insp__conteolbl {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .insp__lote {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .insp__lotedesc {
    margin: 0;
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-tenue);
  }

  .insp__img {
    aspect-ratio: 1;
    border-radius: var(--radio-lg);
    background: var(--color-fondo);
    border: 1px solid var(--color-borde);
    overflow: hidden;
    display: grid;
    place-items: center;
  }
  .insp__pic {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .insp__sinimg {
    color: var(--color-texto-tenue);
  }

  .insp__variantes {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }

  .insp__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .insp__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .insp__form {
    flex: 1;
  }
</style>
