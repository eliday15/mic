<!--
  FieldDate — campo de fecha ISO `YYYY-MM-DD`. Usa un input nativo de tipo date
  (el modelo guarda siempre ISO) y confirma al cambiar.
-->
<script lang="ts">
  import type { CampoDef, Valor } from "$lib/domain/types";

  interface Props {
    campo: CampoDef;
    valor: Valor;
    invalido?: boolean;
    disabled?: boolean;
    onCommit: (valor: Valor) => void;
  }

  let { campo, valor, invalido = false, disabled = false, onCommit }: Props =
    $props();

  function aIso(v: Valor): string {
    return typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : "";
  }

  // svelte-ignore state_referenced_locally
  let iso = $state(aIso(valor));

  $effect(() => {
    iso = aIso(valor);
  });

  function onChange(e: Event): void {
    iso = (e.currentTarget as HTMLInputElement).value;
    onCommit(iso === "" ? null : iso);
  }
</script>

<input
  class="fecha"
  class:fecha--invalido={invalido}
  type="date"
  value={iso}
  aria-label={campo.nombre}
  aria-invalid={invalido}
  {disabled}
  onchange={onChange}
/>

<style>
  .fecha {
    height: var(--alto-control);
    width: 100%;
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
    color: var(--color-texto);
    outline: none;
    transition: border-color var(--transicion);
  }
  .fecha:focus {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }
  .fecha--invalido {
    border-color: var(--color-peligro);
  }
  /* Icono del selector nativo legible en tema oscuro */
  .fecha::-webkit-calendar-picker-indicator {
    filter: invert(0.6);
    cursor: pointer;
  }
</style>
