<!--
  BatchUpdateDialog — actualización masiva de un campo (equivale a
  frmActGrlDat): elige campo, nuevo valor y alcance (selección actual, conjunto
  filtrado o todo el álbum) y aplica el cambio en una sola operación.
-->
<script lang="ts">
  import { Modal, Button, Select, TextInput, NumberInput } from "$lib/components/ui";
  import {
    registrosActualizarMasivo,
    registrosEditarLote,
  } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { QueryReq, Valor } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    /** Notifica que hubo cambios para refrescar la grilla. */
    onAplicado?: () => void;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onAplicado, onCerrar }: Props = $props();

  type Alcance = "seleccionados" | "filtrados" | "todos";

  // Campos editables del usuario: ni calculados ni multidatos.
  const editables = $derived(
    estado.campos
      .filter(
        (c) =>
          c.tabla === estado.tabla &&
          c.modificable &&
          c.tipo !== "calculado" &&
          c.tipo !== "multidato",
      )
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  let campoSel = $state("");
  let valorTexto = $state("");
  let valorNumero = $state<number | null>(null);
  let alcance = $state<Alcance>(
    // svelte-ignore state_referenced_locally
    estado.seleccion.size > 0 ? "seleccionados" : "filtrados",
  );
  let aplicando = $state(false);

  const def = $derived(editables.find((c) => c.nombre === campoSel) ?? null);
  const esNumerico = $derived(
    def?.tipo === "numerico" || def?.tipo === "moneda",
  );

  const opcionesCampo = $derived([
    { valor: "", etiqueta: `(${t.actMasiva.campo})` },
    ...editables.map((c) => ({ valor: c.nombre, etiqueta: c.nombre })),
  ]);

  const opcionesAlcance = $derived([
    ...(estado.seleccion.size > 0
      ? [
          {
            valor: "seleccionados" as Alcance,
            etiqueta: `${t.actMasiva.seleccionados} (${estado.seleccion.size})`,
          },
        ]
      : []),
    { valor: "filtrados" as Alcance, etiqueta: t.actMasiva.filtrados },
    { valor: "todos" as Alcance, etiqueta: t.actMasiva.todos },
  ]);

  function valorFinal(): Valor {
    if (esNumerico) return valorNumero;
    const v = valorTexto.trim();
    return v === "" ? null : v;
  }

  async function aplicar(): Promise<void> {
    if (!def) return;
    aplicando = true;
    const valores = { [def.nombre]: valorFinal() };
    try {
      let n: number;
      if (alcance === "seleccionados") {
        const ids = [...estado.seleccion];
        await registrosEditarLote(estado.albumId, ids, estado.tabla, valores);
        n = ids.length;
      } else {
        // "filtrados" respeta la consulta activa; "todos" la ignora.
        const req: QueryReq =
          alcance === "filtrados"
            ? estado.construirQuery(0, 1)
            : {
                tabla: estado.tabla,
                idPrincipal: null,
                grupo: null,
                filtroRapido: null,
                condiciones: [],
                busqueda: null,
                orden: [],
                incluirOcultos: true,
                offset: 0,
                limit: 1,
              };
        n = await registrosActualizarMasivo(estado.albumId, req, valores);
      }
      ui.exito(`${n} ${t.actMasiva.resultado}`);
      onAplicado?.();
      cerrar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      aplicando = false;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.actMasiva.titulo} ancho="sm" onCerrar={cerrar}>
  <div class="am">
    <label class="am__grupo">
      <span class="am__etq">{t.actMasiva.campo}</span>
      <Select bind:value={campoSel} opciones={opcionesCampo} />
    </label>

    <label class="am__grupo">
      <span class="am__etq">{t.actMasiva.nuevoValor}</span>
      {#if esNumerico}
        <NumberInput
          bind:value={valorNumero}
          step={def && def.decimales > 0 ? 10 ** -def.decimales : 1}
        />
      {:else if def?.tipo === "fecha"}
        <TextInput bind:value={valorTexto} placeholder="AAAA-MM-DD" />
      {:else}
        <TextInput bind:value={valorTexto} disabled={!def} />
      {/if}
    </label>

    <label class="am__grupo">
      <span class="am__etq">{t.actMasiva.alcance}</span>
      <Select bind:value={alcance} opciones={opcionesAlcance} />
    </label>

    <p class="am__aviso">{t.actMasiva.confirmacion}</p>
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    <Button
      variante="primario"
      onclick={aplicar}
      disabled={!def || aplicando}
      cargando={aplicando}
    >
      {t.actMasiva.aplicar}
    </Button>
  {/snippet}
</Modal>

<style>
  .am {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .am__grupo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .am__etq {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .am__aviso {
    margin: 0;
    font-size: var(--tam-fuente-sm);
    color: var(--color-aviso, #d9a440);
  }
</style>
