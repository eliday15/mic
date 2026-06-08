<!--
  AppShell — armazón principal de la aplicación. Layout vertical:
    MenuBar (barra de menús propia) · Toolbar · TabStrip · contenido · StatusBar.
  Aloja la vista del álbum activo (AlbumView), la paleta de comandos y los
  diálogos globales (nuevo álbum, migración), además de la capa de toasts.
-->
<script lang="ts">
  import MenuBar from "./MenuBar.svelte";
  import Toolbar from "./Toolbar.svelte";
  import TabStrip from "./TabStrip.svelte";
  import StatusBar from "./StatusBar.svelte";
  import CommandPalette from "./CommandPalette.svelte";
  import AlbumView from "$lib/components/album/AlbumView.svelte";
  import NewAlbumWizard from "$lib/components/dialogs/NewAlbumWizard.svelte";
  import MigrateDialog from "$lib/components/dialogs/MigrateDialog.svelte";
  import { Toast } from "$lib/components/ui";
  import { albumes } from "$lib/stores/albums.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { ejecutarAccion } from "$lib/acciones";
  import { t } from "$lib/i18n/es";

  interface Props {
    /** Visibilidad de la paleta de comandos (controlada por App.svelte). */
    paletaAbierta?: boolean;
  }

  let { paletaAbierta = $bindable(false) }: Props = $props();

  const activo = $derived(albumes.activo);
  const hayAlbum = $derived(albumes.hayAlbumes);

  const dlgNuevoAlbum = $derived(ui.esModalActivo("nuevo-album"));
  const dlgMigrar = $derived(ui.esModalActivo("migrar"));
</script>

<div class="shell">
  <header class="shell__menu">
    <span class="shell__marca">{t.app.titulo}</span>
    <MenuBar {hayAlbum} onAccion={ejecutarAccion} />
  </header>

  <Toolbar estado={activo} />

  <main class="shell__cuerpo">
    {#if activo}
      <div class="shell__tabs"><TabStrip /></div>
      <div class="shell__vista">
        {#each albumes.abiertos as estado (estado.albumId)}
          {#if estado.albumId === albumes.activoId}
            <AlbumView {estado} />
          {/if}
        {/each}
      </div>
    {:else}
      <TabStrip />
    {/if}
  </main>

  <StatusBar estado={activo} />
</div>

<!-- Paleta de comandos global -->
<CommandPalette bind:abierto={paletaAbierta} />

<!-- Diálogos globales -->
{#if dlgNuevoAlbum}
  <NewAlbumWizard abierto onCerrar={() => ui.cerrarModal()} />
{/if}
{#if dlgMigrar}
  <MigrateDialog abierto onCerrar={() => ui.cerrarModal()} />
{/if}

<!-- Capa de toasts -->
<div class="shell__toasts" aria-live="polite">
  {#each ui.toasts as toast (toast.id)}
    <Toast {toast} onCerrar={() => ui.dismiss(toast.id)} />
  {/each}
</div>

<style>
  .shell {
    height: 100%;
    width: 100%;
    display: flex;
    flex-direction: column;
    background: var(--color-fondo);
    color: var(--color-texto);
    overflow: hidden;
  }

  .shell__menu {
    display: flex;
    align-items: center;
    height: 32px;
    padding-left: var(--esp-3);
    background: var(--color-panel);
    border-bottom: 1px solid var(--color-borde);
    flex-shrink: 0;
  }
  .shell__marca {
    font-size: var(--tam-fuente-sm);
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--color-texto);
    margin-right: var(--esp-3);
  }

  .shell__cuerpo {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .shell__tabs {
    flex-shrink: 0;
  }
  .shell__vista {
    flex: 1;
    min-height: 0;
  }

  .shell__toasts {
    position: fixed;
    bottom: var(--esp-4);
    right: var(--esp-4);
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    z-index: var(--z-toast);
    pointer-events: none;
  }
  .shell__toasts > :global(*) {
    pointer-events: auto;
  }
</style>
