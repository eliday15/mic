<!--
  FieldCurrency — campo de moneda. Idéntico al numérico en captura (REAL f64),
  pero muestra un prefijo de divisa y respeta los decimales de presentación.
-->
<script lang="ts">
  import { NumberInput } from "$lib/components/ui";
  import { validarValor } from "$lib/domain/validation";
  import type { CampoDef, Valor } from "$lib/domain/types";

  interface Props {
    campo: CampoDef;
    valor: Valor;
    invalido?: boolean;
    disabled?: boolean;
    onCommit: (valor: Valor) => void;
    onError?: (mensaje: string) => void;
  }

  let {
    campo,
    valor,
    invalido = false,
    disabled = false,
    onCommit,
    onError,
  }: Props = $props();

  function aNumero(v: Valor): number | null {
    if (v === null || v === "") return null;
    const n = Number(v);
    return Number.isNaN(n) ? null : n;
  }

  // svelte-ignore state_referenced_locally
  let num = $state<number | null>(aNumero(valor));

  $effect(() => {
    num = aNumero(valor);
  });

  function confirmar(): void {
    if (num === null) {
      onCommit(null);
      return;
    }
    const res = validarValor(campo, num);
    if (res.ok) {
      onCommit(res.valor ?? null);
    } else {
      onError?.(res.error ?? "Valor inválido");
    }
  }
</script>

<div class="moneda">
  <span class="moneda__simbolo">$</span>
  <NumberInput
    bind:value={num}
    {invalido}
    {disabled}
    step={campo.decimales > 0 ? 1 / 10 ** campo.decimales : 1}
    onblur={confirmar}
  />
</div>

<style>
  .moneda {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
  }
  .moneda__simbolo {
    color: var(--color-texto-secundario);
    flex-shrink: 0;
  }
  .moneda :global(.num) {
    flex: 1;
  }
</style>
