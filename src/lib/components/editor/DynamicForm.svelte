<!--
  DynamicForm — genera el formulario de un registro a partir de las definiciones
  de campo, ordenadas por `ordenVisible`. Despacha cada campo al componente de
  tipo adecuado (fields/*). Reporta cambios confirmados por campo:
    - escalares  → onCommitValor(nombre, valor)
    - multidatos → onCommitMultidato(nombre, valores[])

  Modo compacto (`compacto`) reduce el espaciado para el InspectorPanel.
-->
<script lang="ts">
  import FieldText from "./fields/FieldText.svelte";
  import FieldNumber from "./fields/FieldNumber.svelte";
  import FieldCurrency from "./fields/FieldCurrency.svelte";
  import FieldDate from "./fields/FieldDate.svelte";
  import FieldCalculated from "./fields/FieldCalculated.svelte";
  import FieldMultidatos from "./fields/FieldMultidatos.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import type { CampoDef, Valor, Valores } from "$lib/domain/types";

  interface Props {
    albumId: number;
    campos: CampoDef[];
    valores: Valores;
    multidatos: Record<string, string[]>;
    compacto?: boolean;
    /** Vista previa en vivo de campos calculados. */
    vistaPreviaCalculados?: boolean;
    onCommitValor: (nombre: string, valor: Valor) => void;
    onCommitMultidato: (nombre: string, valores: string[]) => void;
  }

  let {
    albumId,
    campos,
    valores,
    multidatos,
    compacto = false,
    vistaPreviaCalculados = false,
    onCommitValor,
    onCommitMultidato,
  }: Props = $props();

  const ordenados = $derived(
    campos
      .slice()
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  function noModificable(campo: CampoDef): boolean {
    return !campo.modificable;
  }
</script>

<div class="form" class:form--compacto={compacto}>
  {#each ordenados as campo (campo.id)}
    <div class="form__campo">
      <span class="form__etq" title={campo.nombre}>{campo.nombre}</span>
      <div class="form__control">
        {#if campo.tipo === "texto"}
          <FieldText
            {campo}
            valor={valores[campo.nombre] ?? null}
            disabled={noModificable(campo)}
            onCommit={(v) => onCommitValor(campo.nombre, v)}
          />
        {:else if campo.tipo === "numerico"}
          <FieldNumber
            {campo}
            valor={valores[campo.nombre] ?? null}
            disabled={noModificable(campo)}
            onCommit={(v) => onCommitValor(campo.nombre, v)}
            onError={(m) => ui.error(m)}
          />
        {:else if campo.tipo === "moneda"}
          <FieldCurrency
            {campo}
            valor={valores[campo.nombre] ?? null}
            disabled={noModificable(campo)}
            onCommit={(v) => onCommitValor(campo.nombre, v)}
            onError={(m) => ui.error(m)}
          />
        {:else if campo.tipo === "fecha"}
          <FieldDate
            {campo}
            valor={valores[campo.nombre] ?? null}
            disabled={noModificable(campo)}
            onCommit={(v) => onCommitValor(campo.nombre, v)}
          />
        {:else if campo.tipo === "calculado"}
          <FieldCalculated
            {campo}
            {albumId}
            valor={valores[campo.nombre] ?? null}
            valoresActuales={valores}
            vistaPrevia={vistaPreviaCalculados}
          />
        {:else if campo.tipo === "multidato"}
          <FieldMultidatos
            {campo}
            {albumId}
            valores={multidatos[campo.nombre] ?? []}
            disabled={noModificable(campo)}
            onCambio={(vs) => onCommitMultidato(campo.nombre, vs)}
          />
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .form--compacto {
    gap: var(--esp-2);
  }

  .form__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }

  .form__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--color-texto-secundario);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .form__control {
    min-width: 0;
  }
</style>
