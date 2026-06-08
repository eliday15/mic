<!--
  TabStrip — pestañas de álbumes abiertos (cerrables). Cuando no hay álbumes
  abiertos muestra una pantalla de bienvenida con la lista de recientes y
  accesos directos a "Abrir álbum…" y "Migrar .mdb…".
-->
<script lang="ts">
  import { FolderOpen, Database, FileBox, Clock } from "lucide-svelte";
  import { Tabs, Button, EmptyState } from "$lib/components/ui";
  import { albumes } from "$lib/stores/albums.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { ejecutarAccion } from "$lib/acciones";
  import { albumesRecientes } from "$lib/ipc/commands";
  import { t } from "$lib/i18n/es";
  import type { AlbumReciente } from "$lib/domain/types";

  let recientes = $state<AlbumReciente[]>([]);

  const pestanas = $derived(
    albumes.abiertos.map((a) => ({
      id: a.albumId,
      etiqueta: a.nombre,
      cerrable: true,
    })),
  );

  $effect(() => {
    // Recargar recientes cuando cambie el número de álbumes abiertos.
    void albumes.abiertos.length;
    cargarRecientes();
  });

  async function cargarRecientes(): Promise<void> {
    try {
      recientes = await albumesRecientes();
    } catch {
      recientes = [];
    }
  }

  function activar(id: string | number): void {
    albumes.activar(Number(id));
  }

  async function cerrar(id: string | number): Promise<void> {
    try {
      await albumes.cerrar(Number(id));
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    }
  }

  async function abrirReciente(ruta: string): Promise<void> {
    try {
      await albumes.abrir(ruta);
      ui.exito(t.mensaje.albumAbierto);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.cargarAlbum);
    }
  }
</script>

{#if albumes.hayAlbumes}
  <div class="ts">
    <Tabs
      {pestanas}
      activa={albumes.activoId ?? undefined}
      onActivar={activar}
      onCerrar={cerrar}
    />
  </div>
{:else}
  <div class="bienvenida">
    <div class="bienvenida__caja">
      <div class="bienvenida__head">
        <FileBox size={40} />
        <h1 class="bienvenida__titulo">{t.app.titulo}</h1>
        <p class="bienvenida__sub">{t.app.subtitulo}</p>
      </div>

      <div class="bienvenida__acc">
        <Button variante="primario" onclick={() => ejecutarAccion("nuevo-album")}>
          <FileBox size={16} />
          {t.archivo.nuevoAlbum}
        </Button>
        <Button variante="secundario" onclick={() => ejecutarAccion("abrir")}>
          <FolderOpen size={16} />
          {t.archivo.abrir}
        </Button>
        <Button variante="secundario" onclick={() => ejecutarAccion("migrar")}>
          <Database size={16} />
          {t.archivo.importar}
        </Button>
      </div>

      <div class="bienvenida__recientes">
        <span class="bienvenida__rtitulo">
          <Clock size={13} />
          {t.archivo.recientes}
        </span>
        {#if recientes.length === 0}
          <EmptyState titulo={t.archivo.sinRecientes} />
        {:else}
          <ul class="bienvenida__lista">
            {#each recientes as r (r.ruta)}
              <li>
                <button
                  type="button"
                  class="bienvenida__item"
                  onclick={() => abrirReciente(r.ruta)}
                >
                  <span class="bienvenida__nombre">{r.nombre}</span>
                  <span class="bienvenida__ruta">{r.ruta}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .ts {
    flex-shrink: 0;
  }

  .bienvenida {
    flex: 1;
    display: grid;
    place-items: center;
    padding: var(--esp-8);
    overflow-y: auto;
  }
  .bienvenida__caja {
    width: 100%;
    max-width: 460px;
    display: flex;
    flex-direction: column;
    gap: var(--esp-6);
  }
  .bienvenida__head {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--esp-1);
    color: var(--color-texto-secundario);
    text-align: center;
  }
  .bienvenida__titulo {
    margin: var(--esp-2) 0 0;
    font-size: var(--tam-fuente-xl);
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--color-texto);
  }
  .bienvenida__sub {
    margin: 0;
    font-size: var(--tam-fuente-sm);
  }
  .bienvenida__acc {
    display: flex;
    justify-content: center;
    gap: var(--esp-2);
    flex-wrap: wrap;
  }
  .bienvenida__recientes {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    padding-top: var(--esp-4);
    border-top: 1px solid var(--color-borde);
  }
  .bienvenida__rtitulo {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-texto-secundario);
  }
  .bienvenida__lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .bienvenida__item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: var(--esp-2);
    border: none;
    border-radius: var(--radio);
    background: transparent;
    text-align: left;
    cursor: pointer;
    transition: background var(--transicion-rapida);
  }
  .bienvenida__item:hover {
    background: var(--color-hover);
  }
  .bienvenida__nombre {
    color: var(--color-texto);
    font-weight: 500;
  }
  .bienvenida__ruta {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
