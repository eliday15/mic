<!--
  Spinner — indicador de carga circular, dimensionado con la prop `tamano`.
-->
<script lang="ts">
  interface Props {
    /** Diámetro en píxeles. */
    tamano?: number;
    /** Grosor del trazo en píxeles. */
    grosor?: number;
    /** Etiqueta accesible. */
    etiqueta?: string;
  }

  let { tamano = 18, grosor = 2, etiqueta = "Cargando" }: Props = $props();
</script>

<span
  class="spinner"
  role="status"
  aria-label={etiqueta}
  style="--tam:{tamano}px; --grosor:{grosor}px;"
></span>

<style>
  .spinner {
    display: inline-block;
    width: var(--tam);
    height: var(--tam);
    border: var(--grosor) solid var(--color-borde-fuerte);
    border-top-color: var(--color-acento);
    border-radius: 50%;
    animation: girar 0.7s linear infinite;
  }

  @keyframes girar {
    to {
      transform: rotate(360deg);
    }
  }
</style>
