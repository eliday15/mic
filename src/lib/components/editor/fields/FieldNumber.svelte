<!--
  FieldNumber — campo numérico de un registro. Enlaza un valor numérico local y
  confirma al perder el foco. Valida con `validarValor` antes de notificar.
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

<NumberInput
  bind:value={num}
  {invalido}
  {disabled}
  onblur={confirmar}
/>
