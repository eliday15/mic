<!--
  SearchDialog — búsqueda libre (FTS) sobre el álbum. Fija el término en el
  estado (`setBusqueda`), lo que dispara una recarga de la grilla.
-->
<script lang="ts">
  import { Search } from "lucide-svelte";
  import { Modal, Button, TextInput } from "$lib/components/ui";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // svelte-ignore state_referenced_locally
  let termino = $state(estado.busqueda);

  function buscar(): void {
    estado.setBusqueda(termino.trim());
    cerrar();
  }

  function limpiar(): void {
    termino = "";
    estado.setBusqueda("");
    cerrar();
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      buscar();
    }
  }
</script>

<Modal bind:abierto titulo={t.herramientas.buscar} ancho="sm" onCerrar={cerrar}>
  <div class="sd" onkeydown={onKeydown} role="presentation">
    <TextInput bind:value={termino} placeholder={t.busqueda.placeholder}>
      {#snippet prefijo()}
        <Search size={16} />
      {/snippet}
    </TextInput>
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={limpiar}>{t.busqueda.limpiar}</Button>
    <Button variante="primario" onclick={buscar}>{t.herramientas.buscar}</Button>
  {/snippet}
</Modal>

<style>
  .sd {
    padding: var(--esp-1) 0;
  }
</style>
