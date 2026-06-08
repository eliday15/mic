<!--
  FieldText — campo de texto de un registro. Mantiene un valor local enlazado y
  notifica el cambio confirmado (al perder el foco o pulsar Enter) vía `onCommit`.
-->
<script lang="ts">
  import { TextInput } from "$lib/components/ui";
  import type { CampoDef, Valor } from "$lib/domain/types";

  interface Props {
    campo: CampoDef;
    valor: Valor;
    invalido?: boolean;
    disabled?: boolean;
    /** Notifica un valor confirmado (blur/Enter). */
    onCommit: (valor: Valor) => void;
  }

  let { campo, valor, invalido = false, disabled = false, onCommit }: Props =
    $props();

  // svelte-ignore state_referenced_locally
  let texto = $state(valor === null ? "" : String(valor));

  // Sincroniza con cambios externos del valor (p. ej. recarga del registro).
  $effect(() => {
    const externo = valor === null ? "" : String(valor);
    texto = externo;
  });

  function confirmar(): void {
    const limpio = texto;
    onCommit(limpio === "" ? null : limpio);
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      (e.currentTarget as HTMLElement).blur();
    }
  }
</script>

<TextInput
  bind:value={texto}
  {invalido}
  {disabled}
  placeholder={campo.nombre}
  onblur={confirmar}
  onkeydown={onKeydown}
/>
