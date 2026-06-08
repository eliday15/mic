<!--
  FieldMultidatos — campo multivalor. Muestra los valores como chips removibles,
  un Combobox con autocomplete por categorías para añadir nuevos, y un botón ⋯
  que abre el CategoryManager para gestionar el catálogo del campo.
-->
<script lang="ts">
  import { MoreHorizontal } from "lucide-svelte";
  import { Chip, Combobox, IconButton } from "$lib/components/ui";
  import CategoryManager from "../CategoryManager.svelte";
  import { categoriasSugerir } from "$lib/ipc/commands";
  import type { CampoDef } from "$lib/domain/types";

  interface Props {
    campo: CampoDef;
    albumId: number;
    /** Valores actuales del campo multidato. */
    valores: string[];
    disabled?: boolean;
    /** Notifica el nuevo conjunto de valores. */
    onCambio: (valores: string[]) => void;
  }

  let { campo, albumId, valores, disabled = false, onCambio }: Props =
    $props();

  let entrada = $state("");
  let gestor = $state(false);

  const principal = $derived(campo.tabla === "principal");

  function sugerir(prefijo: string): Promise<string[]> {
    return categoriasSugerir(albumId, campo.id, principal, prefijo);
  }

  function agregar(valor: string): void {
    const v = valor.trim();
    if (v === "") return;
    if (valores.some((x) => x.toLowerCase() === v.toLowerCase())) {
      entrada = "";
      return;
    }
    onCambio([...valores, v]);
    entrada = "";
  }

  function quitar(indice: number): void {
    onCambio(valores.filter((_, i) => i !== indice));
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter" && entrada.trim() !== "") {
      e.preventDefault();
      agregar(entrada);
    }
  }
</script>

<div class="multi">
  {#if valores.length > 0}
    <div class="multi__chips">
      {#each valores as v, i (v)}
        <Chip onQuitar={disabled ? undefined : () => quitar(i)}>{v}</Chip>
      {/each}
    </div>
  {/if}

  <div class="multi__alta">
    <div class="multi__combo" onkeydown={onKeydown} role="presentation">
      <Combobox
        bind:value={entrada}
        buscar={sugerir}
        {disabled}
        placeholder={campo.nombre}
        onSeleccionar={agregar}
      />
    </div>
    <IconButton
      etiqueta="Gestionar categorías"
      tamano="md"
      {disabled}
      onclick={() => (gestor = true)}
    >
      <MoreHorizontal size={16} />
    </IconButton>
  </div>
</div>

{#if gestor}
  <CategoryManager
    bind:abierto={gestor}
    {albumId}
    campoId={campo.id}
    {principal}
    titulo={campo.nombre}
  />
{/if}

<style>
  .multi {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .multi__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }
  .multi__alta {
    display: flex;
    gap: var(--esp-2);
    align-items: center;
  }
  .multi__combo {
    flex: 1;
  }
</style>
