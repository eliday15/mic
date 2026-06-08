<!--
  NumberInput — entrada numérica con binding a `number | null`. Muestra cifras
  tabulares y deja vacío el campo cuando el valor es `null`.
-->
<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  interface Props
    extends Omit<HTMLInputAttributes, "value" | "type" | "size"> {
    /** Valor numérico enlazado (`null` = vacío). */
    value?: number | null;
    invalido?: boolean;
    /** Paso del control. */
    step?: number | string;
    min?: number | string;
    max?: number | string;
  }

  let {
    value = $bindable(null),
    invalido = false,
    step = "any",
    min,
    max,
    ...rest
  }: Props = $props();

  // Texto local para no perder el foco al teclear separadores decimales.
  let texto = $state(value === null ? "" : String(value));

  // Sincroniza el texto cuando el valor cambia desde fuera.
  $effect(() => {
    const externo = value === null ? "" : String(value);
    if (Number(texto) !== value && texto.trim() !== externo) {
      texto = externo;
    }
  });

  function onInput(e: Event): void {
    texto = (e.currentTarget as HTMLInputElement).value;
    const limpio = texto.trim();
    if (limpio === "") {
      value = null;
      return;
    }
    const n = Number(limpio);
    value = Number.isNaN(n) ? value : n;
  }
</script>

<input
  class="num tabular"
  class:num--invalido={invalido}
  type="number"
  inputmode="decimal"
  {step}
  {min}
  {max}
  value={texto}
  oninput={onInput}
  aria-invalid={invalido}
  {...rest}
/>

<style>
  .num {
    height: var(--alto-control);
    width: 100%;
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
    color: var(--color-texto);
    text-align: right;
    outline: none;
    transition: border-color var(--transicion);
  }

  .num:focus {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }

  .num--invalido {
    border-color: var(--color-peligro);
  }
  .num--invalido:focus {
    box-shadow: 0 0 0 2px var(--color-peligro-tenue);
  }
</style>
