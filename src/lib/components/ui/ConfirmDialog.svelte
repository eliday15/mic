<!--
  ConfirmDialog — diálogo de confirmación sobre Modal. Devuelve la decisión por
  los callbacks `onConfirmar` / `onCancelar`.
-->
<script lang="ts">
  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import { t } from "$lib/i18n/es";

  interface Props {
    abierto?: boolean;
    titulo: string;
    mensaje?: string;
    /** Texto del botón de confirmación. */
    textoConfirmar?: string;
    /** Texto del botón de cancelación. */
    textoCancelar?: string;
    /** Usa estilo de peligro en el botón de confirmar. */
    peligro?: boolean;
    cargando?: boolean;
    onConfirmar: () => void;
    onCancelar?: () => void;
  }

  let {
    abierto = $bindable(true),
    titulo,
    mensaje,
    textoConfirmar = t.accion.aceptar,
    textoCancelar = t.accion.cancelar,
    peligro = false,
    cargando = false,
    onConfirmar,
    onCancelar,
  }: Props = $props();

  function cancelar(): void {
    abierto = false;
    onCancelar?.();
  }
</script>

<Modal
  bind:abierto
  {titulo}
  ancho="sm"
  botonCerrar={false}
  cerrarFuera={!cargando}
  onCerrar={cancelar}
>
  {#if mensaje}
    <p class="confirm__msg">{mensaje}</p>
  {/if}

  {#snippet pie()}
    <Button variante="fantasma" disabled={cargando} onclick={cancelar}>
      {textoCancelar}
    </Button>
    <Button
      variante={peligro ? "peligro" : "primario"}
      {cargando}
      onclick={onConfirmar}
    >
      {textoConfirmar}
    </Button>
  {/snippet}
</Modal>

<style>
  .confirm__msg {
    margin: 0;
    color: var(--color-texto-secundario);
    line-height: var(--interlineado);
  }
</style>
