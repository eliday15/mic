<!--
  Select — desplegable nativo estilizado con binding a `value`. Las opciones se
  pasan como lista `{ valor, etiqueta }`.
-->
<script lang="ts" generics="T extends string | number">
  import { ChevronDown } from "lucide-svelte";

  interface Opcion {
    valor: T;
    etiqueta: string;
    deshabilitada?: boolean;
  }

  interface Props {
    /** Valor seleccionado enlazado bidireccionalmente. */
    value?: T;
    opciones: Opcion[];
    /** Opción de marcador (sin valor real) mostrada primero. */
    placeholder?: string;
    invalido?: boolean;
    disabled?: boolean;
    /** Etiqueta accesible (aria-label) si no hay un <label> asociado. */
    etiqueta?: string;
    onCambio?: (valor: T) => void;
  }

  let {
    value = $bindable(),
    opciones,
    placeholder,
    invalido = false,
    disabled = false,
    etiqueta,
    onCambio,
  }: Props = $props();

  function onChange(e: Event): void {
    const sel = (e.currentTarget as HTMLSelectElement).value;
    // Restaura el tipo numérico si las opciones son numéricas.
    const opcion = opciones.find((o) => String(o.valor) === sel);
    if (opcion) {
      value = opcion.valor;
      onCambio?.(opcion.valor);
    }
  }
</script>

<div class="select" class:select--invalido={invalido}>
  <select
    class="select__nativo"
    value={value as string | number | undefined}
    onchange={onChange}
    aria-invalid={invalido}
    aria-label={etiqueta}
    {disabled}
  >
    {#if placeholder !== undefined}
      <option value="" disabled selected={value === undefined}>
        {placeholder}
      </option>
    {/if}
    {#each opciones as op (op.valor)}
      <option value={op.valor} disabled={op.deshabilitada}>
        {op.etiqueta}
      </option>
    {/each}
  </select>
  <span class="select__flecha"><ChevronDown size={14} /></span>
</div>

<style>
  .select {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 100%;
  }

  .select__nativo {
    width: 100%;
    height: var(--alto-control);
    padding: 0 var(--esp-5) 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
    color: var(--color-texto);
    appearance: none;
    cursor: pointer;
    outline: none;
    transition: border-color var(--transicion);
  }

  .select__nativo:focus {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }

  .select__nativo:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .select--invalido .select__nativo {
    border-color: var(--color-peligro);
  }

  .select__flecha {
    position: absolute;
    right: var(--esp-2);
    display: inline-flex;
    pointer-events: none;
    color: var(--color-texto-secundario);
  }

  .select__nativo option {
    background: var(--color-elevado);
    color: var(--color-texto);
  }
</style>
