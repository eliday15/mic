<!--
  Toast — aviso efímero. Recibe el objeto `Toast` del store de UI y un callback
  para cerrarse. Se anima al entrar y al salir.
-->
<script lang="ts">
  import { fly } from "svelte/transition";
  import { CircleCheck, CircleX, Info, TriangleAlert, X } from "lucide-svelte";
  import type { Toast } from "$lib/stores/ui.svelte";

  interface Props {
    toast: Toast;
    onCerrar: () => void;
  }

  let { toast, onCerrar }: Props = $props();

  const iconos = {
    info: Info,
    exito: CircleCheck,
    error: CircleX,
    aviso: TriangleAlert,
  };

  const Icono = $derived(iconos[toast.tipo]);
</script>

<div
  class="toast toast--{toast.tipo}"
  role="alert"
  transition:fly={{ y: 12, duration: 160 }}
>
  <span class="toast__icono"><Icono size={16} /></span>
  <span class="toast__msg">{toast.mensaje}</span>
  <button
    type="button"
    class="toast__cerrar"
    aria-label="Cerrar aviso"
    onclick={onCerrar}
  >
    <X size={14} />
  </button>
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    min-width: 240px;
    max-width: 380px;
    padding: var(--esp-2) var(--esp-3);
    border-radius: var(--radio);
    background: var(--color-elevado);
    border: 1px solid var(--color-borde);
    box-shadow: var(--sombra-2);
    font-size: var(--tam-fuente-sm);
  }

  .toast__icono {
    display: inline-flex;
    flex-shrink: 0;
  }

  .toast__msg {
    flex: 1;
    overflow-wrap: anywhere;
  }

  .toast--info .toast__icono {
    color: var(--color-acento);
  }
  .toast--exito .toast__icono {
    color: var(--color-exito);
  }
  .toast--error .toast__icono {
    color: var(--color-peligro);
  }
  .toast--aviso .toast__icono {
    color: var(--color-aviso);
  }

  .toast__cerrar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    padding: 0;
    border: none;
    border-radius: var(--radio-sm);
    background: transparent;
    color: var(--color-texto-secundario);
    cursor: pointer;
  }

  .toast__cerrar:hover {
    background: var(--color-hover);
    color: var(--color-texto);
  }
</style>
