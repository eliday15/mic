<!--
  CopyAlbumDialog — copia el álbum a otra ruta (equivale a frmCopAlbum):
  copia completa (datos + imágenes) o solo la estructura (campos, categorías,
  grupos y filtros, sin registros). El destino se elige con el diálogo del SO.
-->
<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { Modal, Button, TextInput } from "$lib/components/ui";
  import { albumCopiar } from "$lib/ipc/commands";
  import { formatearEntero } from "$lib/utils/format";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  let destino = $state("");
  let soloEstructura = $state(false);
  let copiando = $state(false);

  async function examinar(): Promise<void> {
    const sel = await save({
      defaultPath: `${estado.nombre} (copia).micdb`,
      filters: [{ name: "MIC", extensions: ["micdb"] }],
    });
    if (typeof sel === "string") destino = sel;
  }

  async function copiar(): Promise<void> {
    if (destino.trim() === "") return;
    copiando = true;
    try {
      const imagenes = await albumCopiar(
        estado.albumId,
        destino.trim(),
        soloEstructura,
      );
      const detalle = soloEstructura
        ? ""
        : ` · ${formatearEntero(imagenes)} ${t.copiarAlbum.imagenes}`;
      ui.exito(`${t.copiarAlbum.resultado}${detalle}`);
      cerrar();
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      copiando = false;
    }
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.copiarAlbum.titulo} ancho="sm" onCerrar={cerrar}>
  <div class="ca">
    <label class="ca__grupo">
      <span class="ca__etq">{t.copiarAlbum.destino}</span>
      <div class="ca__destino">
        <TextInput bind:value={destino} placeholder="/ruta/copia.micdb" />
        <Button variante="secundario" onclick={examinar}>
          {t.accion.examinar}
        </Button>
      </div>
    </label>

    <label class="ca__check">
      <input type="checkbox" bind:checked={soloEstructura} />
      <span>{t.copiarAlbum.soloEstructura}</span>
    </label>
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={cerrar}>{t.accion.cancelar}</Button>
    <Button
      variante="primario"
      onclick={copiar}
      disabled={destino.trim() === "" || copiando}
      cargando={copiando}
    >
      {copiando ? t.copiarAlbum.copiando : t.accion.aceptar}
    </Button>
  {/snippet}
</Modal>

<style>
  .ca {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .ca__grupo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .ca__etq {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
  .ca__destino {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--esp-2);
    align-items: center;
  }
  .ca__check {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
  }
</style>
