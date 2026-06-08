<!--
  SortDialog — define hasta 3 niveles de orden (como frmOrdenar). Cada nivel
  elige un campo ordenable y una dirección. Aplica con `setOrden`.
-->
<script lang="ts">
  import { Modal, Button, Select } from "$lib/components/ui";
  import { MAX_ORDEN } from "$lib/stores/albumState.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { Direccion, OrdenCampo } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // Niveles locales editables (campo "" = sin orden en ese nivel).
  type Nivel = { campo: string; direccion: Direccion };
  let niveles = $state<Nivel[]>(inicial());

  function inicial(): Nivel[] {
    const base: Nivel[] = estado.orden.map((o) => ({
      campo: o.campo,
      direccion: o.direccion,
    }));
    while (base.length < MAX_ORDEN) base.push({ campo: "", direccion: "asc" });
    return base.slice(0, MAX_ORDEN);
  }

  // Campos ordenables: solo los de la tabla mostrada y no multidato (ordenar
  // la tabla principal por un campo de variantes rompería la consulta).
  const ordenables = $derived(
    estado.campos
      .filter((c) => c.tabla === estado.tabla && c.tipo !== "multidato")
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  function opcionesCampo() {
    return [
      { valor: "", etiqueta: t.orden.ninguno },
      ...ordenables.map((c) => ({ valor: c.nombre, etiqueta: c.nombre })),
    ];
  }

  const opcionesDir = [
    { valor: "asc" as Direccion, etiqueta: t.orden.ascendente },
    { valor: "desc" as Direccion, etiqueta: t.orden.descendente },
  ];

  function aplicar(): void {
    const orden: OrdenCampo[] = niveles
      .filter((n) => n.campo !== "")
      .map((n) => ({ campo: n.campo, direccion: n.direccion }));
    estado.setOrden(orden);
    cerrar();
  }

  function limpiar(): void {
    estado.setOrden([]);
    cerrar();
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.orden.titulo} ancho="sm" onCerrar={cerrar}>
  <div class="sort">
    {#each niveles as nivel, i (i)}
      <div class="sort__fila">
        <span class="sort__etq">{i === 0 ? t.orden.por : t.orden.luego}</span>
        <div class="sort__campo">
          <Select bind:value={nivel.campo} opciones={opcionesCampo()} />
        </div>
        <div class="sort__dir">
          <Select bind:value={nivel.direccion} opciones={opcionesDir} disabled={nivel.campo === ""} />
        </div>
      </div>
    {/each}
  </div>

  {#snippet pie()}
    <Button variante="fantasma" onclick={limpiar}>{t.accion.limpiar}</Button>
    <Button variante="primario" onclick={aplicar}>{t.accion.aplicar}</Button>
  {/snippet}
</Modal>

<style>
  .sort {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .sort__fila {
    display: grid;
    grid-template-columns: 70px 1fr 140px;
    align-items: center;
    gap: var(--esp-2);
  }
  .sort__etq {
    font-size: var(--tam-fuente-sm);
    color: var(--color-texto-secundario);
  }
</style>
