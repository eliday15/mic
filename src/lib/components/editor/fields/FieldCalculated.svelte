<!--
  FieldCalculated — campo calculado de solo lectura. Muestra el valor recalculado
  por el backend (devuelto tras guardar) formateado según el tipo. Opcionalmente
  ofrece una vista previa en vivo evaluando la fórmula con `formula_probar`
  (debounced) usando los valores actuales del formulario.
-->
<script lang="ts">
  import { Calculator } from "lucide-svelte";
  import { formatearNumero } from "$lib/utils/format";
  import { formulaProbar } from "$lib/ipc/commands";
  import { debounce } from "$lib/utils/debounce";
  import type { CampoDef, Valor, Valores } from "$lib/domain/types";

  interface Props {
    campo: CampoDef;
    valor: Valor;
    /** Id del álbum (para evaluar la vista previa). */
    albumId: number;
    /** Valores actuales del formulario (para la vista previa en vivo). */
    valoresActuales?: Valores;
    /** Activa la vista previa en vivo con `formula_probar`. */
    vistaPrevia?: boolean;
  }

  let {
    campo,
    valor,
    albumId,
    valoresActuales,
    vistaPrevia = false,
  }: Props = $props();

  let previo = $state<Valor>(null);
  let errorPrevio = $state<string | null>(null);

  function texto(v: Valor): string {
    if (v === null) return "—";
    if (typeof v === "number") return formatearNumero(v, campo.decimales);
    return String(v);
  }

  const evaluar = debounce(async (vals: Valores) => {
    if (!campo.formula) return;
    try {
      previo = await formulaProbar(albumId, campo.formula, vals);
      errorPrevio = null;
    } catch (e) {
      errorPrevio = typeof e === "string" ? e : "Error en la fórmula";
    }
  }, 300);

  $effect(() => {
    if (vistaPrevia && valoresActuales && campo.formula) {
      evaluar({ ...valoresActuales });
    }
  });

  const mostrado = $derived(
    vistaPrevia && errorPrevio === null && previo !== null ? previo : valor,
  );
</script>

<div class="calc" class:calc--error={errorPrevio !== null}>
  <span class="calc__icono"><Calculator size={14} /></span>
  <span class="calc__valor tabular">{texto(mostrado)}</span>
  {#if vistaPrevia && errorPrevio !== null}
    <span class="calc__err" title={errorPrevio}>!</span>
  {/if}
</div>

<style>
  .calc {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    height: var(--alto-control);
    padding: 0 var(--esp-2);
    border: 1px dashed var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-panel);
    color: var(--color-texto-secundario);
  }
  .calc--error {
    border-color: var(--color-peligro);
  }
  .calc__icono {
    display: inline-flex;
    color: var(--color-texto-tenue);
    flex-shrink: 0;
  }
  .calc__valor {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-texto);
  }
  .calc__err {
    color: var(--color-peligro);
    font-weight: 700;
    flex-shrink: 0;
  }
</style>
